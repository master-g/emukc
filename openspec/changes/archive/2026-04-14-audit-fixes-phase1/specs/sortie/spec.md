## MODIFIED Requirements

### Requirement: Map Route Determination
Routes between cells SHALL be determined by the Codex map definitions with random
elements. Route rules MUST evaluate fleet composition against predicates.

#### Scenario: Route with branching rules
- WHEN a cell has multiple outgoing routes
- THEN the route is selected based on route rules evaluated against fleet composition
- THEN route operators (AND, OR, NOT) are applied to fleet predicates (ship type count, drum count, fleet speed, etc.)

#### Scenario: Route with no special rules
- WHEN a cell has routes with no special branch conditions
- THEN the route is selected randomly from available options (weighted)

#### Scenario: Edge case routing
- WHEN no routes are defined for a cell
- THEN the sortie proceeds using default behavior (terminal cell or error)

#### Scenario: Weighted route selection overflow
- WHEN a weighted route roll equals or exceeds the total weight
- THEN the last route target (highest cell number) SHALL be selected
- THEN the first route target SHALL NOT receive disproportionate probability mass

### Requirement: Day Battle Simulation
When the player encounters a battle node, a day battle SHALL be simulated using
the battle subsystem. Implemented via SortieOps::sortie_battle.

#### Scenario: Normal day battle
- WHEN sortie_battle is called with a formation choice
- THEN enemy fleet composition is determined from the Codex map cell definition or fallback enemy builder
- THEN the battle simulation produces: aerial phase (optional), shelling phases, torpedo phase
- THEN battle damage is recorded in the SortieStore as a pending result
- THEN the response includes api_hourai_flag indicating which phases occurred

#### Scenario: Air battle (airbattle)
- WHEN the cell event type calls for an airbattle
- THEN only the aerial phase is simulated
- THEN no shelling or torpedo phases occur

#### Scenario: Special battle types (ld_airbattle, ld_shooting)
- WHEN the cell event type calls for ld_airbattle or ld_shooting
- THEN the appropriate battle mode is used with its specific phase configuration
- THEN midnight battle is disabled for these types

#### Scenario: Enemy selection
- WHEN building the enemy fleet for a map cell
- THEN weighted node compositions are used to select from available enemy fleets in the Codex
- THEN fallback enemy fleets are used only when Codex enemy data is missing (degraded path)

### Requirement: Night Battle
After a day battle, the player SHALL be able to engage in a night battle
when the midnight_flag is set.

#### Scenario: Night battle availability
- WHEN the day battle response indicates midnight_flag is set
- THEN the player can initiate a night battle via sortie_midnight_battle

#### Scenario: Night battle simulation
- WHEN a night battle is initiated
- THEN a single hougeki (shelling) phase is simulated using night battle rules
- THEN night battle damage SHALL NOT be modified by engagement formation modifier
- THEN the result is merged with the pending day battle result

#### Scenario: Night battle unavailability
- WHEN the day battle type does not support midnight (ld_airbattle, ld_shooting, etc.)
- THEN midnight_flag is not set and night battle cannot be initiated

### Requirement: Ship Sinking Protection (轟沈ストッパー)
During battle, a sinking protection mechanism SHALL prevent friendly ships from
being sunk under specific conditions.

#### Scenario: Non-taiha friendly ship protection
- WHEN a friendly ship was not in taiha state at node entry (entry_hp * 4 > max_hp)
- AND the ship is not the flagship
- AND this is a sortie battle (not practice)
- THEN the ship SHALL NOT be reduced to 0 HP
- THEN the remaining HP SHALL be calculated using the protection formula based on entry_hp

#### Scenario: Flagship always protected
- WHEN a friendly flagship would be reduced to 0 HP during a sortie battle
- THEN the flagship SHALL always be protected regardless of taiha state

#### Scenario: Protection formula uses entry HP
- WHEN the protection formula is applied
- THEN the base HP value SHALL be the ship's HP at node entry (entry_hp), not current_hp
- THEN the formula SHALL use integer arithmetic: (entry_hp / 2) + (rand_part * 3) / 10

#### Scenario: Practice and enemy ships excluded
- WHEN the battle is a practice (exercise) battle
- OR the ship is an enemy ship
- THEN no sinking protection is applied

### Requirement: Sortie State Machine
A sortie SHALL be a stateful progression through a map, managed by an in-memory
SortieStore keyed to the profile. Implemented via SortieOps.

#### Scenario: Sortie start
- WHEN a sortie is started for a profile with a valid fleet on a valid map area/stage
- THEN the fleet's fuel and ammo are reduced by the map's consumption rate
- THEN a new ActiveSortieState is created with map cell data
- THEN the starting cell is determined by the map definition
- THEN the response includes cell_data, map area/stage identifiers, and initial cell position

#### Scenario: Sortie start with unavailable fleet
- WHEN the selected fleet is already in a sortie or on an expedition
- THEN the operation fails

#### Scenario: Sortie start with invalid map
- WHEN the specified map area or stage does not exist in the Codex
- THEN the operation fails

#### Scenario: Sortie start with sunk ships in fleet
- WHEN any ship in the selected fleet has HP of 0 (sunk)
- THEN the operation fails

#### Scenario: SortieStore lock resilience
- WHEN a thread panics while holding the SortieStore lock
- THEN subsequent operations SHALL still succeed (no lock poisoning)

### Requirement: Map Records Persistence
Map progress SHALL be persisted in the database via entity::profile::map_record.
Implemented via MapOps.

#### Scenario: Map records initialization
- WHEN map records are first accessed for a profile
- THEN records are created for all maps defined in the Codex

#### Scenario: Map progress update
- WHEN a map stage is cleared during sortie
- THEN the corresponding map_record is updated with the new clear state

#### Scenario: EO map unlock chain
- WHEN an EO map (N-5, N-6, etc.) exists in the Codex
- THEN its prerequisite SHALL be the preceding map in the same area (e.g., 1-5 requires 1-4 cleared, 1-6 requires 1-5 cleared)
- THEN the prerequisite chain SHALL cover all maps including EO maps

## ADDED Requirements

### Requirement: Route Selection Correctness Test
The route selection function SHALL be verified to distribute probability mass
correctly at boundary conditions.

#### Scenario: Roll equals total weight
- WHEN `select_route_target_for_roll` is called with roll equal to the sum of all weights
- THEN the returned target SHALL be the last key in the weight map
- THEN the returned target SHALL NOT be the first key (unless only one key exists)

### Requirement: Sinking Protection Test Coverage
The sinking protection mechanism SHALL have dedicated unit tests verifying all
protection rules.

#### Scenario: Flagship protection test
- WHEN a flagship would take lethal damage in a sortie battle
- THEN the ship survives with protection formula HP

#### Scenario: Non-taiha ship protection test
- WHEN a non-flagship friendly ship with entry_hp > 25% max_hp takes lethal damage
- THEN the ship survives with protection formula HP

#### Scenario: Taiha ship no protection test
- WHEN a non-flagship friendly ship with entry_hp <= 25% max_hp takes lethal damage in sortie
- THEN the ship may be sunk (no protection)

#### Scenario: Entry HP used as base test
- WHEN a ship takes multiple hits reducing current_hp below entry_hp
- THEN the protection formula SHALL use entry_hp as the base, not current_hp
