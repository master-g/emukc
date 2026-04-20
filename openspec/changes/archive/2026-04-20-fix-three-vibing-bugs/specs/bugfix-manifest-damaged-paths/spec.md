## ADDED Requirements

### Requirement: Standard-category ship assets generate base path when damagedSource is "true"
The manifest path generator SHALL always generate the base path for standard ship categories (banner, character_full, character_up, card, etc.) regardless of the `damaged` field value. When `damagedSource == "true"`, the base path SHALL be included. Variant paths SHALL only be generated when `damaged` is `None`.

#### Scenario: Manifest entry with damagedSource "true"
- **WHEN** a ship manifest entry has `damagedSource == Some(true)` and target is a standard category (e.g., "banner")
- **THEN** the generator SHALL produce the base path `kcs2/resources/ship/banner/{ship_id}_{suffix}.png`
- **AND** SHALL NOT produce variant paths

#### Scenario: Manifest entry with no damaged field
- **WHEN** a ship manifest entry has `damaged == None` and target is a standard category with variants
- **THEN** the generator SHALL produce both the base path and all variant paths

#### Scenario: Manifest entry with damagedSource "false"
- **WHEN** a ship manifest entry has `damagedSource == Some(false)` and target is a standard category
- **THEN** the generator SHALL produce the base path
