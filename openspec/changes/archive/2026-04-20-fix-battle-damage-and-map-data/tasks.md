## 1. Battle Damage Fix (core.rs)

- [x] 1.1 Fix shelling hougeki: change `damage.push(vec![raw_dealt])` to `dealt` at line 982 in `crates/emukc_gameplay/src/game/battle/core.rs`
- [x] 1.2 Fix opening torpedo friendly: change `damage: raw_dealt` to `dealt` in TorpedoHit at line 1033
- [x] 1.3 Fix opening torpedo enemy: change `damage: raw_dealt` to `dealt` in TorpedoHit at line 1063; update binding `(raw_dealt, _)` to `(_, dealt)` at line 1057
- [x] 1.4 Fix closing torpedo friendly: change `damage: raw_dealt` to `dealt` in TorpedoHit at line 1109
- [x] 1.5 Fix closing torpedo enemy: change `damage: raw_dealt` to `dealt` in TorpedoHit at line 1139; update binding at line 1133
- [x] 1.6 Fix kouku stage3 enemy: change `api_edam[target_idx] = raw_dealt` to `dealt` at line 1277
- [x] 1.7 Fix kouku stage3 friendly: change `api_fdam[target_idx] = raw_dealt` to `dealt` at line 1294; update binding at line 1293
- [x] 1.8 Fix OASW friendly: change `damage.push(vec![raw_dealt])` to `dealt` at line 1722
- [x] 1.9 Fix OASW enemy: change `damage.push(vec![raw_dealt])` to `dealt` at line 1749; update binding at line 1741
- [x] 1.10 Fix night battle friendly: change `hit_damages.push(raw_dealt)` to `dealt` at line 2666
- [x] 1.11 Fix night battle enemy: change `hit_damages.push(raw_dealt)` to `dealt` at line 2707; update binding at line 2706

## 2. Map Cell Data Fix (sortie.rs)

- [x] 2.1 Change `passed: cell.cell_no > 0` to `passed: false` in `build_sortie_cell_data` at `crates/emukc_gameplay/src/game/sortie.rs:994`

## 3. Codex Map Data Migration

- [x] 3.1 Write Python migration script to merge real KC API data from `docs/real_data/map_start_data/*.json` into `.data/codex/map_catalog.json`: correct `boss_cell_no`, `color_no`, and infer `event_id`/`event_kind` from color mapping
- [x] 3.2 Run migration script and verify output (boss_cell_no matches real data for all 33 maps)
- [ ] 3.3 Commit map_catalog.json data fix separately from code changes

## 4. Verification

- [x] 4.1 Run `cargo test --workspace` — all existing tests pass
- [x] 4.2 Run `cargo clippy --workspace` — no new warnings
- [ ] 4.3 Manual test: sortie on map 1-3, verify fleet visits nodes sequentially, boss at correct position (cell 10/J), battle damage no longer triggers false sinking
