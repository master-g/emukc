## Why

Rust-side bootstrap/cache has 15+ hardcoded resource ID lists (400+ IDs total) for ships, slots, SE, BGM, maps, voices, etc. These lists are fragile — break when game updates add new IDs — and require manual maintenance. main-decoder already scans all 2,152 modules but only extracts battle-related resources (196 modules). Expanding extraction to cover more resource domains reduces hardcoded maintenance, but the current work direction is explicitly `main.js`-first: only data that is directly observable in decoded `main.js` should become extractor scope. Anything that still depends on CDN probing, runtime API state, or Rust-side curated baselines stays out of scope for this change.

## What Changes

- **New TypeScript extractors** in main-decoder that discover resource patterns from ALL modules (not just battle):
  - Ship/slot target type catalogs and Rust-facing generation groups
  - Best-effort ship/slot ID subsets that are directly observable in `main.js`, plus explicit unresolved markers for categories whose membership is not encoded there
  - Audio resources (SE IDs, categorized BGM IDs, voice ranges/file stems)
  - UI resources (map file lists, furniture categories, use item ids, area images, world select files)
- **Four new JSON assets** synced to `crates/emukc_bootstrap/assets/` alongside existing battle assets:
  - `resource_categories.json`
  - `resource_id_sets.json`
  - `audio_resources.json`
  - `ui_resources.json`
- **Rust-side consumption** of new assets in `make_list` where decoded coverage is strong enough to replace hardcoded logic. In this round, `resource_categories.json` is a migration target; `resource_id_sets.json` is advisory-only and does not replace exhaustive Rust ID baselines.
- Pipeline flag consolidation: `--sync-battle-assets` → `--sync-assets` (covers all asset types)

## Capabilities

### New Capabilities

- `resource-category-extraction`: Extract complete ship/slot target type catalogs and Rust-facing generation groups from main.js modules
- `resource-id-set-extraction`: Extract directly observable ship/slot ID subsets from main.js modules and surface unresolved categories instead of synthesizing completeness
- `audio-resource-extraction`: Extract SE, categorized BGM, and voice resource ids/ranges/file stems from main.js modules
- `ui-resource-extraction`: Extract map, furniture, use item, area, and world select resource patterns from main.js modules
- `decoded-asset-consumption`: Rust-side loading and consumption of decoded JSON assets in make_list where decoded coverage is proven sufficient

### Modified Capabilities

## Impact

- **main-decoder/src/**: 5-6 new TypeScript extraction modules, expanded pipeline, new test files
- **main-decoder/src/cli.ts**: New/consolidated CLI flags for asset syncing
- **crates/emukc_bootstrap/assets/**: 4 new JSON asset files
- **crates/emukc_bootstrap/src/make_list/**: Modified source files to load assets instead of hardcoding where coverage is demonstrably complete
- **crates/emukc_bootstrap/src/make_list/source/kcs2/resources/**: ship.rs and slot.rs lose inline category arrays first; exhaustive ship/slot ID baselines such as `SPECIAL_SHIPS` and `BTXT_FLAT_IDS` remain until a later change proves pure-`main.js` coverage is sufficient
- No API changes, no breaking changes to existing behavior
