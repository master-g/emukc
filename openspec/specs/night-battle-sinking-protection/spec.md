## ADDED Requirements

### Requirement: Night battle sinking protection applies during sorties

When `simulate_night` is called for a sortie battle, sinking protection (轟沈ストッパー) SHALL apply to friendly ships exactly as it does during day battles. The `NightBattleInput.is_sortie` field SHALL control this behavior.

#### Scenario: Non-taiha friendly ship survives lethal damage in sortie night battle

- **WHEN** `simulate_night` is called with `NightBattleInput.is_sortie == true`
- **WHEN** a friendly ship that was NOT in taiha (HP > 25% max) at entry receives lethal damage
- **THEN** the ship SHALL survive with HP ≥ 1
- **THEN** the damage applied SHALL be proportional: `floor(0.5 × entry_hp + 0.3 × rand(0..entry_hp))`

#### Scenario: Flagship always survives in sortie night battle

- **WHEN** `simulate_night` is called with `NightBattleInput.is_sortie == true`
- **WHEN** the flagship (index 0) receives lethal damage at any HP state
- **THEN** the flagship SHALL survive with HP ≥ 1

#### Scenario: Taiha non-flagship can be sunk in sortie night battle

- **WHEN** `simulate_night` is called with `NightBattleInput.is_sortie == true`
- **WHEN** a non-flagship friendly ship that WAS in taiha (HP ≤ 25% max) at entry receives lethal damage
- **THEN** the ship MAY be sunk (HP = 0)

#### Scenario: Practice night battle has no sinking protection

- **WHEN** `simulate_night` is called with `NightBattleInput.is_sortie == false`
- **WHEN** any friendly ship receives lethal damage
- **THEN** the ship SHALL be sunk regardless of HP state or flagship status

#### Scenario: Enemy ships never receive sinking protection

- **WHEN** `simulate_night` is called with any `is_sortie` value
- **WHEN** an enemy ship receives lethal damage
- **THEN** the ship SHALL be sunk (HP = 0)

### Requirement: NightBattleInput carries sortie context

The `NightBattleInput` struct SHALL include an `is_sortie: bool` field that indicates whether this night battle occurs during a sortie (true) or practice (false). Callers SHALL supply this field explicitly.

#### Scenario: Sortie night battle caller sets is_sortie to true

- **WHEN** `emukc_gameplay` orchestrate layer calls `simulate_night` for a sortie battle
- **THEN** `NightBattleInput.is_sortie` SHALL be `true`

#### Scenario: Practice night battle caller sets is_sortie to false

- **WHEN** `emukc_gameplay` orchestrate layer calls `simulate_night` for a practice battle
- **THEN** `NightBattleInput.is_sortie` SHALL be `false`
