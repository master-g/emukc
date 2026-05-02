## MODIFIED Requirements

### Requirement: Day Battle Simulation
When the player encounters a battle node, a day battle SHALL be simulated using
the battle subsystem via the `emukc_battle` crate. Implemented via SortieOps::sortie_battle.

#### Scenario: Normal day battle
- **WHEN** sortie_battle is called with a formation choice
- **THEN** enemy fleet composition is determined from the Codex map cell definition or fallback enemy builder
- **THEN** `BattleContext` SHALL be constructed with `is_sortie: true`
- **THEN** `CryptoRng::new()` SHALL be created and passed to `emukc_battle::simulate_day()`
- **THEN** the `BattleSimulation` output SHALL be stored via `SortieRepository::insert_pending_battle()`
- **THEN** the response includes api_hourai_flag indicating which phases occurred
- **THEN** all damage fields in the response (api_damage, api_fydam/eydam, api_fdam/edam) SHALL contain effective damage values (post-sinking-protection), ensuring client HP animation matches server state
- **THEN** HP tracking SHALL use effective (clamped) damage internally

#### Scenario: Air battle (airbattle)
- **WHEN** the cell event type calls for an airbattle
- **THEN** the `BattleFlow::AIR_BATTLE` phase configuration SHALL be used
- **THEN** only the aerial phase is simulated
- **THEN** no shelling or torpedo phases occur

#### Scenario: Special battle types (ld_airbattle, ld_shooting)
- **WHEN** the cell event type calls for ld_airbattle or ld_shooting
- **THEN** the appropriate battle type SHALL use `BattleFlow::AIR_BATTLE` or `BattleFlow::SURFACE_DAY` with its specific phase configuration
- **THEN** midnight battle is disabled for these types

#### Scenario: Enemy selection
- **WHEN** building the enemy fleet for a map cell
- **THEN** weighted node compositions are used to select from available enemy fleets in the Codex
- **THEN** fallback enemy fleets are used only when Codex enemy data is missing (degraded path)

#### Scenario: Session layer delegates to emukc_battle
- **WHEN** `battle::sortie::orchestrate::run_day_battle()` is called
- **THEN** it SHALL construct `BattleContext` from DB-loaded ship data
- **THEN** it SHALL call `emukc_battle::simulate_day()` and SHALL NOT contain any phase simulation logic
- **THEN** it SHALL delegate response building to `battle::sortie::response` module
- **THEN** it SHALL persist results via the `SortieRepository` trait

### Requirement: Sortie State Machine
A sortie SHALL be a stateful progression through a map, managed by an in-memory
SortieStore accessed through the `SortieRepository` trait. Implemented via SortieOps.

#### Scenario: Sortie start
- **WHEN** a sortie is started for a profile with a valid fleet on a valid map area/stage
- **THEN** the fleet's fuel and ammo are reduced by the map's consumption rate
- **THEN** a new ActiveSortieState is created with map cell data via `SortieRepository::insert_active()`
- **THEN** the starting cell is determined by the map definition
- **THEN** the response includes cell_data with `api_passed: 0` for ALL cells (none visited yet)
- **THEN** the response includes map area/stage identifiers and initial cell position

#### Scenario: Sortie start with unavailable fleet
- **WHEN** the selected fleet is already in a sortie or on an expedition
- **THEN** the operation fails

#### Scenario: Sortie start with invalid map
- **WHEN** the specified map area or stage does not exist in the Codex
- **THEN** the operation fails

#### Scenario: Sortie start with sunk ships in fleet
- **WHEN** any ship in the selected fleet has HP of 0 (sunk)
- **THEN** the operation fails

#### Scenario: Sortie start with locked map
- **WHEN** the specified map has `unlocked = false` for the player
- **THEN** the operation fails with an error response

#### Scenario: HasContext provides SortieRepository
- **WHEN** any domain trait implementation accesses `self.sortie_store()`
- **THEN** it SHALL receive `&dyn SortieRepository` (explicit trait object, no default impl)
- **THEN** the four existing `impl HasContext for (...)` blocks SHALL provide their own `sortie_store()` implementation
