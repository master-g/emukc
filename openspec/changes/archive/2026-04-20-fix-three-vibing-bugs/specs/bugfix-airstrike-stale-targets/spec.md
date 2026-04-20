## ADDED Requirements

### Requirement: Airstrike selects alive target per slot
The airstrike phase SHALL recompute the list of alive defenders before selecting a target for each individual bomber slot. A slot SHALL NOT target a defender that was sunk by an earlier slot in the same phase.

#### Scenario: Multiple bomber slots with sinking
- **WHEN** an airstrike phase has multiple bomber slots attacking the defender side
- **AND** an early slot sinks a defender
- **THEN** subsequent slots SHALL NOT select the sunk defender's index as a target
- **AND** SHALL select only from currently alive defenders

#### Scenario: All defenders sunk mid-phase
- **WHEN** all defenders are sunk before remaining slots fire
- **THEN** the remaining slots SHALL deal zero damage and skip targeting

#### Scenario: Single bomber slot
- **WHEN** an airstrike phase has only one bomber slot
- **THEN** behavior SHALL be identical to the stale-list version (no regression)
