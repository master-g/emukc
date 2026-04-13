## 1. Codex: Map Prerequisite Data

- [x] 1.1 Add `prerequisites: HashMap<i64, i64>` field to `MapCatalog` (`crates/emukc_model/src/codex/map.rs`) — maps map_id to prerequisite_map_id
- [x] 1.2 Create `build_regular_prerequisites()` function that returns the hardcoded prerequisite table: same-area sequential (N-M requires N-(M-1)), cross-area (N-4 requires (N+1)-1). 1-1 has no entry (None)
- [x] 1.3 Call `build_regular_prerequisites()` in `MapCatalog::from_manifest()` to populate the field
- [x] 1.4 Add `pub fn prerequisite_for(&self, map_id: i64) -> Option<i64>` accessor on `MapCatalog`
- [x] 1.5 Add `pub fn dependents_of(&self, map_id: i64) -> Vec<i64>` accessor — inverse lookup, returns maps whose prerequisite is the given map_id

## 2. Database: Unlock State Column

- [x] 2.1 Add `unlocked: bool` column to `map_record` SeaORM entity (`crates/emukc_db/src/entity/profile/map_record.rs`), default `true` for migration safety
- [x] 2.2 Run `cargo run -- entity generate` or manually update the entity model to include the new column
- [x] 2.3 Update `MapRecord` model struct (`crates/emukc_model/src/profile/map_record.rs`) — add `unlocked: bool` field
- [x] 2.4 Update `DEFAULT_MAP_RECORDS`: set `unlocked: true` only for map 11 (1-1), `unlocked: false` for all others

## 3. Gameplay: Map Unlock Logic

- [x] 3.1 Modify `ensure_map_records_impl()` (`crates/emukc_gameplay/src/game/map.rs:225`) — set `unlocked` based on prerequisite lookup (true if no prerequisite or prerequisite already cleared, false otherwise)
- [x] 3.2 Modify `build_map_infos()` (`crates/emukc_gameplay/src/game/map.rs:327`) — filter out records where `unlocked = false`
- [x] 3.3 Add `check_and_unlock_dependencies_impl()` function to map module — takes a cleared map_id, looks up dependents via codex, sets `unlocked = true` for any that aren't already unlocked, returns newly unlocked map IDs
- [x] 3.4 Add `is_map_unlocked_impl()` helper — checks map_record unlocked status

## 4. Gameplay: Sortie Gate

- [x] 4.1 In `start_sortie_impl()` (`crates/emukc_gameplay/src/game/sortie.rs`), add unlock check after finding the map definition — if map_record has `unlocked = false`, return `GameplayError::Locked`
- [x] 4.2 Verify error propagates correctly to the API handler (`src/bin/net/router/kcsapi/api_req_map/start.rs`) and returns `api_result = -1`

## 5. Gameplay: Battle Result Unlock Notification

- [x] 5.1 Add `api_next_map_ids: Option<Vec<i64>>` field to `SortieBattleResultResponse` (`crates/emukc_gameplay/src/game/sortie_result.rs:64`), with `#[serde(skip_serializing_if = "Option::is_none")]`
- [x] 5.2 In the map clear path of `sortie_battle_result`, after setting `cleared = true`, call `check_and_unlock_dependencies_impl()` to get newly unlocked map IDs
- [x] 5.3 If the returned list is non-empty, set `api_next_map_ids` on the response; otherwise leave it `None`

## 6. Testing

- [x] 6.1 Unit test: `build_regular_prerequisites()` returns correct prerequisites for maps 11-73
- [x] 6.2 Unit test: `dependents_of(11)` returns `[12]` (1-1 cleared unlocks 1-2)
- [x] 6.3 Unit test: `dependents_of(14)` returns `[21]` (1-4 cleared unlocks 2-1)
- [x] 6.4 Integration test: new profile `get_map_infos()` returns only map 1-1
- [x] 6.5 Integration test: after clearing 1-1, `get_map_infos()` includes 1-2 and `battleresult` returns `api_next_map_ids: [12]`
- [x] 6.6 Integration test: `start_sortie` to locked map (e.g., 2-1) fails for new account
- [x] 6.7 Run `cargo test --workspace` for full regression
- [x] 6.8 Run `cargo clippy --workspace` for lint

## 7. Manual Verification

- [ ] 7.1 `cargo run -- serve` — verify new account mapinfo only shows 1-1
- [ ] 7.2 Clear 1-1 boss, verify mapinfo now includes 1-2
- [ ] 7.3 Attempt sortie to 1-2 before clearing 1-1, verify rejection
