## ADDED Requirements

### Requirement: Day shelling improvement bonus

The system SHALL add equipment star-level (★) improvement bonuses to day shelling basic power. For each equipped weapon, the bonus is `√(★) × type_weight` where type_weight depends on equipment type:
- Small caliber main gun: 1.0
- Medium caliber main gun: 1.0
- Large caliber main gun: 1.0
- Secondary gun: 1.0
- Torpedo: 1.0
- Seaplane bomber / carrier-based dive bomber / carrier-based torpedo bomber: 1.0

The total improvement bonus SHALL be added to `firepower + 5` before formation/engagement/damage-state modifiers.

#### Scenario: Destroyer with ★10 small gun
- **WHEN** a DD equips a small caliber main gun with ★10 (star level 10)
- **THEN** improvement bonus is `√10 ≈ 3.16`, added to basic shelling power

#### Scenario: Ship with no improved equipment
- **WHEN** all equipment has ★0
- **THEN** improvement bonus is 0, basic power unchanged from current behavior

### Requirement: Torpedo improvement bonus

The system SHALL add equipment star-level improvement bonuses to torpedo basic power. For each equipped torpedo (type: Torpedo, SubmarineTorpedo), the bonus is `★ × 1.2`.

The total improvement bonus SHALL be added to `torpedo_stat` before formation/engagement/damage-state modifiers.

#### Scenario: Ship with ★5 torpedo
- **WHEN** a ship equips a torpedo with ★5
- **THEN** torpedo improvement bonus is `5 × 1.2 = 6.0`, added to basic torpedo power

#### Scenario: Submarine with ★8 submarine torpedo
- **WHEN** a submarine equips a submarine torpedo with ★8
- **THEN** improvement bonus is `8 × 1.2 = 9.6`

### Requirement: Night battle improvement bonus

The system SHALL add equipment star-level improvement bonuses to night battle basic power. The bonus formula matches day shelling: `√(★)` per equipment.

The total improvement bonus SHALL be added to `firepower + torpedo + 5` before cap.

#### Scenario: Ship with ★7 medium gun at night
- **WHEN** a ship equips a medium caliber main gun with ★7
- **THEN** night improvement bonus is `√7 ≈ 2.65`, added to basic night power

#### Scenario: Night battle with multiple improved weapons
- **WHEN** a ship equips a ★4 gun and a ★6 torpedo
- **THEN** night improvement bonus is `√4 + √6 = 2.0 + 2.45 = 4.45`
