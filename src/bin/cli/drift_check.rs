//! `battle drift-check` — the loop trigger (plan U3/U4).
//!
//! Detects that the decoded client moved by reading `main-decoder/out/version.txt`
//! and fingerprinting the synced battle/route asset files, comparing against a
//! tracked last-known-good manifest at
//! `crates/emukc_bootstrap/assets/.sync-fingerprint.json`. On drift it emits a
//! structured report and exits non-zero; with `--scaffold` it also writes a
//! ce-plan skeleton populated from the report.
//!
//! The core (fingerprint a set of files + a version string → manifest; diff two
//! manifests → report) is pure over explicit inputs so it is testable without the
//! live tree. The CLI arm wires the real repo paths in.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::Args;
use emukc::bootstrap::prelude::{
    repo_battle_module_index_path, repo_battle_protocol_fields_path,
    repo_battle_resource_rules_path, repo_battle_slot_resource_triggers_path,
    repo_public_map_catalog_overlay_path, repo_wikiwiki_map_catalog_path,
};
use emukc::crypto::SimpleHash;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Sentinel recorded in the manifest when `version.txt` is absent (the decoder
/// has not run yet). Distinguishes "no version known" from any real version
/// string so a later real version is reported as drift.
const VERSION_ABSENT: &str = "<absent>";

#[derive(Debug, Args)]
pub(super) struct DriftCheckArgs {
    #[arg(help = "When drift is found, write a ce-plan scaffold under docs/plans/")]
    #[arg(long)]
    scaffold: bool,

    #[arg(help = "Accept the current decoded state as the new known-good baseline \
                (refresh the fingerprint manifest), whether or not it drifted")]
    #[arg(long)]
    accept: bool,

    #[arg(help = "Print the structured drift report as JSON instead of a human report")]
    #[arg(long)]
    json: bool,
}

/// The tracked last-known-good fingerprint manifest. Serialized to
/// `crates/emukc_bootstrap/assets/.sync-fingerprint.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SyncFingerprint {
    /// `version.txt` content, or [`VERSION_ABSENT`] if the decoder has not run.
    pub version: String,
    /// Asset logical name → canonical-content hash.
    pub assets: BTreeMap<String, String>,
}

/// What `drift-check` concluded about the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum DriftKind {
    /// Manifest matched the current state.
    NoDrift,
    /// Version and/or asset hashes moved.
    Drift,
    /// No manifest existed; current state was taken as the baseline.
    BaselineRecorded,
}

/// The structured diff between a persisted manifest and the current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DriftReport {
    pub kind: DriftKind,
    /// Set only when the recorded version differs from the current one.
    pub version_old: Option<String>,
    pub version_new: String,
    /// `version.txt` was missing (run `bun run decode` first). Not drift on its
    /// own — reported as a prerequisite.
    pub version_missing: bool,
    /// Asset names whose hash changed.
    pub changed_assets: Vec<String>,
    /// Asset names present now but not in the manifest.
    pub added_assets: Vec<String>,
    /// Asset names in the manifest but not present now.
    pub removed_assets: Vec<String>,
}

impl DriftReport {
    fn is_drift(&self) -> bool {
        self.kind == DriftKind::Drift
    }
}

/// Canonicalize JSON bytes so decoder-output formatting churn (key order,
/// whitespace) is not a false positive, while same-version *content* changes
/// still differ. `serde_json::Map` is `BTreeMap`-backed (no `preserve_order`
/// feature in this workspace), so re-serializing sorts object keys.
fn canonicalize_json(raw: &[u8]) -> Result<Vec<u8>> {
    let value: Value =
        serde_json::from_slice(raw).context("asset is not valid JSON; cannot canonicalize")?;
    Ok(serde_json::to_vec(&value)?)
}

/// Hash one asset file's canonical content. Sha256 via the repo's [`SimpleHash`]
/// (bs58-encoded), already used across the binary — no new dependency.
fn hash_asset(path: &Path) -> Result<String> {
    let raw = fs::read(path)
        .with_context(|| format!("failed to read asset for fingerprint: {}", path.display()))?;
    let canonical = canonicalize_json(&raw)
        .with_context(|| format!("failed to canonicalize asset: {}", path.display()))?;
    Ok(canonical.simple_hash())
}

