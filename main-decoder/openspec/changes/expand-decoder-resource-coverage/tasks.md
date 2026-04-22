## 1. Resource Category Extraction (TypeScript)

- [x] 1.1 Create `main-decoder/src/resource-categories.ts` with `extractResourceCategories(moduleGraph)` function that scans all modules for ship/slot `targetType` arguments in `getShip`, `getSlotitem`, `ShipLoader.add`, `SlotLoader.add` calls
- [x] 1.2 Add explicit path scanning for `"kcs2/resources/ship/"` and `"kcs2/resources/slot/"` regex patterns to discover categories not caught by AST extraction
- [x] 1.3 Add SP remodel subcategory discovery by scanning for `"sp_remodel"` path patterns and `SuffixUtils.create` calls with `"ship_sp_remodel/*"` keys
- [x] 1.4 Add Rust-facing generation groups to the extracted category asset so `ship.rs` and `slot.rs` can replace inline target-type arrays without embedding concrete ID sets
- [x] 1.5 Add types for the category asset to `main-decoder/src/types.ts`: `ResourceCategoriesAsset`
- [x] 1.6 Write tests in `main-decoder/test/resource-categories.test.ts` verifying target type discovery, generation-group output, and provenance against the decoded module graph
- [x] 1.7 Wire the category extractor into `main-decoder/src/pipeline.ts`: write `resource_categories.json` to `out/` and sync it to `crates/emukc_bootstrap/assets/` when `--sync-assets` is provided

## 2. Resource ID Set Extraction (TypeScript)

- [ ] 2.1 Create `main-decoder/src/resource-id-sets.ts` with `extractResourceIdSets(moduleGraph)` that records only ship/slot IDs directly observable as literals or inline enumerations in decoded `main.js`
- [ ] 2.2 Extract ship ID subsets for categories whose membership is literally encoded in preload/control-flow modules (`special`, `sp_remodel/*`, `card_round`, `icon_box`, `reward_*`) and record provenance
- [ ] 2.3 Extract slotitem ID subsets only when the ID set is explicitly enumerable in `main.js`; if a category is runtime-driven (for example `btxt_flat` via `slotId` parameters), mark it unresolved instead of synthesizing IDs
- [ ] 2.4 Add `ResourceIdSetsAsset` to `main-decoder/src/types.ts`, including completeness metadata such as `coverageMode` / unresolved keys
- [ ] 2.5 Write tests in `main-decoder/test/resource-id-sets.test.ts` verifying literal-ID extraction, unresolved-category marking, and that Rust baselines are not used as oracle input
- [ ] 2.6 Wire the ID-set extractor into `main-decoder/src/pipeline.ts`: write `resource_id_sets.json` to `out/` and sync it to `crates/emukc_bootstrap/assets/` when `--sync-assets` is provided

## 3. Audio Resource Extraction (TypeScript)

- [ ] 3.1 Create `main-decoder/src/audio-resources.ts` with `extractAudioResources(moduleGraph)` function
- [ ] 3.2 Implement SE ID extraction: scan for `playSE`, `SoundManager`, `"se/"` explicit paths, extract numeric IDs from call arguments and path patterns
- [ ] 3.3 Implement categorized BGM extraction: scan for `playBGM`, `"bgm/"` paths, and BGM manager references, and emit separate `fanfareIds`, `portIds`, and `battleIds`
- [ ] 3.4 Implement voice extraction: scan for `"voice/"` paths, titlecall range patterns (numeric iteration bounds), and tutorial voice file stems
- [ ] 3.5 Add `AudioResourcesAsset` to `main-decoder/src/types.ts`
- [ ] 3.6 Write tests in `main-decoder/test/audio-resources.test.ts`
- [ ] 3.7 Wire audio extractor into the pipeline with sync support

## 4. UI Resource Extraction (TypeScript)

