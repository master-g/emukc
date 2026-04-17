## 1. Equipment star bonus helpers

- [x] 1.1 Add `improvement_bonus_day(codex, ship) -> f64` helper: sums `√(star)` per weapon equipment
- [x] 1.2 Add `improvement_bonus_torpedo(codex, ship) -> f64` helper: sums `star × 1.2` per torpedo equipment
- [x] 1.3 Add `improvement_bonus_night(codex, ship) -> f64` helper: sums `√(star)` per weapon equipment (same as day)
- [x] 1.4 Add unit tests for each helper with ★0, ★5, ★10 equipment

## 2. CV special formula

- [x] 2.1 Add `is_cv_type(codex, ship) -> bool` helper checking CV/CVL/CVB
- [x] 2.2 Add `bomber_slot_count(codex, ship) -> i64` counting dive+torpedo bomber slots
- [x] 2.3 Modify `calculate_shelling_damage` to use CV formula when applicable
- [x] 2.4 Add tests: CVL with bombers, CV without bombers, BB ignores CV formula

## 3. CL light gun correction

- [x] 3.1 Add `light_gun_bonus(codex, ship) -> f64` helper: `√single + 2√twin` for CL/CLT only
- [x] 3.2 Integrate into `calculate_shelling_damage` basic power
- [x] 3.3 Add tests: CL with mixed guns, CLT with medium guns, CA no bonus

## 4. Torpedo improvement bonus

- [x] 4.1 Add `&Codex` parameter to `calculate_torpedo_damage`
- [x] 4.2 Add torpedo improvement bonus to basic power
- [x] 4.3 Update callers: `simulate_raigeki`, `simulate_opening_torpedo`
- [x] 4.4 Add test: torpedo with ★5, ★0 baseline unchanged

## 5. Night battle improvements

- [x] 5.1 Add night improvement bonus to `calculate_night_damage` basic power
- [x] 5.2 Add `&Codex` parameter to `calculate_night_damage`
- [x] 5.3 Add `air_state: Option<AirState>` to `simulate_night_battle_v1` and `BattleContext`
- [x] 5.4 Implement 夜偵 contact bonus: +9/+7/+5 based on air state + night recon check
- [x] 5.5 Update callers in `sortie.rs` and `practice.rs` to pass air state
- [x] 5.6 Add tests: night recon + supremacy, night recon + no air advantage, no night recon

## 6. ASW depth charge projector armor reduction

- [x] 6.1 Add `depth_charge_armor_reduction(codex, attacker) -> f64` helper
- [x] 6.2 Apply reduction to defense power in `calculate_asw_damage`
- [x] 6.3 Add tests: projector with ASW 10, multiple projectors, no projector

## 7. Integration validation

- [x] 7.1 Run `cargo test -p emukc_gameplay` — all existing tests pass
- [x] 7.2 Run `cargo clippy --workspace` — no new warnings
- [x] 7.3 Run `cargo test --workspace` — full workspace green