/// Pure core: fingerprint a `version` string + a set of `(name, path)` assets
/// into a manifest. Errors if any asset is missing or unparseable.
fn fingerprint(version: &str, assets: &[(String, PathBuf)]) -> Result<SyncFingerprint> {
    let mut map = BTreeMap::new();
    for (name, path) in assets {
        map.insert(name.clone(), hash_asset(path)?);
    }
    Ok(SyncFingerprint {
        version: version.to_string(),
        assets: map,
    })
}

/// Pure core: diff a persisted manifest against the current fingerprint.
/// `version_missing` flags that `version.txt` was absent at read time.
fn diff(
    previous: Option<&SyncFingerprint>,
    current: &SyncFingerprint,
    version_missing: bool,
) -> DriftReport {
    let Some(previous) = previous else {
        return DriftReport {
            kind: DriftKind::BaselineRecorded,
            version_old: None,
            version_new: current.version.clone(),
            version_missing,
            changed_assets: Vec::new(),
            added_assets: Vec::new(),
            removed_assets: Vec::new(),
        };
    };

    let mut changed_assets = Vec::new();
    let mut added_assets = Vec::new();
    for (name, hash) in &current.assets {
        match previous.assets.get(name) {
            Some(old) if old != hash => changed_assets.push(name.clone()),
            Some(_) => {}
            None => added_assets.push(name.clone()),
        }
    }
    let removed_assets: Vec<String> = previous
        .assets
        .keys()
        .filter(|name| !current.assets.contains_key(*name))
        .cloned()
        .collect();

    let version_changed = previous.version != current.version;
    let any_drift = version_changed
        || !changed_assets.is_empty()
        || !added_assets.is_empty()
        || !removed_assets.is_empty();

    DriftReport {
        kind: if any_drift {
            DriftKind::Drift
        } else {
            DriftKind::NoDrift
        },
        version_old: version_changed.then(|| previous.version.clone()),
        version_new: current.version.clone(),
        version_missing,
        changed_assets,
        added_assets,
        removed_assets,
    }
}

/// The synced battle/route assets fingerprinted by `drift-check` (KTD3), keyed
/// by logical name. Resolved via `emukc_bootstrap`'s `CARGO_MANIFEST_DIR`-based
/// helpers so the same files resolve identically from any cwd.
fn synced_asset_paths() -> Vec<(String, PathBuf)> {
    vec![
        ("battle_protocol_fields".to_string(), repo_battle_protocol_fields_path()),
        ("battle_resource_rules".to_string(), repo_battle_resource_rules_path()),
        ("battle_module_index".to_string(), repo_battle_module_index_path()),
        ("battle_slot_resource_triggers".to_string(), repo_battle_slot_resource_triggers_path()),
        ("wikiwiki_map_catalog".to_string(), repo_wikiwiki_map_catalog_path()),
        ("public_map_catalog_overlays".to_string(), repo_public_map_catalog_overlay_path()),
    ]
}

/// Repo root, derived from a synced asset path
/// (`<root>/crates/emukc_bootstrap/assets/<file>` → four ancestors up). Keeps
/// `version.txt`, the manifest, and `docs/plans/` resolution cwd-independent and
/// consistent with the asset helpers.
fn repo_root() -> Result<PathBuf> {
    let asset = repo_battle_protocol_fields_path();
    asset
        .ancestors()
        .nth(4)
        .map(Path::to_path_buf)
        .with_context(|| format!("cannot derive repo root from asset path {}", asset.display()))
}

fn version_txt_path(root: &Path) -> PathBuf {
    root.join("main-decoder/out/version.txt")
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join("crates/emukc_bootstrap/assets/.sync-fingerprint.json")
}

/// Read `version.txt`; returns `(version, missing)`. Absent (gitignored, only
/// exists after `bun run decode`) is NOT a panic — it yields [`VERSION_ABSENT`]
/// and `missing=true` so the report names the prerequisite.
fn read_version(path: &Path) -> Result<(String, bool)> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok((raw.trim().to_string(), false)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok((VERSION_ABSENT.to_string(), true))
        }
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn load_manifest(path: &Path) -> Result<Option<SyncFingerprint>> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let manifest = serde_json::from_str(&raw)
                .with_context(|| format!("failed to parse manifest {}", path.display()))?;
            Ok(Some(manifest))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn write_manifest(path: &Path, manifest: &SyncFingerprint) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write manifest {}", path.display()))
}

