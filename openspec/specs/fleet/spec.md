## Purpose
Fleet composition and management for EmuKC. Covers fleet slots (up to 4 decks),
ship assignment, naming, resupply, presets, and mission status tracking.

## Requirements

### Requirement: Fleet Slots
Each profile SHALL have up to 4 fleet slots (decks), each holding up to 6 ship
positions. Implemented via FleetOps.

#### Scenario: Initial fleet state
- WHEN a profile is initialized
- THEN fleet slot 1 is unlocked via unlock_fleet_impl with no ships assigned (all -1)
- THEN fleet slots 2, 3, and 4 do not exist yet (not locked, not created)

#### Scenario: Fleet slot unlock
- WHEN unlock_fleet is called for index 2, 3, or 4
- THEN a new fleet record is created with empty ship slots (all -1)
- THEN the fleet is available for ship assignment

#### Scenario: Fleet unlock for invalid index
- WHEN unlock_fleet is called for index 1 (already exists) or any index outside 2-4
- THEN the operation fails

### Requirement: Fleet Ship Assignment
Ships SHALL be assigned to fleet positions as an ordered array of 6 ship IDs
(-1 for empty positions). Implemented via FleetOps::update_fleet_ships.

#### Scenario: Assign ships to fleet
- WHEN update_fleet_ships is called with a valid fleet index and 6-element ship ID array
- THEN the fleet's ship positions are updated to match the array
- THEN ships are stored in the specified order

#### Scenario: Invalid fleet index
- WHEN update_fleet_ships is called with an index that has no fleet record
- THEN the operation fails with an EntryNotFound error

### Requirement: Fleet Retrieval
Fleets SHALL be retrievable individually or as a complete set.

#### Scenario: Get single fleet
- WHEN get_fleet is called with a valid profile_id and index
- THEN the fleet is returned with its current ship assignment and name
- THEN if the fleet has InMission status but return_time has passed, the status is updated to Returning

#### Scenario: Get all fleets
- WHEN get_fleets is called for a profile
- THEN all existing fleet records are returned ordered by index ascending

#### Scenario: Get fleet ships
- WHEN get_fleet_ships is called for a valid fleet
- THEN ship records are returned in the same order as the fleet's ship array

### Requirement: Fleet Naming
Fleet names SHALL be changeable via update_deck_name.

#### Scenario: Update deck name
- WHEN update_deck_name is called with a valid fleet index
- THEN the fleet's name is updated to the provided string

### Requirement: Fleet Resupply
Fleets SHALL consume fuel and ammo during sorties and MUST be resupplied.
Implemented via ComposeOps::charge_supply.

#### Scenario: Resupply fleet
- WHEN a fleet is resupplied via charge_supply
- THEN fuel and ammo are deducted from the profile's materials
- THEN each ship's current fuel and ammo are restored toward maximum values

#### Scenario: Insufficient materials for resupply
- WHEN the profile lacks sufficient fuel or ammo for full resupply
- THEN the operation fails

### Requirement: Fleet Presets
Profiles SHALL be able to save and load fleet composition presets. Implemented via PresetOps.

#### Scenario: Save fleet preset
- WHEN a fleet composition is saved as a preset
- THEN the preset stores the ship IDs for later recall

#### Scenario: Load fleet preset
- WHEN a preset is loaded
- THEN the current fleet composition is replaced with the preset's ships

### Requirement: Fleet Mission Status
Fleets SHALL track whether they are on an expedition, including mission status and return time.

#### Scenario: Mission status normalization
- WHEN a fleet is retrieved that has InMission status but the return_time has passed
- THEN the status is automatically updated to Returning before returning

### Requirement: Remodel correctly assigns equipment to slot array

When a ship is remodeled, newly created equipment items SHALL be assigned to the ship's `api_slot` array (equipment slot IDs), not `api_onslot` (aircraft capacity). The `api_onslot` array SHALL retain the values from `codex.new_ship()` which reflect the ship's actual aircraft capacity per slot.

