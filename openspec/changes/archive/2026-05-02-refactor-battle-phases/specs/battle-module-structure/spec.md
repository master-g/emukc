## ADDED Requirements

### Requirement: Battle module structure
The battle simulation code SHALL be organized into phase-oriented modules under `crates/emukc_gameplay/src/game/battle/`. The public API (types, entry functions) SHALL remain identical to the pre-refactor `core.rs` exports, so that all external consumers (`sortie.rs`, `practice.rs`, `game/sortie.rs`, `game/sortie_result.rs`) require no behavioral changes.

#### Scenario: External consumers compile unchanged
- **WHEN** the refactored module structure is in place
- **THEN** `battle::sortie.rs` imports `BattleContext`, `BattlePacket`, `BattleSimulation`, `EngagementType`, `NightBattlePacket`, `simulate_day_battle_v1`, `simulate_night_battle_v1` from the battle module
- **THEN** `battle::practice.rs` imports all previously-used types and functions
- **THEN** `game::sortie.rs` imports `BattleShipInput`, `BattleType`, `EngagementType`, `BattleContext`, `BattlePacket`, `BattleNightHougeki`
- **THEN** `game::sortie_result.rs` references `BattleShipInput`

#### Scenario: Day battle simulation produces identical output
- **WHEN** `simulate_day_battle_v1` is called with the same `BattleContext` and no concurrent behavioral changes have been applied
- **THEN** the `BattleSimulation` output SHALL be bit-identical to pre-refactor output (same RNG sequence, same damage values, same packet structure)

#### Scenario: Night battle simulation produces identical output
- **WHEN** `simulate_night_battle_v1` is called with the same inputs and no concurrent behavioral changes have been applied
- **THEN** the `NightBattleSimulation` output SHALL be bit-identical to pre-refactor output

#### Scenario: All existing tests pass
- **WHEN** `cargo test --test gameplay_tests` and `cargo test -p emukc_gameplay` are run
- **THEN** all tests pass without modification

#### Scenario: Module file organization
- **WHEN** the battle module directory is inspected
- **THEN** the following files exist: `mod.rs`, `types.rs`, `damage.rs`, `targeting.rs`, `outcome.rs`, `simulation.rs`, `tests.rs`, `phases/mod.rs`, `phases/kouku.rs`, `phases/asw.rs`, `phases/torpedo.rs`, `phases/shelling.rs`, `phases/night.rs`
- **THEN** `core.rs` no longer exists

#### Scenario: Dependency direction is acyclic
- **WHEN** module dependencies are analyzed
- **THEN** `types.rs` has no dependencies on other battle sub-modules
- **THEN** `damage.rs` depends only on `types.rs`
- **THEN** `targeting.rs` depends only on `types.rs`
- **THEN** phase modules (`phases/*`) depend on `types.rs`, `damage.rs`, and `targeting.rs` but not on each other
- **THEN** `simulation.rs` depends on phase modules and `outcome.rs`
