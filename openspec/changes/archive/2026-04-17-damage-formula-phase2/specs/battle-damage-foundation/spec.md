## MODIFIED Requirements

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