- [ ] 4.1 Create `main-decoder/src/ui-resources.ts` with `extractUiResources(moduleGraph)` function
- [ ] 4.2 Implement map resource extraction that emits explicit `defaultFiles` and `eventFiles` lists instead of only area/map identifiers
- [ ] 4.3 Implement furniture extraction: scan for `"furniture/"` paths and emit nested furniture data used by `furniture.rs`
- [ ] 4.4 Implement use item extraction: scan for `"useitem/"` paths and emit separate `cardIds` and `underlineIds`
- [ ] 4.5 Implement area/world select extraction: scan for `"area/"` and `"worldselect/"` paths and emit nested `area`/`worldSelect` data
- [ ] 4.6 Add `UiResourcesAsset` to `main-decoder/src/types.ts`
- [ ] 4.7 Write tests in `main-decoder/test/ui-resources.test.ts`
- [ ] 4.8 Wire UI extractor into the pipeline with sync support

## 5. CLI & Pipeline Integration

- [ ] 5.1 Add `--sync-assets` flag to `main-decoder/src/cli.ts` as the consolidated asset sync flag
- [ ] 5.2 Keep `--sync-battle-assets` and `--sync-resource-manifest` as backwards-compatible aliases for `--sync-assets`
- [ ] 5.3 Update pipeline summary output to show new extractor stats (category groups, observed ID subset sizes/unresolved keys, audio counts, UI counts)
- [ ] 5.4 Update CLI tests and `main-decoder/README.md` for `--sync-assets` and the four synced JSON assets
- [ ] 5.5 Update manifest-loader/help text in Rust to point users at `--sync-assets`
- [ ] 5.6 Run full pipeline (`bun run decode -- --sync-assets`) and verify all JSON assets are generated correctly

## 6. Rust Asset Consumption — Resource Categories

- [ ] 6.1 Define Rust types for `resource_categories.json` in `crates/emukc_bootstrap/src/`
- [ ] 6.2 Load `resource_categories.json` via `include_str!` and lazy statics in a new `crates/emukc_bootstrap/src/resource_categories.rs` module
- [ ] 6.3 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/ship.rs`: replace hardcoded category arrays in `make_non_graph` with generation groups from JSON, while keeping existing exhaustive ship ID baselines unchanged
- [ ] 6.4 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/slot.rs`: replace hardcoded category arrays in `make_default` with generation groups from JSON, while keeping `BTXT_FLAT_IDS` unchanged
- [ ] 6.5 Write Rust tests verifying loaded category groups are non-empty and preserve current category-array behavior

## 7. Rust Asset Consumption — Audio Resources

- [ ] 7.1 Define Rust types for `audio_resources.json` and load via `include_str!` in a new `crates/emukc_bootstrap/src/audio_resources.rs` module
- [ ] 7.2 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/unversioned.rs`: replace `SE`, tutorial voice stems, and titlecall ranges with JSON-loaded audio data
- [ ] 7.3 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/bgm.rs`: use categorized BGM IDs from JSON for `fanfare`, `port`, and `battle`

## 8. Rust Asset Consumption — UI Resources

- [ ] 8.1 Define Rust types for `ui_resources.json` and load via `include_str!` in a new `crates/emukc_bootstrap/src/ui_resources.rs` module
- [ ] 8.2 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/map.rs`: use explicit default/event map file lists from JSON
- [ ] 8.3 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/furniture.rs`: use nested furniture data from JSON where categories are currently hardcoded
- [ ] 8.4 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/use_item.rs`: use `cardIds` and `underlineIds` from JSON
- [ ] 8.5 Modify `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/unversioned.rs`: use UI JSON data for `area/sally`, `area/airunit`, and `worldselect`
- [ ] 8.6 Add warning-path tests for empty or partial UI/audio/category assets so runtime skips missing data without panicking

## 9. Verification

- [ ] 9.1 Run `cd main-decoder && bun test` — all TS tests pass
- [ ] 9.2 Run `cd main-decoder && bun run decode -- --sync-assets` — all JSON assets generated without errors
- [ ] 9.3 Run `cargo test -p emukc_bootstrap` — all Rust tests pass
- [ ] 9.4 Run `cargo test --test gameplay_tests` — integration tests pass
- [ ] 9.5 Compare generated cache list sizes before/after for migrated ship/slot category groups plus audio/UI resources: verify non-regression (`new >= old`) only for domains that actually switched to JSON-driven generation in this change
- [ ] 9.6 Run `cargo clippy --workspace` — no new warnings
