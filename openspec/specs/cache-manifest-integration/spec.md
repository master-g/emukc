## ADDED Requirements

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

### Requirement: Manifest generation uses decoder category groups for deterministic ship and slot gaps
The decoder-driven cache-list generation path SHALL use decoder category-group assets to generate deterministic ship and slot categories that are visible in decoder outputs but not represented as concrete manifest ship/slot entries.

#### Scenario: Ship category is present in decoder categories but absent from manifest entries
- **WHEN** decoder assets show a deterministic ship category such as `power_up` in the decoder category groups
- **THEN** the decoder-driven cache-list generation path MUST generate the corresponding ship paths using the same templates as the existing bootstrap implementation

#### Scenario: Slot category is present in decoder categories but absent from manifest entries
- **WHEN** decoder assets show a deterministic slot category such as `card_t` in the decoder category groups
- **THEN** the decoder-driven cache-list generation path MUST generate the corresponding slot paths using the same templates as the existing bootstrap implementation

### Requirement: Manifest generation constrains sparse ship and slot categories with decoder subsets
The decoder-driven cache-list generation path SHALL use decoder sparse-subset assets to constrain categories whose membership is not universal across all friendly ships or all slotitems.

#### Scenario: Sparse ship subset constrains special-resource generation
- **WHEN** the decoder sparse-subset asset provides an observed ship subset for a sparse category such as `special`, `card_round`, or `reward_*`
- **THEN** cache-list generation MUST limit output for that category to the observed subset instead of expanding the category across all friendly ships

#### Scenario: Sparse ship subset constrains sp_remodel generation
- **WHEN** the decoder sparse-subset asset provides independent subsets for `sp_remodel` image assets and remodel-message assets
- **THEN** cache-list generation MUST apply those subsets separately so `sp_remodel` output is not expanded to unrelated ships

### Requirement: Manifest generation consumes decoder audio and UI coverage assets
The decoder-driven cache-list generation path SHALL consume decoder audio and UI coverage assets to add currently missing non-ship/slot domains into the generated cache list.

#### Scenario: Audio coverage assets are available
- **WHEN** decoder output includes audio coverage data for sound effects, BGM, or voice resources
- **THEN** the decoder-driven cache-list generation path MUST include those audio paths in the candidate cache list

#### Scenario: UI coverage assets are available
- **WHEN** decoder output includes UI coverage data for map, furniture, useitem, area, or world-select resources
- **THEN** the decoder-driven cache-list generation path MUST include those UI paths in the candidate cache list

### Requirement: Decoder-driven generation remains tolerant to partial coverage assets
The decoder-driven cache-list generation path SHALL tolerate missing or partial decoder coverage assets without aborting the entire generation run.

#### Scenario: Optional coverage asset is missing
- **WHEN** a decoder coverage asset for one domain is missing or unreadable
- **THEN** cache-list generation MUST log a warning for that domain
- **THEN** generation MUST continue for the remaining available decoder assets

#### Scenario: Sparse subset is unresolved
- **WHEN** a sparse category is marked `partial` or `unresolved` in the decoder coverage assets
- **THEN** cache-list generation MUST avoid claiming complete decoder coverage for that category
- **THEN** the generation path MUST fall back to the existing bootstrap behavior for that category or skip decoder-only expansion for it
