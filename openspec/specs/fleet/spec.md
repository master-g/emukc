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
