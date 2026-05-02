## Context

`crates/emukc_gameplay/src/game/battle/core.rs` is a 4,626-line file containing all battle simulation logic: types, entry points, damage formulas, phase simulation, and helpers. The file contains ~110 non-test functions/methods plus a 1,747-line `#[cfg(test)] mod tests` with 72 test functions. External consumers are:
- `battle/sortie.rs` — imports `BattleContext`, `BattlePacket`, `BattleRuntimeShip`, `BattleSimulation`, `EngagementType`, `NightBattlePacket`, `simulate_day_battle_v1`, `simulate_night_battle_v1`
- `battle/practice.rs` — imports most public types + both simulate functions
- `game/sortie.rs` — imports `BattleContext`, `BattlePacket`, `BattleShipInput`, `BattleType`, `EngagementType`, `BattleNightHougeki`
- `game/sortie_result.rs` — references `BattleShipInput` via `super::battle::core::BattleShipInput`

All external references use `crate::game::battle::core::` or `super::core::` paths.

## Goals / Non-Goals

**Goals:**
- Split `core.rs` into focused modules organized by battle phase
- Maintain exact same public API — all external consumers unchanged
- Enable future per-phase testing and independent modification
- Make it easy to add new phases/mechanics (LBAS, support, combined fleet)

**Non-Goals:**
- Any behavioral change (damage formulas, phase ordering, etc.)
- Changing function signatures or type layouts
- Refactoring `sortie.rs` or `practice.rs`
- Adding new battle mechanics
- Changing the `fix-battle-attack-system` change

## Decisions

### 1. Module structure: phase-oriented split

**Decision:** Organize by phase under `battle/phases/`, with shared types in `battle/types.rs`, damage calculation in `battle/damage.rs`, orchestrator entry points in `battle/simulation.rs`, and outcome functions in `battle/outcome.rs`.

```
battle/
├── mod.rs          — pub re-exports, module declarations
├── types.rs        — BattleContext, BattleRuntimeShip, BattleShipInput,
│                     BattleType, EngagementType, BattlePhase, TargetClass,
│                     AirState, AttackCapability, NightAttackType,
│                     all Serialize structs (BattleKouku*, BattleHougeki,
│                     BattleRaigeki, etc.), ShellingParams, AirstrikeOutput,
│                     NightBattleParams, BattleRandom, TorpedoAttackerSide,
│                     TorpedoHit, equipment type constants, name marker constants
├── damage.rs       — apply_cap, calculate_defense_power, damage_state_modifier,
│                     resolve_damage, calculate_scratch_damage,
│                     calculate_shelling_damage, calculate_torpedo_damage,
│                     calculate_night_damage, calculate_asw_damage,
│                     night_recon_bonus, light_gun_bonus,
│                     improvement_bonus_day/torpedo/night,
│                     shelling/torpedo/asw_formation_modifier,
│                     depth_charge_armor_reduction,
│                     apply_damage (impl method on BattleRuntimeShip)
├── targeting.rs    — select_random_target_index, select_submarine_target,
│                     target_class, ship_mst, ship_type, slotitem_mst,
│                     is_pt_target_name, is_installation_target_name,
│                     has_slotitem_type, has_slotitem_id,
│                     attack_capability_for_phase,
│                     can_shell_day_ship, can_attack_night_ship,
│                     can_attack_submarine_day_shelling,
│                     can_attack_submarine_night_shelling,
│                     can_opening_torpedo_ship, can_closing_torpedo_ship,
│                     can_opening_torpedo, can_closing_torpedo,
│                     any_alive, has_any_air_combat_planes
├── outcome.rs      — calculate_mvp, calculate_win_rank,
│                     verify_protected_ships_alive
├── simulation.rs   — simulate_day_battle_v1, simulate_night_battle_v1
│                     (orchestrator entry points that call phase modules)
├── phases/
│   ├── mod.rs      — module declarations
│   ├── kouku.rs    — simulate_kouku, execute_airstrike_phase,
│   │                 calculate_single_slot_airstrike_damage,
│   │                 is_fighter_power_type, calculate_fighter_power,
│   │                 is_air_combat_type, is_airstrike_attack_type,
│   │                 total_plane_count, total_attack_plane_count,
│   │                 attack_plane_from, first_touch_plane,
│   │                 best_bomber_index, apply_plane_losses
│   ├── asw.rs      — simulate_opening_taisen,
│   │                 can_opening_asw, equipment_asw_total,
│   │                 asw_synergy_modifier, has_active_asw_aircraft
│   ├── torpedo.rs  — simulate_opening_torpedo, simulate_raigeki
│   ├── shelling.rs — simulate_shelling_side,
│   │                 day_attack_display_ids, night_attack_display_ids,
│   │                 is_day_surface_display_type, is_asw_display_slotitem,
│   │                 is_night_main_gun_type, is_night_secondary_gun_type,
│   │                 is_night_torpedo_type, is_radar_type,
│   │                 collect_matching_slot_ids, collect_asw_display_ids,
│   │                 first_or_default, extend_limit,
│   │                 is_cv_type, bomber_slot_count
│   └── night.rs    — simulate_night_hougeki, resolve_night_attack,
│                     night_ci_trigger_rate, detect_night_attack_type,
│                     NightAttackType (enum + impl),
│                     count_equipment_type, is_main_gun_type,
│                     count_main_guns, count_secondary_guns, has_radar
├── practice.rs     — unchanged
└── sortie.rs       — unchanged
```

