## MODIFIED Requirements

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
