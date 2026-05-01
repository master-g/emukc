## 1. Fix sortno=0 ship classification in generate.rs

- [x] 1.1 In `graph_group_ship_ids_from_cache_rules()` (generate.rs:217), change `graph.api_sortno.is_some()` to `graph.api_sortno.is_some_and(|s| s > 0)` in the friend_graph filter
- [x] 1.2 Add a test confirming shipgraph entries with `api_sortno=Some(0)` are excluded from character_full/character_up ID resolution

## 2. Update EVENT_SHIP_HOLES for ships 6244-6262

- [x] 2.1 Add ships 6244-6262 to `EVENT_SHIP_HOLES.full` in `ship.rs`
- [x] 2.2 Add ships 6244-6262 to `EVENT_SHIP_HOLES.full_dmg`, `EVENT_SHIP_HOLES.up`, and `EVENT_SHIP_HOLES.up_dmg` in `ship.rs` (following existing pattern)
- [x] 2.3 Update `cache_rules.json` `eventShipHoles` to include 6244-6262 in all four categories
- [x] 2.4 Verify existing test `test_real_manifest_path_rules_match_ship_constants` passes with updated holes

## 3. Fix explicit path directory detection in generate.rs

- [x] 3.1 In `generate_explicit_paths()` (generate.rs:751), add a heuristic after the `ends_with('/')` check: skip paths where the last segment has no file extension (no `.` in the final path component) and the path doesn't end with `/`
- [x] 3.2 Add a test confirming `"resources/voice"` and `"resources/friendly_panel/e"` are filtered while `"resources/stype/etext/sp001.png"` is preserved

## 4. Scope template area paths to observed IDs

- [x] 4.1 In `add_template_area_paths()` (resources/mod.rs:261), replace the unconditional `api_mst_mapinfo` iteration with a scoped set of area IDs derived from decoder UI resources or hardcoded fallback
- [x] 4.2 Add a test confirming areas 001-005 do not produce airunit/airunit_extend_confirm paths

## 5. Scope template gauge paths to known gauge maps

- [x] 5.1 In `add_template_gauge_paths()` (resources/mod.rs:231), replace the `api_mst_mapinfo` iteration with the union of `MAP_ID_LIST` and `EVENT_MAP_ID_LIST` from gauge.rs (expose as needed)
- [x] 5.2 Add a test confirming regular maps (e.g., 00101, 00201) do not produce gauge JSON paths

## 6. Verify

- [x] 6.1 Run `cargo test -p emukc_bootstrap` and confirm all tests pass
- [x] 6.2 Run `cargo clippy --workspace` and confirm no new warnings
