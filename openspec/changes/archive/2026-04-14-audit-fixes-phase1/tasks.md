## 1. Bug Fixes

- [x] 1.1 Fix `select_route_target_for_roll` overflow bias — change `.next()` to `.last()` in `map_route.rs:448`
- [x] 1.2 Remove `* engagement.modifier()` from `calculate_night_damage` in `battle/core.rs:1255`
- [x] 1.3 Fix sinking protection base HP — change `self.current_hp` to `self.entry_hp` in `battle/core.rs:190`
- [x] 1.4 Rewrite protection formula to integer arithmetic — replace `floor(0.5 * h + 0.3 * rand_part)` with `(h / 2) + (rand_part * 3) / 10` in `battle/core.rs:193-195`

## 2. EO Map Prerequisites

- [x] 2.1 Extend `build_regular_prerequisites` in `codex/map.rs` to iterate all defined map numbers per area (not just 2..=4), building unlock chains that include EO maps (N-5, N-6, etc.)
- [x] 2.2 Verify `is_map_unlocked_by_default` handles EO maps correctly (no longer falling into `None → false` trap)

## 3. SortieStore Mutex Replacement

- [x] 3.1 Add `parking_lot` dependency to `emukc_gameplay/Cargo.toml`
- [x] 3.2 Replace `std::sync::Mutex` with `parking_lot::Mutex` in `sortie_store.rs`
- [x] 3.3 Remove all `.unwrap()` calls on lock acquisition (parking_lot returns guard directly)

## 4. Test Coverage

- [x] 4.1 Add unit test for `select_route_target_for_roll` with roll = total weight, asserting last key returned
- [x] 4.2 Add unit test for EO map prerequisites: `prerequisite_for(15)` returns `14`, `prerequisite_for(16)` returns `15`, etc.
- [x] 4.3 Add unit tests for `apply_damage` sinking protection: flagship always protected, non-taiha protected, taiha not protected, entry_hp used as base
- [x] 4.4 Verify existing night battle tests pass with engagement modifier removed (update expected values if needed)

## 5. Validation

- [x] 5.1 Run `cargo test -p emukc_gameplay` — all tests pass
- [x] 5.2 Run `cargo test --test gameplay_tests` — all integration tests pass
- [x] 5.3 Run `cargo clippy --workspace` — no new warnings
