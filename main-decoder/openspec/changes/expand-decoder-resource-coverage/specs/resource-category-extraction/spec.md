## ADDED Requirements

### Requirement: Extract ship target type catalog
The system SHALL scan all 2,152 modules and extract every unique `targetType` string argument passed to `resources.getShip`, `ShipLoader.add`, and explicit `"kcs2/resources/ship/"` path patterns. The output SHALL include the complete set of ship resource categories the client uses, grouped so Rust can consume named generation groups without hardcoding category arrays.

#### Scenario: Discovers all standard ship categories
- **WHEN** the extractor processes the module graph
- **THEN** the output contains all categories currently hardcoded in Rust: `album_status`, `banner`, `banner2`, `banner2_dmg`, `banner2_g`, `banner2_g_dmg`, `banner3`, `banner3_g`, `banner3_g_dmg`, `banner_dmg`, `banner_g`, `banner_g_dmg`, `card`, `card_dmg`, `card_round`, `character_full`, `character_full_dmg`, `character_up`, `character_up_dmg`, `full`, `full_dmg`, `icon_box`, `power_up`, `remodel`, `remodel_dmg`, `reward_card`, `reward_icon`, `special`, `supply_character`, `supply_character_dmg`

#### Scenario: Discovers SP remodel subcategories
- **WHEN** the extractor processes modules containing `sp_remodel` patterns
- **THEN** the output contains subcategories: `animation_key`, `full_x2`, `silhouette`, `text_class`, `text_name`, `text_remodel_mes`

#### Scenario: Discovers new categories not in Rust hardcodes
- **WHEN** the game adds a new ship resource category (e.g., `banner4`)
- **THEN** the extractor discovers it and includes it in the output

### Requirement: Extract slot target type catalog
The system SHALL scan all modules and extract every unique `targetType` string argument passed to `resources.getSlotitem`, `SlotLoader.add`, and explicit `"kcs2/resources/slot/"` path patterns. The output SHALL include named generation groups that replace Rust's current slot target-type arrays.

#### Scenario: Discovers all standard slot categories
- **WHEN** the extractor processes the module graph
- **THEN** the output contains all categories: `card`, `card_t`, `item_on`, `item_up`, `remodel`, `statustop_item`, `airunit_banner`, `airunit_fairy`, `airunit_name`, `btxt_flat`, `item_character`

#### Scenario: Output includes module provenance
- **WHEN** a target type is found
- **THEN** the entry records which module IDs and readable names contain that pattern

### Requirement: Output includes Rust-facing generation groups
The extracted category asset SHALL include named generation groups that map cleanly onto Rust's current Default strategy call sites, while remaining separate from concrete ship/slot ID sets.

#### Scenario: Ship Default strategy groups are present
- **WHEN** the JSON asset is generated
- **THEN** it includes enough named ship category groups to replace the inline arrays currently used by `make_non_graph`

#### Scenario: Slot Default strategy groups are present
- **WHEN** the JSON asset is generated
- **THEN** it includes enough named slot category groups to replace the inline arrays currently used by `make_default`

### Requirement: Output synced as JSON asset
The extractor output SHALL be written to `crates/emukc_bootstrap/assets/resource_categories.json` when `--sync-assets` flag is provided.

#### Scenario: JSON structure is valid
- **WHEN** the JSON asset is synced
- **THEN** it contains `shipTargetTypes`, `slotTargetTypes`, `shipGenerationGroups`, `slotGenerationGroups`, `spRemodelSubcategories`, and `scriptVersion`

#### Scenario: Asset includes script version
- **WHEN** the JSON asset is synced
- **THEN** it includes `scriptVersion` matching the decoded main.js version
