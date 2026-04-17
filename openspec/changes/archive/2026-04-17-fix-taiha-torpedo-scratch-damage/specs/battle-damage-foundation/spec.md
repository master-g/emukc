## MODIFIED Requirements

### Requirement: Scratch damage when attack power below defense

When capped attack power is less than defense power, the system SHALL deal scratch (proportional) damage using the formula `floor(0.06 × H_t + 0.08 × random(0, H_t − 1))` where `H_t` is the target's current HP.

This SHALL apply to all attack types (shelling, torpedo, airstrike, ASW, night battle), not just night battle submarine targets.

When capped attack power is greater than or equal to defense power, normal damage `floor(capped_power − defense)` SHALL apply.

When capped attack power is less than or equal to zero, the system SHALL return 0 damage (no scratch, no normal damage).

#### Scenario: Shelling attack weaker than defense
- **WHEN** capped shelling power is 50 and defense power is 60 against a target with 100 HP
- **THEN** scratch damage is applied using `floor(0.06 × 100 + 0.08 × rand(0, 99))`

#### Scenario: Shelling attack stronger than defense
- **WHEN** capped shelling power is 80 and defense power is 60
- **THEN** normal damage `floor(80 − 60) = 20` is applied

#### Scenario: Torpedo attack weaker than defense
- **WHEN** capped torpedo power is 40 and defense power is 50 against a target with 80 HP
- **THEN** scratch damage is applied (NOT forced minimum 1)

#### Scenario: Zero power from taiha torpedo
- **WHEN** a taiha ship (≤25% HP) performs a torpedo attack and `damage_state_modifier` returns 0.0
- **THEN** capped power is 0 and the system SHALL return 0 damage (NOT scratch damage)

#### Scenario: Negative capped power
- **WHEN** capped power is negative due to modifier edge cases
- **THEN** the system SHALL return 0 damage
