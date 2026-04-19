## Purpose
Sortie (map excursion) and battle system for EmuKC. Covers the full sortie
state machine (start -> navigate -> battle -> result -> retreat), map routing,
day/night battle simulation, and practice battles.

## Requirements

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

#### Scenario: Sortie start with locked map
- WHEN the specified map has `unlocked = false` for the player
- THEN the operation fails with an error response

### Requirement: Map Navigation (next)
After starting, the player SHALL advance through map cells via the next operation.

#### Scenario: Advance to next cell
- WHEN the player advances from the current cell
- THEN the next cell is determined by the map route rules
- THEN the cell event type determines what happens (battle, resource gain, maelstrom, etc.)
- THEN non-battle cell effects (resource gain, maelstrom damage, item discovery) are applied immediately

#### Scenario: Advance blocked while battle pending
- WHEN the player has an unprocessed battle result
- THEN the advance operation fails

#### Scenario: Boss cell reached
- WHEN the current cell is the boss cell after advancing
- THEN bosscomp is set to true in the response

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

### Requirement: Day Battle Simulation
When the player encounters a battle node, a day battle SHALL be simulated using
the battle subsystem. Implemented via SortieOps::sortie_battle.

#### Scenario: Normal day battle
- WHEN sortie_battle is called with a formation choice
- THEN enemy fleet composition is determined from the Codex map cell definition or fallback enemy builder
- THEN the battle simulation produces: aerial phase (optional), shelling phases, torpedo phase
- THEN battle damage is recorded in the SortieStore as a pending result
- THEN the response includes api_hourai_flag indicating which phases occurred
- THEN all damage fields in the response (api_damage, api_fydam/eydam, api_fdam/edam) SHALL contain raw (pre-clamp) damage values, allowing overkill display
- THEN HP tracking SHALL use effective (clamped) damage internally

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
- THEN the result is merged with the pending day battle result

#### Scenario: Night battle unavailability
- WHEN the day battle type does not support midnight (ld_airbattle, ld_shooting, etc.)
- THEN midnight_flag is not set and night battle cannot be initiated

### Requirement: Battle Result Processing
After battle(s), the result SHALL be claimed via sortie_battle_result.

#### Scenario: Battle result claim
- WHEN sortie_battle_result is called
- THEN the pending battle result is consumed from the SortieStore
- THEN ship HP changes are persisted to the database
- THEN experience is awarded to surviving ships and the admiral (HQ)
- THEN a drop ship may be granted based on drop eligibility and random roll
- THEN quest progress is updated for sortie-related quest conditions (see quest capability)

#### Scenario: Battle result without prior battle
- WHEN no pending battle result exists in the SortieStore
- THEN the operation fails

#### Scenario: Ship sinking
- WHEN a ship's HP reaches 0 after battle
- THEN the ship is marked as sunk in the result
- THEN sunk ships may be excluded from certain post-battle processing

#### Scenario: Map clear unlocks new maps
- WHEN a map is cleared for the first time and the clear triggers prerequisite satisfaction for other maps
- THEN those maps are set to `unlocked = true`
- THEN the battle result response includes `api_next_map_ids` containing the IDs of newly unlocked maps

#### Scenario: Map clear with no new unlocks
- WHEN a map is cleared but no new maps are unlocked (all dependents already unlocked or no dependents)
- THEN `api_next_map_ids` is absent from the battle result response

### Requirement: Sortie Conclusion
The sortie SHALL end by retreating (goback_port) or completing the map.

#### Scenario: Retreat to port
- WHEN sortie_goback_port is called
- THEN the ActiveSortieState is removed from the SortieStore
- THEN the fleet is returned to port status (no longer in sortie)

#### Scenario: Map completion
- WHEN the boss is defeated and the result is processed
- THEN the map record is updated to mark the stage as cleared
- THEN the sortie may continue to additional cells or be ended

### Requirement: Map Records Persistence
Map progress SHALL be persisted in the database via entity::profile::map_record.
Implemented via MapOps.

