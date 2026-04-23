# pathrules-makelist-integration Specification

## Purpose
TBD - created by archiving change pathrules-makelist. Update Purpose after archive.
## Requirements
### Requirement: generate.rs uses PathRules for category and variant lookups
`generate_entry_paths()` in `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` SHALL check `path_rules()` for ship damage variants, standard categories, full categories, and slot categories. When `path_rules()` returns `Some`, those values SHALL replace the hardcoded `SHIP_DAMAGE_VARIANTS`, `SHIP_STANDARD_CATEGORIES`, `SHIP_FULL_CATEGORIES`, and `SLOT_STANDARD_CATEGORIES` constants.

#### Scenario: pathRules available for generate
- **WHEN** `path_rules()` returns `Some(rules)` during Default/Greedy generation
- **THEN** damage variant lookups SHALL use `rules.ship_damage_variants`
- **THEN** category membership checks SHALL use `rules.ship_standard_categories`, `rules.ship_full_categories`, `rules.slot_standard_categories`
- **THEN** no hardcoded constants SHALL be consulted

#### Scenario: pathRules unavailable for generate
- **WHEN** `path_rules()` returns `None`
- **THEN** `generate_entry_paths()` SHALL use the existing hardcoded constants
- **THEN** output SHALL be identical to current behavior

### Requirement: slot.rs uses PathRules for coverage and generation
Functions in `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/slot.rs` SHALL check `path_rules()` for enemy plane IDs, btxt_flat slot IDs, and character hole IDs.

#### Scenario: pathRules available for slot generation
- **WHEN** `path_rules()` returns `Some(rules)` during Default/Greedy slot generation
- **THEN** `make_enemy_plane()` SHALL use `rules.enemy_plane_ids` instead of `ENEMY_PLANE_MAX_ID`
- **THEN** `make_btxt_flat()` SHALL use `rules.btxt_flat_slot_ids` instead of `BTXT_FLAT_IDS`
- **THEN** `make_character()` SHALL use `rules.character_hole_ids` instead of `CHARACTER_HOLES`

#### Scenario: pathRules unavailable for slot generation
- **WHEN** `path_rules()` returns `None`
- **THEN** slot generation SHALL use existing constants
- **THEN** output SHALL be identical to current behavior

### Requirement: ship.rs uses PathRules for hole and special ship lists
Functions in `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/ship.rs` SHALL check `path_rules()` for event ship holes, enemy ship holes, special ships, sp_remodel data, card rounds, and reward ships.

#### Scenario: pathRules available for ship generation
- **WHEN** `path_rules()` returns `Some(rules)` during Default/Greedy ship generation
- **THEN** hole lookups SHALL use `rules.event_ship_holes`, `rules.enemy_ship_holes`
- **THEN** special ship checks SHALL use `rules.special_ships`
- **THEN** sp_remodel generation SHALL use `rules.sp_remodel_ships`, `rules.sp_remodel_mes`
- **THEN** card/reward generation SHALL use `rules.card_rounds`, `rules.reward_ships`

#### Scenario: pathRules unavailable for ship generation
- **WHEN** `path_rules()` returns `None`
- **THEN** ship generation SHALL use existing `LazyLock` constants
- **THEN** output SHALL be identical to current behavior

### Requirement: has_btxt_flat_coverage uses manifest-derived set
`has_btxt_flat_coverage()` SHALL check `BTXT_FLAT_COVERAGE` OnceLock first. If initialized, it SHALL query the `HashSet<i64>`. If not initialized, it SHALL fall back to the `BTXT_FLAT_IDS` constant.

#### Scenario: Coverage check with manifest-derived set
- **WHEN** a v2 manifest has been loaded and `BTXT_FLAT_COVERAGE` is populated
- **THEN** `has_btxt_flat_coverage(known_id)` SHALL return `true` for IDs in the manifest set
- **THEN** `has_btxt_flat_coverage(unknown_id)` SHALL return `false` for IDs not in the manifest set

#### Scenario: Coverage check without manifest
- **WHEN** no v2 manifest has been loaded and `BTXT_FLAT_COVERAGE` is not populated
- **THEN** `has_btxt_flat_coverage()` SHALL return the same result as `BTXT_FLAT_IDS.contains()`

### Requirement: Output parity validation
The system SHALL include a test that verifies Default strategy output with v2 `pathRules` produces the same resource paths as Default strategy without `pathRules`.

#### Scenario: pathRules values match constants
- **WHEN** `pathRules` fields are populated from the same game version as the hardcoded constants
- **THEN** Default strategy output with `pathRules` SHALL be identical to Default strategy output with constants
- **THEN** Greedy strategy output with `pathRules` SHALL be identical to Greedy strategy output with constants

#### Scenario: pathRules values differ from constants
- **WHEN** `pathRules` fields contain values that differ from hardcoded constants (game version mismatch)
- **THEN** a test warning SHALL report which fields differ and by how many entries
- **THEN** generation SHALL proceed with `pathRules` values (not constants)

