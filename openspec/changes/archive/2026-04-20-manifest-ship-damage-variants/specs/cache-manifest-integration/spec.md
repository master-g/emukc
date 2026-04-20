## MODIFIED Requirements

### Requirement: Ship resource path generation
The system SHALL generate cache list paths for resolved ship entries using `SuffixUtils` and the same path templates as the existing `make_list/source/kcs2/resources/ship.rs`. For each ship entry, the system SHALL generate paths for the base target type AND all applicable damage variant target types, based on the `damagedSource` field.

#### Scenario: Ship full graph with damage variants
- **WHEN** a ship entry has `targetType = "full"` and resolved ID 500 with variable `damagedSource`
- **THEN** the path SHALL include both `kcs2/resources/ship/full/0500_{suffix}_{filename}.png` and `kcs2/resources/ship/full_dmg/0500_{suffix}_{filename}.png`

#### Scenario: Ship banner with all damage variants
- **WHEN** a ship entry has `targetType = "banner"` and resolved ID 500 with variable `damagedSource`
- **THEN** paths SHALL be generated for `banner`, `banner_dmg`, `banner_g_dmg`, and `banner_g`

#### Scenario: Ship card with only base and card_dmg
- **WHEN** a ship entry has `targetType = "card"` and resolved ID 500 with variable `damagedSource`
- **THEN** paths SHALL be generated for `card` and `card_dmg` only
