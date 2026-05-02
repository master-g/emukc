## 1. Suppress missing_docs on types.rs

- [x] 1.1 Add `#[allow(missing_docs)]` attribute to `crates/emukc_battle/src/types.rs` module declaration in `lib.rs`
- [x] 1.2 Add `///` doc comments to public enums: `BattleType`, `EngagementType`, `AirState`, `BattleOutcome`
- [x] 1.3 Add `///` doc comments to public methods and associated functions in `types.rs`

## 2. Annotate dead code

- [x] 2.1 Add `#[allow(dead_code)]` + `// TODO: night battle` to unused constants in `targeting.rs`: `NIGHT_MAIN_GUN_TYPES`, `NIGHT_SECONDARY_GUN_TYPES`, `NIGHT_TORPEDO_TYPES`, `RADAR_DISPLAY_TYPES`
- [x] 2.2 Add `#[allow(dead_code)]` + `// TODO: night battle` to unused functions in `targeting.rs`: `is_night_main_gun_type`, `is_night_secondary_gun_type`, `is_night_torpedo_type`, `is_radar_type`
- [x] 2.3 Add `#[allow(dead_code)]` + `// TODO: airstrike` to unused functions in `damage.rs`: `calculate_single_slot_airstrike_damage`, `is_airstrike_attack_type`
- [x] 2.4 Add `#[allow(dead_code)]` to unused field `is_sortie` in `state.rs`
- [x] 2.5 Remove unused imports in `simulation/night.rs`: `is_night_main_gun_type`, `is_night_secondary_gun_type`, `is_night_torpedo_type`, `is_radar_type`

## 3. Fix misc warnings

- [x] 3.1 Fix doc backticks in `lib.rs` and `types.rs`

## 4. Introduce NightBattleInput parameter struct

- [x] 4.1 Define `NightBattleInput` struct in `types.rs` with fields: `friendly`, `enemy`, `friendly_formation_id`, `enemy_formation_id`, `engagement`, `air_state`
- [x] 4.2 Refactor `simulate_night` in `simulation/mod.rs` to accept `NightBattleInput` instead of 6 individual params
- [x] 4.3 Update call sites in `emukc_gameplay` (sortie + practice orchestrate) to construct `NightBattleInput`
- [x] 4.4 Re-export `NightBattleInput` from `lib.rs`

## 5. Verify

- [x] 5.1 `cargo clippy --workspace` — zero warnings
- [x] 5.2 `cargo test -p emukc_battle` — 70 tests pass (pre-existing failures in emukc_gameplay unrelated)
- [x] 5.3 `cargo fmt --all --check` — formatting clean
