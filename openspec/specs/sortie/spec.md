## MODIFIED Requirements

### Requirement: Day Battle Simulation
When the player encounters a battle node, a day battle SHALL be simulated using
the battle subsystem. Implemented via SortieOps::sortie_battle.

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

### Requirement: Sortie State Machine
A sortie SHALL be a stateful progression through a map, managed by an in-memory
SortieStore keyed to the profile. Implemented via SortieOps.

#### Scenario: Sortie start
- **WHEN** a sortie is started for a profile with a valid fleet on a valid map area/stage
- **THEN** the fleet's fuel and ammo are reduced by the map's consumption rate
- **THEN** a new ActiveSortieState is created with map cell data
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

## ADDED Requirements

### Requirement: Map cell data correctness

The Codex map catalog (`map_catalog.json`) SHALL contain correct cell metadata matching the real KanColle game data for all maps with available API captures. Specifically:
- `boss_cell_no` SHALL match the real `api_bosscell_no`
- `color_no` per cell SHALL match real `api_color_no` values
- `event_id` and `event_kind` SHALL be inferred from `color_no` using the standard mapping (color 0=start, 2=resource, 3=maelstrom, 4=battle, 5=boss, 9+=special)

#### Scenario: Boss cell position matches real game
- **WHEN** a map has real KC API capture data
- **THEN** the codex `boss_cell_no` equals the real `api_bosscell_no`

#### Scenario: Cell event types match real game
- **WHEN** a cell in the codex has `color_no` updated from real data
- **THEN** `event_id` and `event_kind` are set to match the color_no mapping

#### Scenario: Battle node not misidentified as safe
- **WHEN** a cell should be a battle node (real `api_color_no` = 4)
- **THEN** the codex has `event_kind = 1` (battle) and `event_id = 4`
- **THEN** the client correctly triggers battle UI when arriving at this cell
