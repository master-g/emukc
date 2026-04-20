## ADDED Requirements

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
