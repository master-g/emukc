## ADDED Requirements

### Requirement: Unmarried ship level cap enforcement
The system SHALL prevent unmarried ships from exceeding level 99 through any XP-granting mechanism. When a ship is not married and has reached level 99, the system SHALL set XP gain to 0 and SHALL NOT increase the ship's level.

#### Scenario: Practice XP blocked at level 99 for unmarried ship
- **WHEN** an unmarried ship at level 99 gains XP from a practice battle
- **THEN** the ship's XP gain SHALL be 0
- **THEN** the ship's level SHALL remain 99

#### Scenario: Sortie XP blocked at level 99 for unmarried ship
- **WHEN** an unmarried ship at level 99 gains XP from a sortie battle
- **THEN** the ship's XP gain SHALL be 0
- **THEN** the ship's level SHALL remain 99

#### Scenario: Married ship can exceed level 99
- **WHEN** a married ship at level 99 gains XP
- **THEN** the ship's level SHALL increase beyond 99 up to the married cap

#### Scenario: Ship at level 98 gains partial XP to level 99
- **WHEN** an unmarried ship at level 98 gains enough XP to reach level 100
- **THEN** the ship's level SHALL be clamped to 99
- **THEN** excess XP beyond level 99 SHALL be discarded
