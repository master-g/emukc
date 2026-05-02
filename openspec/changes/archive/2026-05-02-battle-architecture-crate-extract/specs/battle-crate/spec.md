## ADDED Requirements

### Requirement: Standalone battle simulation crate
The battle simulation engine SHALL reside in an independent `emukc_battle` crate at `crates/emukc_battle/`. The crate SHALL have zero dependencies on database (no `emukc_db`), HTTP, or gameplay orchestration crates. Its only external dependencies SHALL be `emukc_model` (for `Codex`, API types) and `emukc_crypto` (for RNG primitives used by the `BattleRng` trait implementors).

#### Scenario: Independent compilation
- **WHEN** `cargo check -p emukc_battle` is run
- **THEN** the crate SHALL compile without requiring `emukc_db`, `emukc_gameplay`, or any I/O crate

#### Scenario: Public API includes all types used by session layer
- **WHEN** `battle/sortie.rs` or `battle/practice.rs` import from `emukc_battle`
- **THEN** `BattleContext`, `BattleShipInput`, `BattleRuntimeShip`, `BattleType`, `EngagementType`, `BattleSimulation`, `NightBattleSimulation`, `BattlePacket`, `NightBattlePacket`, `BattleOutcome`, `AirState`, and `BattleRng` SHALL be accessible as `pub` items

#### Scenario: Phase implementations are crate-private
- **WHEN** external code imports from `emukc_battle`
- **THEN** individual phase functions (`simulate_kouku`, `simulate_shelling_side`, etc.) SHALL NOT be accessible
- **THEN** `BattleState` SHALL NOT be accessible outside the crate

#### Scenario: Simulation entry points produce complete output
- **WHEN** `simulate_day()` is called with valid `BattleContext` and `BattleRng` implementations
- **THEN** the returned `BattleSimulation` SHALL contain all fields populated correctly (matches current `simulate_day_battle_v1` output)
- **THEN** the orchestrator SHALL verify sinking protection invariants before returning

### Requirement: Dependency direction
`emukc_battle` SHALL be a leaf dependency in the workspace. `emukc_gameplay` SHALL depend on `emukc_battle`, not vice versa. The dependency chain SHALL be: `emukc_gameplay → emukc_battle → emukc_model → emukc_crypto`.

#### Scenario: No reverse dependency
- **WHEN** `emukc_battle/Cargo.toml` is inspected
- **THEN** it SHALL NOT list `emukc_gameplay` or `emukc_db` as dependencies

#### Scenario: Gameplay implementation code uses battle crate
- **WHEN** `game/sortie.rs` builds battle context and calls simulation
- **THEN** it SHALL use `use emukc_battle::simulate_day` (or appropriate path) to invoke simulation
