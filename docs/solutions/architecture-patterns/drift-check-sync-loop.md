---
title: "drift-check: the client-sync loop trigger — fingerprint, diff, three-state report, scaffold"
date: 2026-06-25
category: architecture-patterns
module: emukc
problem_type: architecture_pattern
component: cli_command
severity: medium
applies_when:
  - "Understanding or modifying the battle drift-check command"
  - "Adding or removing a synced asset that drift-check should track"
  - "Investigating why drift-check reports DRIFT, no drift, or baseline recorded"
tags: [drift-check, sync-fingerprint, client-sync-loop, canonicalize, scaffold, ce-plan, cli]
related_components: [emukc_bootstrap, main-decoder]
---

# drift-check: the client-sync loop trigger — fingerprint, diff, three-state report, scaffold

## Context

`cargo run -- battle drift-check` (`src/bin/cli/drift_check.rs`) is the entry
point of the battle/map client-sync loop: it detects that the decoded client
moved and turns that into actionable work. It answers one question — *do the
synced assets in the repo still match a known-good baseline for the decoded
client version?* — and on "no" it can scaffold the plan to fix it. The
`--accept` refresh side and *why it is not auto-wired into the decoder* are
covered separately (see Related); this doc is the mechanism overview.

## Guidance

### The end-to-end flow (`exec`, drift_check.rs:253)

1. `repo_root()` (drift_check.rs:205) derives the root from a synced asset path
   (`<root>/crates/emukc_bootstrap/assets/<file>` → 4 ancestors up), so
   `version.txt`, the manifest, and `docs/plans/` all resolve cwd-independently.
2. `read_version()` reads `main-decoder/out/version.txt` → `(version, missing)`.
   The file is gitignored and only exists after `bun run decode`; **absent is
   not a panic and not drift** — it yields `VERSION_ABSENT` + `missing=true`,
   reported as a prerequisite.
3. `fingerprint()` hashes the 6 tracked assets (`synced_asset_paths()`,
   drift_check.rs:190) into a `SyncFingerprint { version, assets: name→hash }`.
4. `load_manifest()` reads the tracked baseline
   `crates/emukc_bootstrap/assets/.sync-fingerprint.json` (absent → `None`).
5. `diff(previous, current, version_missing)` → `DriftReport`.
6. Branch: `--accept` rewrites the baseline; a first run with no manifest seeds
   the baseline; `--scaffold` writes a ce-plan on drift; a real drift exits
   non-zero (`bail`).

### Canonicalize so formatting churn isn't false drift

`hash_asset` (drift_check.rs:110) runs each asset through `canonicalize_json`
(drift_check.rs:102) before hashing with the repo's `SimpleHash` (Sha256,
bs58 — no new dependency). `serde_json::Map` is `BTreeMap`-backed (this
workspace has no `preserve_order`), so re-serializing sorts object keys and
normalizes whitespace. Result: re-emitting an asset with reordered keys is
**not** drift, but a real same-version content change still is.

### Three-state result, version-missing is orthogonal

`DriftKind` (drift_check.rs:65) is `NoDrift` / `Drift` / `BaselineRecorded`.
Drift is `version_changed || changed || added || removed` assets
(drift_check.rs:167). `version_missing` is a separate flag on the report, not a
drift state — it surfaces "run `bun run decode` first" without failing the
diff. `--accept` refuses while `version_missing` (drift_check.rs:266) so a
`VERSION_ABSENT` baseline can't be recorded.

### Pure core / CLI wiring split

`fingerprint` and `diff` are pure over explicit inputs (a version string + a
`(name, path)` list, two manifests), so the loop logic is unit-tested without a
live tree (`scaffold_on_drift_writes_a_plan`, etc.). The CLI arm just wires the
real repo paths in. Keep new logic in the pure functions, not the `exec` arm.

### `--scaffold` emits a ce-plan; it won't clobber

On drift, `scaffold` (drift_check.rs:354) writes
`docs/plans/<date>-sync-battle-protocol-<version-slug>-plan.md` from the report
(Summary auto-filled, Implementation Units templated for a human). If the slug
already exists it **bails rather than overwrites**. No drift → writes nothing.

### Adding or removing a tracked asset

Edit `synced_asset_paths()` (drift_check.rs:190) — add the `(name, path)` pair
and its `repo_*_path()` helper — then refresh the baseline with
`battle drift-check --accept` in the same commit, so the committed
`.sync-fingerprint.json` matches the committed asset set. The 6 assets are 4
battle assets + 2 map-catalog assets; the latter are not decoder-produced (see
Related).

## Why This Matters

Synced assets and the decoded client version drift apart silently otherwise:
someone re-decodes without re-syncing, or hand-edits a synced asset, and the
mismatch only surfaces as a runtime battle/map bug. drift-check makes the
mismatch a loud, structured, exit-non-zero signal at the source, and
`--scaffold` turns it straight into a plan — closing the loop from "client
moved" to "here's the work."

## When to Apply

- Modifying drift-check: put logic in the pure `fingerprint`/`diff` core.
- Adding/removing a synced asset: update `synced_asset_paths()` and `--accept`
  the baseline in the same commit.
- Reading a DRIFT report: `version: X -> Y` plus changed/added/removed asset
  lists tell you exactly what moved.

## Related

- `docs/solutions/architecture-patterns/drift-check-baseline-refresh-boundary.md`
  — the `--accept` refresh and why it stays a deliberate Rust-side gate, not a
  decoder auto-step.
- `docs/solutions/architecture-patterns/map-data-authority.md` — the 2
  map-catalog assets drift-check tracks but the decoder does not produce.
- CLAUDE.md *Client-Derived Battle Validation* — the decode→sync workflow that
  feeds the assets drift-check fingerprints.
