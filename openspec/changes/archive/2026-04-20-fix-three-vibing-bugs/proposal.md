## Why

Three regressions from the recent vibing sprint (7e0d397–e866d02) break core gameplay: missing damaged ship art in the cache manifest, wrong sortie map cell state, and stale target selection in multi-slot airstrikes.

## What Changes

- **Manifest generator**: Fix the `damagedSource == "true"` branch so standard-category assets (banner, character_full, character_up, card, etc.) still generate their base path. Currently `gen_base` is `false` and `gen_variants` is gated on `damaged.is_none()`, producing zero paths for the `"true"` case.
- **Sortie cell init**: Restore the fix from 7e0d397 — all cells should start unpassed (`passed: false`), not `cell.cell_no != 0`.
- **Airstrike target refresh**: Re-snapshot alive targets before each slot's attack instead of once at phase start, so later slots skip sunk defenders.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

_None (no existing specs in openspec/specs/)._

## Impact

- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` — ship path generation logic
- `crates/emukc_gameplay/src/game/sortie.rs` — sortie cell initialization
- `crates/emukc_gameplay/src/game/battle/core.rs` — airstrike phase target selection
- `crates/emukc_gameplay/tests/sortie_battle.rs` — test assertion for cell passed state
- `crates/emukc_bootstrap/assets/resource_manifest.json` — regenerated manifest after fix

## Non-goals

- No changes to non-standard ship categories or equipment path generation.
- No refactor of the airstrike phase beyond fixing the stale-target bug.
- No new gameplay features or API changes.
