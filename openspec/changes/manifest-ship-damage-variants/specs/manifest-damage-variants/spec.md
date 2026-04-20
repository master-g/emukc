## ADDED Requirements

### Requirement: Ship damage variant path generation
The manifest-driven ship path generator SHALL produce damage variant paths (`_dmg`, `_g_dmg`, `_g`) for base target types that have known variants. The variant mapping SHALL be defined in a static table.

#### Scenario: Banner entry with variable damagedSource
- **WHEN** a ship manifest entry has `targetType = "banner"` and `damagedSource = "_0x1a3f79"` (obfuscated/variable)
- **THEN** the generator SHALL produce paths for `banner`, `banner_dmg`, `banner_g_dmg`, and `banner_g`

#### Scenario: Full entry with damagedSource false
- **WHEN** a ship manifest entry has `targetType = "full"` and `damagedSource = "false"`
- **THEN** the generator SHALL produce only the `full` path, NOT `full_dmg`

#### Scenario: Full entry with damagedSource true
- **WHEN** a ship manifest entry has `targetType = "full"` and `damagedSource = "true"`
- **THEN** the generator SHALL produce only the `full_dmg` path (the damaged variant of the base type)

#### Scenario: Character entry with variable damagedSource
- **WHEN** a ship manifest entry has `targetType = "character_full"` and `damagedSource = "damaged"`
- **THEN** the generator SHALL produce paths for both `character_full` and `character_full_dmg`

#### Scenario: Entry with no damage variants
- **WHEN** a ship manifest entry has `targetType = "album_status"` (no damage variant exists in the mapping)
- **THEN** the generator SHALL produce only the base `album_status` path regardless of `damagedSource`

### Requirement: Damage variant mapping table
The system SHALL maintain a static mapping from base ship target types to their damage variant target types.

#### Scenario: Known variant mapping
- **WHEN** the variant table is consulted for base type `"banner"`
- **THEN** it SHALL return variants `["banner_dmg", "banner_g_dmg", "banner_g"]`

#### Scenario: Known variant mapping for card
- **WHEN** the variant table is consulted for base type `"card"`
- **THEN** it SHALL return variants `["card_dmg"]`

#### Scenario: Unknown base type
- **WHEN** the variant table is consulted for a base type not in the table (e.g., `"special"`)
- **THEN** it SHALL return no variants (empty)