**Rationale:** Phase-oriented modules align with how KanColle battles actually work (sequential phases), making it natural to add new phases (LBAS, support) alongside existing ones. `simulation.rs` isolates the orchestrator logic that coordinates all phases. `outcome.rs` groups post-battle result calculation. Helper functions are placed with the phase that primarily uses them.

**Alternative considered:** Trait-based phase abstraction (e.g., `trait BattlePhase { fn simulate(...) }`). Rejected as over-engineering for a pure refactor — phases have different signatures and don't share a common interface.

### 2. Visibility strategy

**Decision:** All split modules use `pub(crate)` visibility. `battle/mod.rs` re-exports the original public API (`simulate_day_battle_v1`, `BattleContext`, `BattleSimulation`, etc.) so external consumers see no change.

**Rationale:** Minimizes import changes in `sortie.rs`, `practice.rs`, `game/sortie.rs`, `game/sortie_result.rs`. These files only need to update from `core::X` to `X` (or keep using `core::` if we re-export through it).

### 3. Migration approach: big-bang vs incremental

**Decision:** Big-bang in a single commit. Delete `core.rs`, create all new files, update `mod.rs`, fix imports.

**Rationale:** Incremental would require temporary `pub use` shuffles and dual-path imports that add confusion. Since this is a pure refactor with no behavior change, a single clean commit is safest and easiest to verify (`cargo test` passes before and after).

### 4. Constants placement

**Decision:** Equipment type constants (`DAY_SURFACE_DISPLAY_TYPES`, `ASW_DISPLAY_TYPES`, `NIGHT_*_TYPES`, `RADAR_DISPLAY_TYPES`, `*_TARGET_NAME_MARKERS`) move to `types.rs` alongside the type definitions they support.

**Rationale:** These constants are used across multiple phases. Placing them with the types keeps related items together.

### 5. Test code placement

**Decision:** Move the entire `#[cfg(test)] mod tests` block (L2879-L4626) into a separate `battle/tests.rs` file, declared via `#[cfg(test)] mod tests;` in `battle/mod.rs`. Test helper functions (`sample_ship`, `first_ship_mst_by_type`, etc.) and all 72 test functions move together.

**Rationale:** Splitting tests to follow their corresponding modules would require updating all `use super::*` imports and managing test-only dependencies per module. Keeping tests in one file is simpler for a pure structural refactor and preserves the existing test suite unchanged. Future changes can migrate individual tests to their phase modules as needed.

### 6. Dependency direction

```
types.rs ← damage.rs ← phases/* ← simulation.rs
                    ← targeting.rs ← phases/*
types.rs ← outcome.rs
types.rs ← targeting.rs
```

No circular dependencies. `types.rs` is dependency-free (only types/constants). Phase modules may depend on both `damage.rs` and `targeting.rs` but never on each other. `simulation.rs` depends on all phase modules.

## Risks / Trade-offs

- **Compile breakage during development** → Mitigate by doing all file moves in one pass, then fixing imports. Use `cargo check` frequently.
- **Missed re-export** → Mitigate by compiling after each file creation. The compiler will report missing items.
- **Circular dependencies between new modules** → Mitigate by keeping `types.rs` dependency-free (only types/constants, no logic). Phase modules depend on `types.rs` and `damage.rs`, not on each other. Helper functions are placed with their primary consumer.
- **Merge conflict with `fix-battle-attack-system`** → That change modifies `core.rs` behavior. This refactor moves code without changing behavior. Apply the refactor first (structurally), then rebase the attack-system change onto the new module structure.
