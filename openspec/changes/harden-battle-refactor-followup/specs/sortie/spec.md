## MODIFIED Requirements

### Requirement: Day Battle Simulation

When the player encounters a battle node, a day battle SHALL be simulated using the battle subsystem. Implemented via `SortieOps::sortie_battle`. The orchestration entry points (`run_day_battle`, `run_night_battle`, `run_sp_midnight_battle`) SHALL accept a caller-supplied `rng: &mut dyn BattleRng` parameter and SHALL NOT construct their own RNG instance internally.

#### Scenario: Normal day battle

- **WHEN** sortie_battle is called with a formation choice
- **THEN** enemy fleet composition is determined from the Codex map cell definition or fallback enemy builder
- **THEN** the battle simulation produces: aerial phase (optional), shelling phases, torpedo phase
- **THEN** battle damage is recorded in the SortieStore as a pending result
- **THEN** the response includes api_hourai_flag indicating which phases occurred
- **THEN** all damage fields in the response (api_damage, api_fydam/eydam, api_fdam/edam) SHALL contain effective damage values (post-sinking-protection), ensuring client HP animation matches server state
- **THEN** HP tracking SHALL use effective (clamped) damage internally

#### Scenario: Air battle (airbattle)

- **WHEN** the cell event type calls for an airbattle
- **THEN** only the aerial phase is simulated
- **THEN** no shelling or torpedo phases occur

#### Scenario: Special battle types (ld_airbattle, ld_shooting)

- **WHEN** the cell event type calls for ld_airbattle or ld_shooting
- **THEN** the appropriate battle mode is used with its specific phase configuration
- **THEN** midnight battle is disabled for these types

#### Scenario: Enemy selection

- **WHEN** building the enemy fleet for a map cell
- **THEN** weighted node compositions are used to select from available enemy fleets in the Codex
- **THEN** fallback enemy fleets are used only when Codex enemy data is missing (degraded path)

#### Scenario: RNG injected at orchestration boundary

- **WHEN** `run_day_battle(store, codex, input, rng)` is called
- **THEN** all random decisions during the day battle SHALL use the provided `rng` value
- **THEN** the function SHALL NOT call `ProductionRng::default()`, `SeededRng::new(...)`, or any other RNG constructor internally

#### Scenario: Tests inject seeded RNG end-to-end

- **WHEN** a gameplay integration test invokes the sortie battle path with a `SeededRng::new(seed)`
- **THEN** every random decision from `simulate_day` and `simulate_night` SHALL consume from that seeded sequence
- **THEN** the test SHALL be able to assert on specific damage and target-selection outcomes deterministically

## ADDED Requirements

### Requirement: Production RNG plumbing through SortieOps

The blanket implementations of `SortieOps` SHALL construct exactly one `ProductionRng` instance per battle entry point (day battle, night battle, sp midnight battle) and SHALL forward that instance to the corresponding `run_*_battle` orchestration function. No `CryptoRng` symbol SHALL remain in the gameplay crate.

#### Scenario: Single RNG construction per battle

- **WHEN** the production server services a sortie battle request
- **THEN** the `SortieOps` implementation SHALL construct one `ProductionRng` and pass it through to the orchestration function
- **THEN** `simulate_day` and any subsequent same-call `simulate_night` SHALL share that RNG state if invoked sequentially within the same entry point

#### Scenario: Symbol rename

- **WHEN** `cargo check --workspace` runs after the change
- **THEN** the symbol `CryptoRng` SHALL NOT exist in `crates/emukc_gameplay`
- **THEN** the symbol `ProductionRng` SHALL exist in `crates/emukc_gameplay/src/game/battle/rng.rs`
- **THEN** every battle orchestration entry point SHALL use `ProductionRng`
