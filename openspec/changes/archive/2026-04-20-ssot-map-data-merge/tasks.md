## 1. Field-Authority Merge (emukc_model)

- [x] 1.1 Change `merge_cells()` in `crates/emukc_model/src/codex/map/merge.rs`: for `color_no`, `event_id`, `event_kind`, change from fill-missing (`cell.x <= 0 && other.x > 0`) to last-non-zero-wins (`if other.x > 0 { cell.x = other.x }`). Keep fill-missing for `next_cells`, `node_label`, `master_cell_id`, `distance`.
- [x] 1.2 Change `merge_variant_definition()`: for `boss_cell_no`, change to last-non-zero-wins (`if other.boss_cell_no > 0 { definition.boss_cell_no = other.boss_cell_no }`).
- [x] 1.3 Update existing test `merge_variant_definition_remaps_secondary_cells_by_node_label` to verify last-non-zero-wins behavior: secondary source's non-zero `color_no`/`event_id`/`event_kind` overwrite primary's values.

## 2. Overlay Capture Completion (emukc_bootstrap)

- [x] 2.1 Add `boss_cell_no: i64` field to `CapturedMapStart` in `crates/emukc_bootstrap/src/map_overlay/capture.rs`
- [x] 2.2 Extract `api_bosscell_no` from API data in `extract_map_start_capture_from_api_data` (line ~144)
- [x] 2.3 Set `boss_cell_no: 0` in `extract_map_start_capture_from_response_saver` (line ~104, response-saver format lacks this field)
- [x] 2.4 Set `stage.boss_cell_no` from capture in `merge_capture_into_overlay` (line ~110 in merge.rs) when `boss_cell_no > 0`

## 3. Color-to-Event Inference in Overlay Build (emukc_bootstrap)

- [x] 3.1 Add `fn infer_event_from_color(color_no: i64) -> (i64, i64)` to `crates/emukc_bootstrap/src/map_overlay/merge.rs`: `0→(0,0), 2→(2,0), 3→(3,0), 4→(4,1), 5→(5,1), n≥6→(n,1)`
- [x] 3.2 Apply inference in `merge_capture_into_overlay`: when inserting new cells (None branch, line ~175) and when updating existing cells' `color_no` (Some branch, line ~170), set `event_id`/`event_kind` from inferred values

## 4. Regenerate Overlay Static Asset

- [x] 4.1 Run `cargo run -- wikiwiki-map build-overlays` to regenerate `crates/emukc_bootstrap/assets/public_map_catalog_overlays.json` with boss_cell_no and inferred event_id/event_kind
- [x] 4.2 Verify regenerated asset: spot-check map 13 has `boss_cell_no > 0`, cells have non-zero event_id/event_kind where color_no > 0
- [ ] 4.3 Commit regenerated overlay asset

## 5. Remove kc_data Map Source (emukc_bootstrap)

- [x] 5.1 Remove `kcdata_map_count` and `kcdata_catalog` from `ResolvedMapSources` in `crates/emukc_bootstrap/src/map_pipeline/sources.rs`
- [x] 5.2 Remove `kcdata_map_count` from `MapCatalogBuildReport` in `crates/emukc_bootstrap/src/map_pipeline/report.rs`
- [x] 5.3 Remove kcdata merge step from `assemble_final_map_catalog` in `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- [x] 5.4 Remove `kcdata` module from `crates/emukc_bootstrap/src/map_pipeline/mod.rs` (keep file for reference but remove from build), or delete `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` entirely
- [x] 5.5 Update any tests referencing kcdata map counts or kcdata catalog fields

## 6. stat.json Source Plumbing (emukc_bootstrap)

- [x] 6.1 Add `stat_catalog: Option<MapCatalog>` and `stat_map_count: usize` to `ResolvedMapSources` in `crates/emukc_bootstrap/src/map_pipeline/sources.rs`
- [x] 6.2 Add `stat_map_count: usize` and `stat_source: MapCatalogStatSource` enum (`Downloaded`/`Cached`/`Unavailable`) to `MapCatalogBuildReport` in `crates/emukc_bootstrap/src/map_pipeline/report.rs`
- [x] 6.3 Add stat.json download/cache logic: download from GitHub URL to `.data/stat.json`, use cached file if download fails, return None if both fail
- [x] 6.4 Add stat.json parsing: deserialize into flat `MapCatalog` where each cell has `node_label = Some(<letter>)` and `event_id`/`event_kind` from stat data
- [x] 6.5 Update `load_explicit_source_set` and `load_repo_source_set` to load stat catalog
- [x] 6.6 Update `assemble_final_map_catalog` in `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`: assembly order becomes wikiwiki → overlay → stat (stat merges last, highest authority)

## 7. Verification

- [x] 7.1 Run `cargo test -p emukc_model` — merge tests pass with last-non-zero-wins semantics
- [x] 7.2 Run `cargo test -p emukc_bootstrap` — overlay build tests pass with new boss_cell_no and event inference
- [x] 7.3 Run `cargo test -p emukc_bootstrap` — stat.json parsing tests: unique label match, duplicate label skip, missing label skip
- [x] 7.4 Run `cargo run -- bootstrap` and verify `.data/codex/map_catalog.json`: map 13 has correct boss_cell_no (overlay), event_id/event_kind from stat.json, next_cells from wikiwiki preserved
- [ ] 7.5 Test stat fallback: temporarily make stat.json unavailable, verify bootstrap completes with overlay color inference as fallback, build report shows `stat_source: Unavailable`
- [ ] 7.6 Start server, sortie 1-3: nodes show correct colors, fleet follows graph edges, boss at correct cell
- [ ] 7.7 Verify kc_data removal: confirm no regression for maps that previously relied on kc_data-only data (check map coverage in assembled catalog)
