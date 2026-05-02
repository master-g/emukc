## MODIFIED Requirements

### Requirement: Damage application with mode-dependent capping
The system SHALL apply damage differently based on target type and battle mode. For enemy ships in sortie battles, the system SHALL allow effective damage to exceed current HP (overkill), permitting HP to go negative in battle results. For enemy ships in practice battles, effective damage SHALL be capped at current HP. For friendly ships in sortie battles, sinking protection (轟沈ストッパー) SHALL apply as currently implemented. For friendly ships in practice battles, effective damage SHALL be capped at current HP.

#### Scenario: Enemy ship receives overkill damage in sortie
- **WHEN** an enemy ship has 50 HP remaining and receives 200 raw damage in a sortie battle
- **THEN** the enemy ship's HP SHALL be set to -150 and effective damage SHALL be reported as 200

#### Scenario: Enemy ship damage capped in practice
- **WHEN** an enemy ship has 50 HP remaining and receives 200 raw damage in a practice battle
- **THEN** the enemy ship's HP SHALL be set to 0 and effective damage SHALL be reported as 50

#### Scenario: Friendly ship sinking protection unchanged
- **WHEN** a friendly ship in sortie has 100 HP remaining and receives 150 raw damage
- **THEN** sinking protection SHALL apply as currently implemented (ship not sunk unless taiha at node entry)

#### Scenario: Friendly ship damage capped in practice
- **WHEN** a friendly ship in practice has 50 HP remaining and receives 200 raw damage
- **THEN** effective damage SHALL be capped at 50 and HP SHALL be set to 0
