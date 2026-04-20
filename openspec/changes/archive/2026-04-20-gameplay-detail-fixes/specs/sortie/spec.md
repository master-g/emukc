## MODIFIED Requirements

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

#### Scenario: Level 99 cap on experience gain (unmarried)
- WHEN a surviving ship has `married == false` in the database and `api_lv >= 99`
- THEN that ship receives 0 experience from the battle
- THEN the experience gain field in the response is 0 for that ship
- THEN no level-up processing occurs for that ship

#### Scenario: Married ship receives experience past 99
- WHEN a surviving ship has `married == true` in the database
- THEN that ship receives normal experience from the battle regardless of level

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

#### Scenario: Practice level 99 cap on experience gain (unmarried)
- WHEN a surviving ship in practice has `married == false` and `api_lv >= 99`
- THEN that ship receives 0 experience from the practice battle
