## 1. Create shared type modules

- [ ] 1.1 Create `battle/types.rs` with all shared types, enums, constants: `BattleType`, `EngagementType` (+ impl), `BattlePhase`, `TargetClass` (+ impl), `AirState` (+ impl), `AttackCapability`, `NightAttackType` (+ impl: `api_sp_list`, `damage_multiplier`, `hit_count`, `ci_coefficient`), `BattleShipInput`, `BattleRuntimeShip` (+ impl: `new`, `hp`, `is_alive`, `is_sunk`), `BattleContext`, `BattleRandom` (+ impl: `new`, `choose_index`, `roll_scratch_damage`, `random_f64_range`, `roll_range`), `ShellingParams`, `AirstrikeOutput`, `NightBattleParams`, `TorpedoAttackerSide`, `TorpedoHit`, all Serialize structs (`BattleKouku`, `BattleKoukuStage1/2/3`, `BattleOpeningAttack` (+ impl: `blank`, `record_torpedo_hit`), `BattleHougeki`, `BattleNightHougeki`, `BattleRaigeki` (+ impl: `blank`, `record_torpedo_hit`), `BattlePacket`, `BattleOutcome`, `BattleSimulation`, `NightBattlePacket`, `NightBattleSimulation`), all constants (`DAY_SURFACE_DISPLAY_TYPES`, `ASW_DISPLAY_TYPES`, `NIGHT_MAIN_GUN_TYPES`, `NIGHT_SECONDARY_GUN_TYPES`, `NIGHT_TORPEDO_TYPES`, `RADAR_DISPLAY_TYPES`, `PT_TARGET_NAME_MARKERS`, `INSTALLATION_TARGET_NAME_MARKERS`), `#[cfg(test)] impl From<BattleShipInput> for BattleRuntimeShip`

## 2. Create damage module

- [ ] 2.1 Create `battle/damage.rs` with: `apply_cap`, `calculate_defense_power`, `damage_state_modifier`, `resolve_damage`, `calculate_scratch_damage`, `calculate_shelling_damage`, `calculate_torpedo_damage`, `calculate_night_damage`, `calculate_asw_damage`, `night_recon_bonus`, `light_gun_bonus`, `improvement_bonus_day`, `improvement_bonus_torpedo`, `improvement_bonus_night`, `shelling_formation_modifier`, `torpedo_formation_modifier`, `asw_formation_modifier`, `depth_charge_armor_reduction`, `apply_damage` method (on `BattleRuntimeShip` via re-import from types)

## 3. Create targeting module

- [ ] 3.1 Create `battle/targeting.rs` with: `select_random_target_index`, `select_submarine_target`, `target_class`, `ship_mst`, `ship_type`, `slotitem_mst`, `is_pt_target_name`, `is_installation_target_name`, `has_slotitem_type`, `has_slotitem_id`, `attack_capability_for_phase`, `can_shell_day_ship`, `can_attack_night_ship`, `can_attack_submarine_day_shelling`, `can_attack_submarine_night_shelling`, `can_opening_torpedo_ship`, `can_closing_torpedo_ship`, `can_opening_torpedo`, `can_closing_torpedo`, `any_alive`, `has_any_air_combat_planes`

## 4. Create outcome module

- [ ] 4.1 Create `battle/outcome.rs` with: `calculate_mvp`, `calculate_win_rank`, `verify_protected_ships_alive`

## 5. Create phase modules

- [ ] 5.1 Create `battle/phases/mod.rs` declaring all phase submodules
- [ ] 5.2 Create `battle/phases/kouku.rs` with: `simulate_kouku`, `execute_airstrike_phase`, `calculate_single_slot_airstrike_damage`, `is_fighter_power_type`, `calculate_fighter_power`, `is_air_combat_type`, `is_airstrike_attack_type`, `total_plane_count`, `total_attack_plane_count`, `attack_plane_from`, `first_touch_plane`, `best_bomber_index`, `apply_plane_losses`
- [ ] 5.3 Create `battle/phases/asw.rs` with: `simulate_opening_taisen`, `can_opening_asw`, `equipment_asw_total`, `asw_synergy_modifier`, `has_active_asw_aircraft`
- [ ] 5.4 Create `battle/phases/torpedo.rs` with: `simulate_opening_torpedo`, `simulate_raigeki`
- [ ] 5.5 Create `battle/phases/shelling.rs` with: `simulate_shelling_side`, `day_attack_display_ids`, `night_attack_display_ids`, `is_day_surface_display_type`, `is_asw_display_slotitem`, `is_night_main_gun_type`, `is_night_secondary_gun_type`, `is_night_torpedo_type`, `is_radar_type`, `collect_matching_slot_ids`, `collect_asw_display_ids`, `first_or_default`, `extend_limit`, `is_cv_type`, `bomber_slot_count`
- [ ] 5.6 Create `battle/phases/night.rs` with: `simulate_night_hougeki`, `resolve_night_attack`, `night_ci_trigger_rate`, `detect_night_attack_type`, `count_equipment_type`, `is_main_gun_type`, `count_main_guns`, `count_secondary_guns`, `has_radar`

## 6. Create simulation module

- [ ] 6.1 Create `battle/simulation.rs` with: `simulate_day_battle_v1`, `simulate_night_battle_v1` (orchestrator entry points that call phase modules in sequence)

## 7. Create test module

- [ ] 7.1 Create `battle/tests.rs` with the entire `#[cfg(test)] mod tests` block from `core.rs` (L2879-L4626: 72 test functions + 8 test helper functions)

## 8. Update module declarations

- [ ] 8.1 Update `battle/mod.rs` to declare `pub(crate) mod types; pub(crate) mod damage; pub(crate) mod targeting; pub(crate) mod outcome; pub(crate) mod simulation; pub(crate) mod phases; #[cfg(test)] mod tests;` and re-export all public items (`simulate_day_battle_v1`, `simulate_night_battle_v1`, `BattleContext`, `BattleSimulation`, `NightBattleSimulation`, `BattlePacket`, `NightBattlePacket`, `BattleOutcome`, `BattleShipInput`, `BattleRuntimeShip`, `BattleType`, `EngagementType`, `AirState`, `BattleKouku`, `BattleHougeki`, `BattleNightHougeki`, `BattleRaigeki`, `BattleOpeningAttack`, `apply_cap`, `any_alive`, `calculate_mvp`, `calculate_win_rank`)

## 9. Fix external imports

- [ ] 9.1 Update `battle/sortie.rs` imports from `super::core::` to use new module paths or re-exports from `mod.rs`
- [ ] 9.2 Update `battle/practice.rs` imports from `game::battle::core::` to use new module paths or re-exports from `mod.rs`
- [ ] 9.3 Update `game/sortie.rs` imports from `battle::core::` to use new module paths or re-exports from `mod.rs`
- [ ] 9.4 Update `game/sortie_result.rs` reference to `super::battle::core::BattleShipInput`

## 10. Delete old file and verify

- [ ] 10.1 Delete `battle/core.rs`
- [ ] 10.2 Run `cargo check -p emukc_gameplay` — fix any remaining compile errors
- [ ] 10.3 Run `cargo test -p emukc_gameplay` — all tests pass
- [ ] 10.4 Run `cargo test --test gameplay_tests` — integration tests pass
- [ ] 10.5 Run `cargo clippy --workspace` — no new warnings