pub(super) fn exec(args: &DriftCheckArgs) -> Result<()> {
    let root = repo_root()?;
    let (version, version_missing) = read_version(&version_txt_path(&root))?;
    let current = fingerprint(&version, &synced_asset_paths())?;
    let manifest_path = manifest_path(&root);
    let previous = load_manifest(&manifest_path)?;
    let report = diff(previous.as_ref(), &current, version_missing);

    // `--accept` closes the refresh loop: record the current state as the new
    // known-good baseline (whether or not it drifted), so a reviewed drift can be
    // accepted without hand-deleting the manifest. Refuse while version.txt is
    // absent — accepting `VERSION_ABSENT` would poison the baseline.
    if args.accept {
        if version_missing {
            bail!(
                "cannot accept a baseline while main-decoder/out/version.txt is absent; \
                 run `bun run decode` first"
            );
        }
        write_manifest(&manifest_path, &current)?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_report(&report, &manifest_path);
            println!("baseline accepted: {}", manifest_path.display());
        }
        return Ok(());
    }

    // Seed the baseline on first run so subsequent runs can detect real drift.
    if report.kind == DriftKind::BaselineRecorded {
        write_manifest(&manifest_path, &current)?;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report, &manifest_path);
    }

    if args.scaffold {
        match scaffold(&root, &report)? {
            Some(plan_path) => println!("scaffolded ce-plan: {}", plan_path.display()),
            None => println!("nothing to scaffold (no drift)"),
        }
    }

    if report.is_drift() {
        bail!("client drift detected; see report above");
    }

    Ok(())
}

fn print_report(report: &DriftReport, manifest_path: &Path) {
    println!("battle drift-check");
    if report.version_missing {
        println!(
            "prerequisite: main-decoder/out/version.txt is absent; run `bun run decode` first"
        );
    }
    match report.kind {
        DriftKind::NoDrift => {
            println!("result: no drift");
            println!("version: {}", report.version_new);
        }
        DriftKind::BaselineRecorded => {
            println!("result: baseline recorded ({})", manifest_path.display());
            println!("version: {}", report.version_new);
        }
        DriftKind::Drift => {
            println!("result: DRIFT");
            match &report.version_old {
                Some(old) => println!("version: {old} -> {}", report.version_new),
                None => println!("version: {} (unchanged)", report.version_new),
            }
        }
    }
    if !report.changed_assets.is_empty() {
        println!("changed assets:");
        for name in &report.changed_assets {
            println!("- {name}");
        }
    }
    if !report.added_assets.is_empty() {
        println!("added assets:");
        for name in &report.added_assets {
            println!("- {name}");
        }
    }
    if !report.removed_assets.is_empty() {
        println!("removed assets:");
        for name in &report.removed_assets {
            println!("- {name}");
        }
    }
}

/// U4: write a ce-plan scaffold under `docs/plans/` when drift is found.
/// Returns the written path, or `None` if there is no drift (nothing to
/// scaffold). Errors on a slug collision rather than clobbering.
fn scaffold(root: &Path, report: &DriftReport) -> Result<Option<PathBuf>> {
    if !report.is_drift() {
        return Ok(None);
    }
    let date = emukc::time::chrono::Local::now().format("%Y-%m-%d").to_string();
    let version_slug = slugify(&report.version_new);
    let plans_dir = root.join("docs/plans");
    let filename = format!("{date}-sync-battle-protocol-{version_slug}-plan.md");
    let target = plans_dir.join(&filename);
    if target.exists() {
        bail!("scaffold target already exists, refusing to clobber: {}", target.display());
    }
    let body = render_scaffold(&date, report);
    fs::create_dir_all(&plans_dir)
        .with_context(|| format!("failed to create {}", plans_dir.display()))?;
    fs::write(&target, body).with_context(|| format!("failed to write {}", target.display()))?;
    Ok(Some(target))
}

