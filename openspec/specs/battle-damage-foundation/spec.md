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

### Requirement: CV special shelling formula

The system SHALL use a special shelling formula for CV/CVL/CVB ship types when the ship has dive bombers or torpedo bombers equipped. The basic power SHALL be `(1.5 × (dive_bomber_count + torpedo_bomber_count)) + 55` instead of `firepower + 5`.

When no bomber aircraft are equipped, the standard `firepower + 5` formula SHALL be used regardless of ship type.

Bomber count SHALL count each equipped slot that contains a dive bomber (type 7) or torpedo bomber (type 8), NOT the number of individual planes.

#### Scenario: CVL with 2 torpedo bomber slots and 1 dive bomber slot
- **WHEN** a CVL has 3 slots equipped with bombers
- **THEN** basic power is `1.5 × 3 + 55 = 59.5`

#### Scenario: CV with no bombers equipped
- **WHEN** a CV has no dive/torpedo bombers in any slot
- **THEN** basic power uses standard `firepower + 5` formula

#### Scenario: BB with bombers (edge case)
- **WHEN** a BB (not CV type) has seaplane bombers
- **THEN** basic power uses standard `firepower + 5` formula (CV special only for CV/CVL/CVB)

### Requirement: CL light gun correction (軽砲補正)

The system SHALL apply a light gun correction bonus for CL and CLT ship types. The bonus is `√(single_mount_count) + 2 × √(twin_mount_count)` where:
- single_mount_count: number of equipped small caliber main guns
- twin_mount_count: number of equipped medium caliber main guns

The bonus SHALL be added to basic shelling power before formation/engagement modifiers.

This correction SHALL NOT apply to ship types other than CL and CLT.

#### Scenario: CL with 2 single-mount and 1 twin-mount guns
- **WHEN** a CL equips 2 small caliber guns and 1 medium caliber gun
- **THEN** light gun bonus is `√2 + 2 × √1 ≈ 1.41 + 2.0 = 3.41`

#### Scenario: CLT with 3 medium caliber guns
- **WHEN** a CLT equips 3 medium caliber guns (twin-mount)
- **THEN** light gun bonus is `0 + 2 × √3 ≈ 3.46`

#### Scenario: CA with medium guns (no bonus)
- **WHEN** a CA equips medium caliber guns
- **THEN** no light gun correction is applied (CL/CLT only)

### Requirement: Night recon contact bonus (夜偵)

The system SHALL add a night reconnaissance aircraft contact bonus to night battle basic power when:
1. The attacker has a night recon aircraft (type 42) equipped
2. The air battle achieved Air Superiority (制空権確保) or Air Supremacy (制空権保証)

The bonus values:
- Air Supremacy: +9
- Air Superiority: +7
- No air advantage: +5 (night recon present but no air advantage)

When no night recon is equipped, no bonus SHALL be applied.

The bonus SHALL be added to basic night power before the 360 cap.

#### Scenario: Ship with night recon and air supremacy
- **WHEN** a ship has night recon equipped and air battle resulted in supremacy
- **THEN** night power bonus is +9

#### Scenario: Ship with night recon and air superiority
- **WHEN** a ship has night recon equipped and air battle resulted in superiority
- **THEN** night power bonus is +7

#### Scenario: Ship with night recon but no air advantage
- **WHEN** a ship has night recon equipped but air battle resulted in parity or worse
- **THEN** night power bonus is +5

#### Scenario: Ship without night recon
- **WHEN** no night recon aircraft is equipped
- **THEN** no bonus is applied regardless of air state

### Requirement: ASW depth charge projector armor reduction

The system SHALL apply an armor reduction to submarine targets when the attacker has depth charge projector equipment equipped. The armor reduction is `√(equip_asw − 2)` per projector, summed across all projector equipment.

This reduction SHALL be subtracted from the submarine's defense power in the ASW damage calculation.

Depth charge projector item IDs SHALL be identified from the equipment type system (DepthCharge type with specific item ID checks for projector variants).

#### Scenario: Ship with depth charge projector (ASW stat 10)
- **WHEN** an ASW attacker equips a depth charge projector with ASW stat 10
- **THEN** target submarine defense is reduced by `√(10 − 2) = √8 ≈ 2.83`

#### Scenario: Multiple projectors
- **WHEN** an ASW attacker equips two projectors with ASW stats 10 and 8
- **THEN** armor reduction is `√8 + √6 ≈ 2.83 + 2.45 = 5.28`

#### Scenario: No projector equipped
- **WHEN** no depth charge projector is equipped
- **THEN** no armor reduction is applied to the target
