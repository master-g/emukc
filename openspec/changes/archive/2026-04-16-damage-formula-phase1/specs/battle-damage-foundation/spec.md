## ADDED Requirements

### Requirement: Defense power uses randomized formula

The system SHALL calculate defense power using the formula `floor(0.7 × A_t + 0.6 × random(0, floor(A_t) − 1))` where `A_t` is the defender's armor stat, for all attack types (shelling, torpedo, airstrike, ASW, night battle).

The random component SHALL be a uniform integer in `[0, floor(A_t) − 1]`.

The defense calculation SHALL be extracted into a shared `calculate_defense_power()` function used by all damage calculators.

#### Scenario: Shelling attack against armored target
- **WHEN** a ship with armor stat 80 is attacked by shelling
- **THEN** defense power is `floor(0.7 × 80 + 0.6 × rand(0, 79))` yielding a value in range `[56, 103]`

#### Scenario: Torpedo attack against armored target
- **WHEN** a ship with armor stat 50 is attacked by torpedo
- **THEN** defense power uses the same randomized formula (NOT `A × 0.55`)

#### Scenario: Airstrike against armored target
- **WHEN** a ship with armor stat 60 is attacked by airstrike
- **THEN** defense power uses the same randomized formula (NOT `A × 0.6`)

#### Scenario: Armor stat of 1
- **WHEN** a ship has armor stat 1
- **THEN** defense power is `floor(0.7 + 0)` = 0 (no random range when `floor(A) − 1 = 0`)

### Requirement: Damage state modifier reduces attack power

The system SHALL apply a pre-cap damage state modifier based on the attacker's HP ratio at the time of attack:

| HP Ratio | State | Shelling/ASW | Torpedo |
|---|---|---|---|
| >75% | Normal | ×1.0 | ×1.0 |
| 25%–75% | Chuuha | ×0.7 | ×0.8 |
| <25% | Taiha | ×0.4 | ×0.0 |

The modifier SHALL be applied after formation and engagement modifiers, before the cap function.

#### Scenario: Chuuha ship in shelling phase
- **WHEN** a ship with 40% HP remaining performs a shelling attack
- **THEN** attack power is multiplied by 0.7 before cap application

#### Scenario: Taiha ship attempting torpedo
- **WHEN** a ship with 20% HP remaining performs a torpedo attack
- **THEN** attack power is multiplied by 0.0, resulting in zero damage

#### Scenario: Chuuha ship in torpedo phase
- **WHEN** a ship with 50% HP remaining performs a torpedo attack
- **THEN** attack power is multiplied by 0.8 before cap application

#### Scenario: ASW attack by chuuha ship
- **WHEN** a ship with 30% HP remaining performs an ASW attack
- **THEN** attack power is multiplied by 0.7 before cap application

### Requirement: Scratch damage when attack power below defense

When capped attack power is less than defense power, the system SHALL deal scratch (proportional) damage using the formula `floor(0.06 × H_t + 0.08 × random(0, H_t − 1))` where `H_t` is the target's current HP.

This SHALL apply to all attack types (shelling, torpedo, airstrike, ASW, night battle), not just night battle submarine targets.

When capped attack power is greater than or equal to defense power, normal damage `floor(capped_power − defense)` SHALL apply.

#### Scenario: Shelling attack weaker than defense
- **WHEN** capped shelling power is 50 and defense power is 60 against a target with 100 HP
- **THEN** scratch damage is applied using `floor(0.06 × 100 + 0.08 × rand(0, 99))`

#### Scenario: Shelling attack stronger than defense
- **WHEN** capped shelling power is 80 and defense power is 60
- **THEN** normal damage `floor(80 − 60) = 20` is applied

#### Scenario: Torpedo attack weaker than defense
- **WHEN** capped torpedo power is 40 and defense power is 50 against a target with 80 HP
- **THEN** scratch damage is applied (NOT forced minimum 1)

### Requirement: Torpedo basic power without shelling constant

Torpedo basic attack power SHALL be calculated as `torpedo_stat` (the ship's `api_raisou` base stat), without the `+5` constant used in shelling.

The `+5` constant is specific to shelling and SHALL NOT be applied to opening torpedo, closing torpedo, or any other torpedo attack.

#### Scenario: Opening torpedo calculation
- **WHEN** a ship with torpedo stat 100 performs opening torpedo
- **THEN** basic attack power is 100 (NOT 105)

#### Scenario: Closing torpedo calculation
- **WHEN** a ship with torpedo stat 80 performs closing torpedo
- **THEN** basic attack power is 80 (NOT 85)

#### Scenario: Shelling still uses +5
- **WHEN** a ship with firepower stat 100 performs shelling attack
- **THEN** basic attack power is 105 (shelling formula unchanged)