/// Lowercase, keep alnum, collapse the rest to `-`. Keeps the version readable
/// in the filename (`6.3.0.0` → `6-3-0-0`).
fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Render the ce-plan skeleton, mirroring the layout of an existing plan
/// (Summary / Problem Frame / Requirements / Key Technical Decisions /
/// Implementation Units). The Summary is auto-populated from the drift report;
/// Implementation Units is a templated starting list for a human to refine.
fn render_scaffold(date: &str, report: &DriftReport) -> String {
    let version_line = match &report.version_old {
        Some(old) => format!("`{old}` → `{}`", report.version_new),
        None => format!("`{}` (unchanged; content drift)", report.version_new),
    };
    let changed = list_or_none(&report.changed_assets);
    let added = list_or_none(&report.added_assets);
    let removed = list_or_none(&report.removed_assets);

    format!(
        r#"---
title: "feat: Sync battle protocol to client {version}"
type: feat
date: {date}
status: draft
origin: generated by `cargo run -- battle drift-check --scaffold`
related:
  - docs/plans/2026-06-15-002-feat-battle-map-client-sync-loop-plan.md
---

# feat: Sync battle protocol to client {version}

## Summary

A `drift-check` run detected that the decoded client moved. This plan is a
generated **starting point** for the sync round — review and refine it before
implementing.

- **Version:** {version_line}
- **Changed assets:** {changed}
- **Added assets:** {added}
- **Removed assets:** {removed}

## Problem Frame

The synced battle/route assets under `crates/emukc_bootstrap/assets/` no longer
match the last-known-good fingerprint (`.sync-fingerprint.json`). EmuKC's
simulation/validation logic is derived from these assets, so a drift may mean
the validator field tables, resource rules, or routing topology are now stale
relative to the client.

## Requirements

- R1. Re-run `cd main-decoder && bun run decode -- --sync-battle-assets` so the
  tracked assets reflect the current client.
- R2. Re-validate that the existing battle gate (`sim_validation_gate`) and
  battle-rules tests still pass against the new assets.
- R3. Refresh the tracked fingerprint manifest so `drift-check` returns to a
  no-drift state once the round is complete.

## Key Technical Decisions

- KTD1. _Fill in: did the protocol field set change shape, or only values?_
- KTD2. _Fill in: do any validator field tables in `battle_rules.rs` need
  updating to match the new `battle_protocol_fields.json`?_

## Implementation Units

### U1. Re-sync the decoded assets

- **Goal:** Bring the tracked assets up to date with the current client.
- **Files:** `crates/emukc_bootstrap/assets/*.json` (regenerated, not hand-edited).
- **Approach:** `cd main-decoder && bun run decode -- --sync-battle-assets`.

### U2. Reconcile validator field tables{table_unit}

- **Goal:** Keep `battle_rules.rs` field tables consistent with the new protocol.
- **Files:** `crates/emukc_bootstrap/src/battle_rules.rs`.
- **Approach:** Diff the changed asset(s) and update the day/night field tables.

### U3. Re-validate and re-freeze

- **Goal:** Confirm the sim still conforms and the golden transcript is current.
- **Files:** `crates/emukc_gameplay/tests/`, `tests/gameplay_tests/battle_golden.rs`.
- **Approach:** Run the gate + battle-rules tests; re-freeze goldens only if a
  legitimate logic change shifts the output, explaining the diff.

### U4. Refresh the fingerprint manifest

- **Goal:** Return `drift-check` to a clean no-drift state.
- **Files:** `crates/emukc_bootstrap/assets/.sync-fingerprint.json`.
- **Approach:** Once the round is reconciled, run
  `cargo run -- battle drift-check --accept` to record the current state as the new
  known-good baseline, then commit the refreshed `.sync-fingerprint.json`.
"#,
        version = report.version_new,
        date = date,
        version_line = version_line,
        changed = changed,
        added = added,
        removed = removed,
        table_unit = if report.changed_assets.iter().any(|a| a == "battle_protocol_fields") {
            " (protocol fields changed)"
        } else {
            ""
        },
    )
}

