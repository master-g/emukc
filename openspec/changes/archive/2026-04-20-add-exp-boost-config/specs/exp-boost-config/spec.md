## ADDED Requirements

### Requirement: GameConfig contains ct_exp_boost field
GameConfig SHALL include a `ct_exp_boost: f64` field with serde default of `1.0`. This field controls the exp multiplier applied to all sortie ships when a training cruiser (CT) is the fleet flagship.

#### Scenario: Default value when config omitted
- **WHEN** GameConfig is deserialized without `ct_exp_boost`
- **THEN** `ct_exp_boost` defaults to `1.0` (no extra boost)

#### Scenario: Custom CT boost value
- **WHEN** GameConfig is deserialized with `ct_exp_boost: 1.15`
- **THEN** exp calculations use 1.15 as CT flagship multiplier

### Requirement: GameConfig contains practice_exp_boost field
GameConfig SHALL include a `practice_exp_boost: f64` field with serde default of `1.0`. This field controls an additional exp multiplier applied during practice (演習) battles.

#### Scenario: Default value when config omitted
- **WHEN** GameConfig is deserialized without `practice_exp_boost`
- **THEN** `practice_exp_boost` defaults to `1.0` (no extra boost)

#### Scenario: Custom practice boost value
- **WHEN** GameConfig is deserialized with `practice_exp_boost: 1.5`
- **THEN** practice exp calculations multiply result by 1.5

### Requirement: Sortie exp uses configurable CT multiplier
`calculate_sortie_ship_exp` in `emukc_gameplay` SHALL use `GameConfig.ct_exp_boost` instead of the hardcoded `ct_mult = 300`. When CT is not flagship, multiplier remains 1.0.

#### Scenario: Sortie with CT flagship and ct_exp_boost = 1.15
- **WHEN** a sortie battle completes with CT as flagship and `ct_exp_boost = 1.15`
- **THEN** each ship's exp gain is multiplied by 1.15 (MVP and flagship bonuses still apply on top)

#### Scenario: Sortie without CT flagship
- **WHEN** a sortie battle completes without CT as flagship
- **THEN** exp calculation is unaffected by `ct_exp_boost`

### Requirement: Practice exp uses both multipliers
`calculate_practice_ship_exp` in `emukc_gameplay` SHALL use `GameConfig.ct_exp_boost` (when CT flagship) and `GameConfig.practice_exp_boost` multiplicatively.

#### Scenario: Practice with CT flagship and both boosts
- **WHEN** a practice battle completes with CT as flagship, `ct_exp_boost = 1.15`, `practice_exp_boost = 1.5`
- **THEN** each ship's exp gain = base × ct_exp_boost × practice_exp_boost

#### Scenario: Practice without CT flagship, only practice boost
- **WHEN** a practice battle completes without CT as flagship, `practice_exp_boost = 1.5`
- **THEN** each ship's exp gain = base × 1.5

### Requirement: Multipliers stack multiplicatively
`ct_exp_boost` and `practice_exp_boost` SHALL be applied as independent multipliers. Final exp = floor(base × ct_mult × practice_mult).

#### Scenario: Both boosts applied simultaneously
- **WHEN** both ct_exp_boost > 1.0 and practice_exp_boost > 1.0
- **THEN** final exp = floor(base × ct_exp_boost × practice_exp_boost)