#### Scenario: Ship remodeled with default equipment
- **WHEN** a ship is remodeled and the target form has 3 default equipment items
- **THEN** each equipment item's database ID is written to `api_slot[0]`, `api_slot[1]`, `api_slot[2]`
- **THEN** `api_onslot` retains the aircraft capacity values from the codex `api_maxeq` data
- **THEN** the ship's equipment slots show the new equipment in the client

#### Scenario: Ship remodeled with no default equipment
- **WHEN** a ship is remodeled and the target form has no default equipment (all `item_id == 0`)
- **THEN** `api_slot` remains `[-1; 5]` (all empty)
- **THEN** `api_onslot` retains correct capacity values (may be all 0 for non-CV ships)

### Requirement: Ship slot aircraft capacity correctness

Ship slot aircraft capacities (`api_maxeq` / `onslot` values in the Codex) SHALL match real KanColle data. Non-CV/CVL/CVB ship types SHALL NOT have slot capacities that exceed reasonable bounds for their ship type.

Specifically:
- CV/CVL/CVB: slot capacities follow official data (can be 0–40+ per slot)
- BB/BBV: seaplane bomber slots typically 0–4
- CA/CAV/CL/CLT/DD: aircraft slots typically 0–4, with most being 0 or 1
- SS/SSV: typically 0–1 for any aircraft slot
- AO: typically 0–4

Any slot capacity exceeding these bounds SHALL be flagged as a data error.

#### Scenario: CA ship slot audit
- **WHEN** ship slot data is loaded into the Codex
- **THEN** no CA-class ship has any slot with capacity exceeding 4
- **THEN** no CA-class ship has a total aircraft capacity exceeding 8

#### Scenario: BB ship slot audit
- **WHEN** ship slot data is loaded into the Codex
- **THEN** no BB-class ship has any slot with capacity exceeding 4

#### Scenario: DD ship slot audit
- **WHEN** ship slot data is loaded into the Codex
- **THEN** no DD-class ship has any slot with capacity exceeding 1

### Requirement: Existing remodel data repair

Ships that were previously remodeled before the slot/onslot fix SHALL have their corrupted `onslot_*` and `slot_*` database values repaired. The repair SHALL reset `onslot_*` to codex `api_maxeq` values and restore correct equipment assignment via `slot_*`.

#### Scenario: Repair corrupted onslot values
- **WHEN** a ship's `onslot_*` values contain numbers that exceed the ship type's maximum aircraft capacity
- **THEN** the repair resets `onslot_*` to the codex `api_maxeq` values for that ship's mst_id
- **THEN** the ship's aircraft capacity display is correct after repair

#### Scenario: No corruption found
- **WHEN** all ships have `onslot_*` values within expected bounds
- **THEN** the repair makes no changes

### Requirement: HP restoration after remodel
The system SHALL restore a ship's current HP to its maximum HP after a successful remodel. After `cal_ship_status` computes the new max HP for the remodeled ship, `api_nowhp` SHALL be set equal to `api_maxhp`.

#### Scenario: Ship at partial HP before remodel
- **WHEN** a ship with 30/50 HP is remodeled to a form with 60 max HP
- **THEN** the ship SHALL have 60/60 HP after remodel completes

#### Scenario: Ship at full HP before remodel
- **WHEN** a ship with 50/50 HP is remodeled to a form with 60 max HP
- **THEN** the ship SHALL have 60/60 HP after remodel completes

### Requirement: CT ship repair time modifier
The system SHALL use the correct repair time modifier for CT (練習巡洋艦) ship type. The modifier SHALL be verified against wikiwiki documentation and applied correctly in the repair time calculation formula.

#### Scenario: CT ship repair time uses correct modifier
- **WHEN** a CT ship is placed in the repair dock
- **THEN** the repair time SHALL be calculated using the verified CT ship_type_mod value

#### Scenario: CT ship repair time is distinct from CL
- **WHEN** a CT and a CL of the same level have the same HP deficit
- **THEN** the repair times SHALL differ if the CT modifier differs from CL
