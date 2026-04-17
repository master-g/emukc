## 1. Shared Defense Calculation

- [x] 1.1 Add `calculate_defense_power(random: &mut BattleRandom, armor_stat: i64) -> f64` using formula `floor(0.7×A + 0.6×rand(0, floor(A)−1))`
- [x] 1.2 Add edge case handling: when `armor_stat ≤ 1`, random range is empty, use `floor(0.7 × A)` directly

## 2. Damage State Modifier

- [x] 2.1 Add `damage_state_modifier(current_hp: i64, max_hp: i64, phase: BattlePhase) -> f64` with chuuha (×0.7/0.8) and taiha (×0.4/0.0) logic
- [x] 2.2 Integrate damage state into `calculate_shelling_damage()` as pre-cap multiplier
- [x] 2.3 Integrate damage state into `calculate_torpedo_damage()` as pre-cap multiplier
- [x] 2.4 Integrate damage state into `calculate_asw_damage()` as pre-cap multiplier

## 3. Scratch Damage Trigger

- [x] 3.1 Refactor `calculate_shelling_damage()` to check `capped_power < defense` and return scratch damage
- [x] 3.2 Refactor `calculate_torpedo_damage()` to check `capped_power < defense` and return scratch damage
- [x] 3.3 Refactor `calculate_airstrike_damage()` to check `capped_power < defense` and return scratch damage
- [x] 3.4 Refactor `calculate_asw_damage()` to check `capped_power < defense` and return scratch damage
- [x] 3.5 Refactor `calculate_night_damage()` to check `capped_power < defense` and return scratch damage
- [x] 3.6 Add `random` parameter to all `calculate_*_damage()` functions that need it (defense + scratch both need RNG)

## 4. Torpedo Base Power Fix

- [x] 4.1 Remove `+5.0` from `calculate_torpedo_damage()` basic power calculation
- [x] 4.2 Verify shelling `calculate_shelling_damage()` still uses `+5` (no regression)

## 5. Tests

- [x] 5.1 Add unit test for `calculate_defense_power()` with seeded RNG, verify output range matches `[floor(0.7×A), floor(0.7×A + 0.6×(A−1))]`
- [x] 5.2 Add unit test for `damage_state_modifier()` covering all HP ratio thresholds
- [x] 5.3 Add unit test verifying scratch damage triggers when attack < defense
- [x] 5.4 Add unit test verifying normal damage when attack ≥ defense
- [x] 5.5 Add unit test verifying torpedo base power equals `api_raisou` without `+5`
- [x] 5.6 Run `cargo test -p emukc_gameplay` and fix any broken assertions from defense variance
- [x] 5.7 Run `cargo test --workspace` to verify no broader regressions