fn list_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.iter().map(|i| format!("`{i}`")).collect::<Vec<_>>().join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a temp asset file with the given JSON content, return its path.
    fn write_asset(dir: &Path, name: &str, json: &str) -> PathBuf {
        let path = dir.join(format!("{name}.json"));
        fs::write(&path, json).unwrap();
        path
    }

    fn assets(dir: &Path, specs: &[(&str, &str)]) -> Vec<(String, PathBuf)> {
        specs.iter().map(|(name, json)| (name.to_string(), write_asset(dir, name, json))).collect()
    }

    #[test]
    fn canonicalize_ignores_key_order_and_whitespace() {
        let a = br#"{"b":1,"a":2}"#;
        let b = b"{\n  \"a\": 2,\n  \"b\": 1\n}";
        assert_eq!(canonicalize_json(a).unwrap(), canonicalize_json(b).unwrap());
    }

    #[test]
    fn canonicalize_detects_value_change() {
        let a = br#"{"a":1}"#;
        let b = br#"{"a":2}"#;
        assert_ne!(canonicalize_json(a).unwrap(), canonicalize_json(b).unwrap());
    }

    #[test]
    fn no_drift_when_manifest_matches() {
        let dir = tempfile::tempdir().unwrap();
        let assets = assets(dir.path(), &[("battle_protocol_fields", r#"{"x":1}"#)]);
        let baseline = fingerprint("6.3.0.0", &assets).unwrap();
        let report = diff(Some(&baseline), &baseline, false);
        assert_eq!(report.kind, DriftKind::NoDrift);
        assert!(!report.is_drift());
    }

    #[test]
    fn accepted_baseline_clears_subsequent_drift() {
        // `--accept` writes the current fingerprint as the new manifest; a follow-up
        // diff against that same state then reports no drift — the refresh loop
        // closes instead of re-reporting the accepted drift forever.
        let asset_dir = tempfile::tempdir().unwrap();
        let current = fingerprint(
            "6.3.0.1",
            &assets(asset_dir.path(), &[("battle_protocol_fields", r#"{"x":2}"#)]),
        )
        .unwrap();

        let manifest_dir = tempfile::tempdir().unwrap();
        let manifest = manifest_dir.path().join(".sync-fingerprint.json");
        write_manifest(&manifest, &current).unwrap();

        let reloaded = load_manifest(&manifest).unwrap();
        let report = diff(reloaded.as_ref(), &current, false);
        assert_eq!(report.kind, DriftKind::NoDrift, "accepted baseline must clear drift");
        assert!(!report.is_drift());
    }

    #[test]
    fn formatting_churn_is_not_drift() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_assets = assets(dir.path(), &[("a", r#"{"b":1,"a":2}"#)]);
        let baseline = fingerprint("6.3.0.0", &baseline_assets).unwrap();

        // Re-write the same asset reformatted (sorted keys, whitespace).
        let dir2 = tempfile::tempdir().unwrap();
        let reformatted = assets(dir2.path(), &[("a", "{\n  \"a\": 2,\n  \"b\": 1\n}")]);
        let current = fingerprint("6.3.0.0", &reformatted).unwrap();

        let report = diff(Some(&baseline), &current, false);
        assert_eq!(report.kind, DriftKind::NoDrift, "formatting churn must not trip drift");
    }

    #[test]
    fn version_drift_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let a = assets(dir.path(), &[("a", r#"{"x":1}"#)]);
        let baseline = fingerprint("6.2.9.1", &a).unwrap();
        let current = fingerprint("6.3.0.0", &a).unwrap();
        let report = diff(Some(&baseline), &current, false);
        assert_eq!(report.kind, DriftKind::Drift);
        assert_eq!(report.version_old.as_deref(), Some("6.2.9.1"));
        assert_eq!(report.version_new, "6.3.0.0");
        assert!(report.changed_assets.is_empty());
    }

    #[test]
    fn content_drift_same_version_is_reported() {
        let baseline_dir = tempfile::tempdir().unwrap();
        let baseline = fingerprint(
            "6.3.0.0",
            &assets(baseline_dir.path(), &[("battle_protocol_fields", r#"{"x":1}"#)]),
        )
        .unwrap();

        let current_dir = tempfile::tempdir().unwrap();
        let current = fingerprint(
            "6.3.0.0",
            &assets(current_dir.path(), &[("battle_protocol_fields", r#"{"x":2}"#)]),
        )
        .unwrap();

        let report = diff(Some(&baseline), &current, false);
        assert_eq!(report.kind, DriftKind::Drift);
        assert!(report.version_old.is_none(), "same version must not report a version change");
        assert_eq!(report.changed_assets, vec!["battle_protocol_fields".to_string()]);
    }

    #[test]
    fn first_run_records_baseline_not_drift() {
        let dir = tempfile::tempdir().unwrap();
        let current = fingerprint("6.3.0.0", &assets(dir.path(), &[("a", r#"{"x":1}"#)])).unwrap();
        let report = diff(None, &current, false);
        assert_eq!(report.kind, DriftKind::BaselineRecorded);
        assert!(!report.is_drift());
    }

    #[test]
    fn missing_version_is_flagged_not_drift() {
        let dir = tempfile::tempdir().unwrap();
        let a = assets(dir.path(), &[("a", r#"{"x":1}"#)]);
        // version.txt absent → VERSION_ABSENT sentinel, version_missing=true.
        let baseline = fingerprint(VERSION_ABSENT, &a).unwrap();
        let current = fingerprint(VERSION_ABSENT, &a).unwrap();
        let report = diff(Some(&baseline), &current, true);
        assert_eq!(report.kind, DriftKind::NoDrift);
        assert!(report.version_missing);
    }

    #[test]
    fn read_version_absent_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let (version, missing) = read_version(&dir.path().join("nope.txt")).unwrap();
        assert!(missing);
        assert_eq!(version, VERSION_ABSENT);
    }

    #[test]
    fn manifest_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".sync-fingerprint.json");
        let manifest = fingerprint("6.3.0.0", &assets(dir.path(), &[("a", r#"{"x":1}"#)])).unwrap();
        write_manifest(&path, &manifest).unwrap();
        let loaded = load_manifest(&path).unwrap().unwrap();
        assert_eq!(loaded, manifest);
    }

    #[test]
    fn load_manifest_missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_manifest(&dir.path().join("absent.json")).unwrap().is_none());
    }

    #[test]
    fn scaffold_on_drift_writes_a_plan() {
        let root = tempfile::tempdir().unwrap();
        let report = DriftReport {
            kind: DriftKind::Drift,
            version_old: Some("6.2.9.1".to_string()),
            version_new: "6.3.0.0".to_string(),
            version_missing: false,
            changed_assets: vec!["battle_protocol_fields".to_string()],
            added_assets: Vec::new(),
            removed_assets: Vec::new(),
        };
        let written = scaffold(root.path(), &report).unwrap().expect("drift should scaffold");
        assert!(written.exists());
        assert!(written.file_name().unwrap().to_str().unwrap().contains("6-3-0-0"));
        let body = fs::read_to_string(&written).unwrap();
        assert!(body.contains("## Summary"), "scaffold must have a Summary section");
        assert!(body.contains("## Implementation Units"), "scaffold must list units");
        assert!(body.contains("`6.2.9.1` → `6.3.0.0`"), "summary must carry the version delta");
        assert!(
            body.contains("(protocol fields changed)"),
            "protocol-field drift should annotate the reconcile unit"
        );
    }

    #[test]
    fn scaffold_with_no_drift_writes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let report = DriftReport {
            kind: DriftKind::NoDrift,
            version_old: None,
            version_new: "6.3.0.0".to_string(),
            version_missing: false,
            changed_assets: Vec::new(),
            added_assets: Vec::new(),
            removed_assets: Vec::new(),
        };
        assert!(scaffold(root.path(), &report).unwrap().is_none());
        // docs/plans should not even be created for a no-drift scaffold.
        assert!(!root.path().join("docs/plans").exists());
    }

    #[test]
    fn scaffold_does_not_clobber_existing_plan() {
        let root = tempfile::tempdir().unwrap();
        let report = DriftReport {
            kind: DriftKind::Drift,
            version_old: Some("6.2.9.1".to_string()),
            version_new: "6.3.0.0".to_string(),
            version_missing: false,
            changed_assets: Vec::new(),
            added_assets: Vec::new(),
            removed_assets: Vec::new(),
        };
        let first = scaffold(root.path(), &report).unwrap().unwrap();
        // A second scaffold for the same date+version collides; must refuse.
        let err = scaffold(root.path(), &report).unwrap_err();
        assert!(
            err.to_string().contains("already exists"),
            "collision must be reported, got: {err}"
        );
        assert!(first.exists(), "the original plan must be untouched");
    }
}
