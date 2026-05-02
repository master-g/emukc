## 1. Rename SortieStore inherent methods

- [x] 1.1 Rename `get_pending_battle` → `get_pending_battle_sortie` in `sortie_store.rs`
- [x] 1.2 Rename `insert_pending_battle` → `insert_pending_battle_sortie` in `sortie_store.rs`
- [x] 1.3 Rename `take_pending_battle` → `take_pending_battle_sortie` in `sortie_store.rs`
- [x] 1.4 Add `pub(super) fn get_pending_result_sortie` to `SortieStore` (consistent delegation)
- [x] 1.5 Update `SortieRepository for SortieStore` impl bodies to call renamed inherent methods
- [x] 1.6 Add `#[must_use]` to `SortieRepository::insert_active` in `repository.rs`
- [x] 1.7 **Gate**: `cargo check --workspace` compiles

## 2. Remove dead code and unused imports

- [x] 2.1 Delete `fn enemy_slot_ids(&BattleShipInput)` from `game/sortie.rs`
- [x] 2.2 Make `enemy_slot_ids` in `battle/sortie/response.rs` `pub(crate)`, re-export from `sortie/mod.rs`
- [x] 2.3 Update test imports in `game/sortie.rs` `mod tests` to use `super::battle::sortie::enemy_slot_ids`
- [x] 2.4 Remove `BattleType`, `EngagementType`, `CryptoRng` from `practice/mod.rs` imports
- [x] 2.5 **Gate**: `cargo check --workspace` compiles
- [x] 2.6 **Gate**: `cargo clippy --workspace` no new warnings

## 3. Final verification

- [x] 3.1 `cargo fmt --all`
- [x] 3.2 `cargo test -p emukc_gameplay` — all tests pass
- [x] 3.3 `cargo test --test gameplay_tests` — integration tests pass
- [x] 3.4 `cargo test --workspace` — full suite passes
