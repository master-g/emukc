## Why

Battle phase responses report raw (pre-protection) damage instead of effective (post-sinking-protection) damage. This causes the client to animate ships being sunk when sinking protection actually kept them alive, resulting in visual glitches: ships appear sunk, then continue fighting in later phases.

Separately, the codex map catalog (`map_catalog.json`) has systematic data corruption — nearly all maps have incorrect `boss_cell_no`, and hundreds of cells have wrong `color_no`/`event_id`/`event_kind` values. This causes fleets to skip battle nodes (wrongly marked as safe), bosses to appear at wrong positions, and clientside rendering failures.

Both issues were discovered during manual gameplay testing.

## What Changes

- Replace `raw_dealt` with `dealt` in all 11 battle phase damage output locations in `core.rs`, so the client receives damage values that match actual HP changes after sinking protection
- Fix codex `map_catalog.json` data for all 33 maps with known real KC API data: correct `boss_cell_no`, `color_no`, `event_id`, `event_kind` per cell
- Fix `build_sortie_cell_data` to initialize `passed: false` for all cells at sortie start (currently incorrectly sets `passed: cell.cell_no > 0`)

## Capabilities

### New Capabilities

_None_

### Modified Capabilities

- `battle-damage-foundation`: Fix damage reporting to use effective (post-protection) values instead of raw values in all battle phase output arrays
- `sortie`: Fix map cell data initialization (`passed` flag) and correct codex map catalog data (boss positions, cell event types/colors)

## Impact

- **Code**: `crates/emukc_gameplay/src/game/battle/core.rs` (11 edits), `crates/emukc_gameplay/src/game/sortie.rs` (1 edit)
- **Data**: `.data/codex/map_catalog.json` (systematic correction for 30+ maps)
- **API behavior**: Battle response phase damage arrays will contain lower values when sinking protection triggers (correct behavior); map start/next responses will return correct cell metadata
- **Tests**: Existing tests unaffected (enemy ships don't trigger sinking protection, so `raw_dealt == dealt` for enemies). New tests should verify protection case.

## Non-goals

- Combined fleet battle support (separate concern)
- Night battle per-phase HP tracking (not needed — night battle already uses separate response)
- Map routing rule corrections (routing rules appear correct; only cell metadata is wrong)
- Adding new gameplay features
