---
title: "Drift-check baseline refresh stays a Rust-side review gate, not a decoder auto-step"
date: 2026-06-25
category: architecture-patterns
module: emukc (cli)
problem_type: architecture_pattern
component: cli_command
severity: medium
applies_when:
  - "Considering auto-wiring `battle drift-check --accept` into the main-decoder sync flow"
  - "Modifying `main-decoder` --sync-battle-assets / --sync-assets behavior"
  - "Changing the set of assets tracked by .sync-fingerprint.json"
tags: [drift-check, sync-fingerprint, main-decoder, battle-assets, boundary, rejected-approach]
related_components: [main-decoder, emukc_bootstrap]
---

# Drift-check baseline refresh stays a Rust-side review gate, not a decoder auto-step

## Context

`battle drift-check` fingerprints the synced assets under
`crates/emukc_bootstrap/assets/` against the tracked baseline
`crates/emukc_bootstrap/assets/.sync-fingerprint.json`, and `--accept`
(added 2026-06-25, commit `ea575cf`) refreshes that baseline to bless the
current on-disk state. The decode/sync side lives in `main-decoder` (Bun +
TypeScript): `bun run decode -- --sync-battle-assets` copies the 4 battle
assets into the bootstrap assets dir, `--sync-assets` copies 6 more.

A natural-looking follow-up was raised: make the sync flow call `--accept`
automatically so the baseline never goes stale after a sync. **This was
deliberately rejected.** The two-step loop — decode → sync → review
`git diff` → `cargo run -- battle drift-check --accept` → commit assets +
baseline together — is the intended design.

## Guidance

Keep baseline refresh as an explicit, separate Rust-side command. Do NOT wire
`--accept` into any `main-decoder` sync path. When you add or rename a synced
asset, update the asset list in `src/bin/cli/drift_check.rs` and refresh the
baseline with `--accept` in the same commit that lands the asset.

## Why This Matters

Three concrete reasons the auto-wire is wrong, not just unnecessary:

1. **Cross-language coupling.** Sync is Bun/TS; `--accept` is the Rust CLI.
   Auto-wiring forces the decode pipeline to shell out to
   `cargo run -- battle drift-check --accept`, hanging a cargo build/run
   dependency off the pure-TS decode and `bun test` flow. The only alternative
   — reimplementing the canonicalize+hash fingerprint algorithm in TypeScript —
   creates two sources of truth for the fingerprint and is worse.

2. **Scope mismatch.** The baseline tracks 6 assets, but `--sync-battle-assets`
   writes only the 4 battle ones. The other two — `wikiwiki_map_catalog` and
   `public_map_catalog_overlays` — are not produced by `main-decoder` at all
   (`rg` for them in `main-decoder/src/` is empty); they come from the Rust-side
   wikiwiki/overlay capture. Auto-accepting after a battle sync would re-bless
   all 6 — including any uncommitted hand-edits to the two map-catalog assets —
   off a partial 4-asset sync.

3. **`--accept` is the review gate.** Drift-check exists precisely to force a
   human to look at `git diff` after a sync and confirm the decoded change is
   intended before re-baselining. Auto-blessing on every sync deletes the
   checkpoint the tool was built to enforce.

## When to Apply

- Whenever someone proposes "just refresh the baseline automatically after
  sync" — point here.
- When editing `main-decoder/src/pipeline.ts` sync branches or the asset list
  in `src/bin/cli/drift_check.rs`: the baseline is refreshed by a separate
  deliberate `--accept`, never as a side effect of decode/sync.

## Related

- `docs/solutions/architecture-patterns/map-data-authority.md` — the
  wikiwiki/overlay assets that the baseline tracks but the decoder does not
  produce.
- `docs/plans/archive/2026-06-15-002-feat-battle-map-client-sync-loop-plan.md`
  — the client-sync loop plan; `--accept` closed its baseline-refresh
  follow-up, and this is the boundary for the deeper deferred item.
