## ADDED Requirements

### Requirement: Extract map resource patterns
The system SHALL scan modules for map image loading patterns (`"map/"` paths, map spot references, area image paths) and extract explicit map file lists that are sufficient to reproduce Rust's current default map output without relying on hardcoded filename rules.

#### Scenario: Discovers default map files
- **WHEN** the extractor processes map-related modules
- **THEN** the output includes explicit default-map files covering the current hardcoded default map list

#### Scenario: Discovers event map files
- **WHEN** the extractor processes event map modules
- **THEN** the output includes explicit event-map files covering current spot/image/info variants

### Requirement: Extract furniture resource patterns
The system SHALL scan modules for furniture image loading patterns (`"furniture/"` paths, furniture type references) and extract furniture ID ranges and type information.

#### Scenario: Discovers furniture categories
- **WHEN** the extractor processes furniture modules
- **THEN** the output includes categories: normal, movable, script, thumbnail, reward, card, outside

### Requirement: Extract use item resource patterns
The system SHALL scan modules for use item image loading patterns (`"useitem/"` paths) and extract the separate ID sets needed for both `useitem/card` and `useitem/card_`.

#### Scenario: Discovers use item card and underline ids
- **WHEN** the extractor processes use item modules
- **THEN** the output includes separate id sets covering the current hardcoded lists (102 cards, 38 underlines)

### Requirement: Extract area and world select patterns
The system SHALL scan modules for area image paths (`"area/sally/"`, `"area/airunit/"`) and world select image paths.

#### Scenario: Discovers area image identifiers
- **WHEN** the extractor processes area-related modules
- **THEN** the output includes sally area IDs and airunit area IDs

### Requirement: Output synced as JSON asset
The extractor output SHALL be written to `crates/emukc_bootstrap/assets/ui_resources.json` when `--sync-assets` flag is provided.

#### Scenario: JSON structure is valid
- **WHEN** the JSON asset is synced
- **THEN** it contains nested `map`, `furniture`, `useItem`, `area`, and `worldSelect` objects with `scriptVersion`

#### Scenario: Use item fields are explicit
- **WHEN** the JSON asset is synced
- **THEN** `useItem` contains both `cardIds` and `underlineIds`

#### Scenario: Area and world select fields are explicit
- **WHEN** the JSON asset is synced
- **THEN** `area` contains `sallyIds` and `airunitIds`, and `worldSelect` contains `files`