#### Scenario: Map records initialization
- WHEN map records are first accessed for a profile
- THEN records are created for all maps defined in the Codex
- THEN only map 1-1 has `unlocked = true`; all others have `unlocked = false`

#### Scenario: Map progress update
- WHEN a map stage is cleared during sortie
- THEN the corresponding map_record is updated with the new clear state
- THEN dependent maps are checked and unlocked if their prerequisite is now satisfied

#### Scenario: Mapinfo filtering by unlock
- WHEN `api_get_member/mapinfo` is called
- THEN only maps with `unlocked = true` are included in the response
- THEN locked maps are completely absent from the response (not returned with a locked flag)

### Requirement: Event Map Rank Selection
Event maps SHALL allow difficulty rank selection.

#### Scenario: Select event map rank
- WHEN select_eventmap_rank is called for an event map
- THEN the map record is updated with the selected rank
- THEN the map's enemy compositions and rewards are adjusted by rank

### Requirement: Practice Battle
Practice (exercise) SHALL use the shared battle subsystem against another player's
fleet. Implemented via PracticeOps.

#### Scenario: Practice day battle
- WHEN a practice battle is initiated
- THEN the opponent's fleet data is loaded for battle simulation
- THEN the same battle core is used as sortie battles

#### Scenario: Practice night battle
- WHEN midnight_flag is set after a practice day battle
- THEN a practice night battle can be initiated using the shared midnight battle logic

#### Scenario: Practice result
- WHEN the practice result is processed
- THEN experience is awarded based on practice rules
- THEN quest progress is updated for exercise-related quest conditions

### Requirement: Map cell data correctness

The Codex map catalog SHALL contain cell metadata derived from per-field authority sources. Cell type classification (battle, resource, maelstrom, boss, start) SHALL be authoritative from game client data (kcs2-mapdata `stat.json`) or real API captures (overlay), not from wiki-parsed approximations.

#### Scenario: Boss cell position matches real game
- **WHEN** a map has overlay capture data with `boss_cell_no > 0`
- **THEN** the codex `boss_cell_no` equals the overlay value (real API)
- **THEN** the client correctly identifies the boss node position

#### Scenario: Cell event types match real game
- **WHEN** stat.json provides `event_id` and `event_kind` for a cell with a unique node label
- **THEN** the codex cell has those exact values (stat.json is highest authority)
- **THEN** the client triggers the correct UI for that cell type (battle, resource, maelstrom, etc.)

#### Scenario: Battle node not misidentified as safe
- **WHEN** a cell should be a battle node (stat.json `event_id = 4, event_kind = 1`)
- **THEN** the codex has `event_id = 4, event_kind = 1`
- **THEN** the client correctly triggers battle UI when arriving at this cell

#### Scenario: Map data stable across re-bootstraps
- **WHEN** bootstrap is run multiple times
- **THEN** the assembled map catalog produces identical cell metadata each time
- **THEN** field-authority merge order (wikiwiki → overlay → stat) consistently produces the same results

### Requirement: kc_data map source removed

The map catalog assembly pipeline SHALL NOT use kc_data YAML map data as a source. kc_data's contributions (node labels, route topology, boss flags, inferred color/event) are fully covered by wikiwiki (labels, routing rules, enemies, drops), overlay (color_no, boss_cell_no), and stat.json (event_id/event_kind).

#### Scenario: kc_data not loaded during bootstrap
- **WHEN** bootstrap runs
- **THEN** kc_data `_map/*.json` files are NOT read or parsed
- **THEN** the assembly pipeline uses wikiwiki, overlay, and stat.json only

#### Scenario: kc_data removal does not regress map coverage
- **WHEN** a map exists in kc_data but not in wikiwiki
- **THEN** that map's basic structure is provided by `ensure_synthetic_variants()` (minimal fallback)
- **THEN** stat.json and overlay supplement with authoritative metadata where available

#### Scenario: Duplicate node labels handled safely
- **WHEN** a map variant has duplicate node labels and stat.json has data for that label
- **THEN** stat data is NOT applied to either cell (prevents misattribution)
- **THEN** the cell falls back to overlay color inference for event_id/event_kind
