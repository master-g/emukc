## ADDED Requirements

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
