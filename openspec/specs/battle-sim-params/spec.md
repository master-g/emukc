## ADDED Requirements

### Requirement: Night battle input struct
`simulate_night` SHALL accept battle parameters via a `NightBattleInput` struct rather than individual parameters.

#### Scenario: simulate_night accepts struct parameter
- **WHEN** `simulate_night` is called
- **THEN** it takes `&Codex`, `NightBattleInput`, and `&mut impl BattleRng` as its only parameters

### Requirement: NightBattleInput contains all battle context
The `NightBattleInput` struct SHALL contain fields: `friendly`, `enemy`, `friendly_formation_id`, `enemy_formation_id`, `engagement`, `air_state`.

#### Scenario: All night battle parameters available through struct
- **WHEN** a caller constructs `NightBattleInput`
- **THEN** all six parameters from the previous signature are accessible as struct fields

### Requirement: Backward-compatible call site update
The single call site in `emukc_gameplay` SHALL be updated to construct `NightBattleInput` and pass it to `simulate_night`.

#### Scenario: Gameplay crate calls updated simulate_night
- **WHEN** `emukc_gameplay` invokes night battle simulation
- **THEN** it constructs `NightBattleInput` and passes it as a single argument
