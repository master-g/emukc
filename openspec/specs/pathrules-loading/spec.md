# pathrules-loading Specification

## Purpose
TBD - created by archiving change pathrules-makelist. Update Purpose after archive.
## Requirements
### Requirement: PathRules deserialization from v2 manifest
The system SHALL deserialize the `pathRules` block from `resource_manifest.json` version 2 into a typed `PathRules` struct. The struct SHALL contain fields for all hardcoded constant categories: `shipDamageVariants` (HashMap), `shipStandardCategories` (Vec), `shipFullCategories` (Vec), `slotStandardCategories` (Vec), `enemyPlaneIds` (Vec), `btxtFlatSlotIds` (Vec), `characterHoleIds` (Vec), `eventShipHoles` (HashMap), `enemyShipHoles` (HashMap), `specialShips` (Vec), `spRemodelShips` (Vec), `spRemodelMes` (Vec), `cardRounds` (Vec), `rewardShips` (Vec).

#### Scenario: Valid v2 manifest with pathRules
- **WHEN** `resource_manifest.json` has `version: 2` and a `pathRules` block
- **THEN** the system SHALL produce a `PathRules` struct with all fields populated
- **THEN** `pathRules` SHALL be stored in a `static PATH_RULES: OnceLock<PathRules>` for downstream access

#### Scenario: v1 manifest without pathRules
- **WHEN** `resource_manifest.json` has `version: 1` and no `pathRules` block
- **THEN** deserialization SHALL succeed without error
- **THEN** `PATH_RULES` OnceLock SHALL remain unpopulated
- **THEN** downstream code SHALL fall back to hardcoded constants

#### Scenario: pathRules present but some fields empty
- **WHEN** `pathRules` exists but some fields are omitted or empty arrays
- **THEN** those fields SHALL deserialize as empty collections (default)
- **THEN** downstream code using those fields SHALL fall back to constants when the collection is empty

### Requirement: Backward-compatible ResourceManifest loading
`ResourceManifest` SHALL accept both v1 and v2 manifests. The `path_rules` field SHALL use `#[serde(default)]` so v1 manifests deserialize with `path_rules: None`.

#### Scenario: Loading v1 manifest
- **WHEN** a v1 manifest (no `pathRules` key) is loaded
- **THEN** `ResourceManifest.path_rules` SHALL be `None`
- **THEN** no warning SHALL be emitted

#### Scenario: Loading v2 manifest
- **WHEN** a v2 manifest with `pathRules` is loaded
- **THEN** `ResourceManifest.path_rules` SHALL be `Some(PathRules { ... })`
- **THEN** `PATH_RULES` and `BTXT_FLAT_COVERAGE` OnceLocks SHALL be populated

### Requirement: BTXT_FLAT_COVERAGE OnceLock initialization
When `pathRules.btxtFlatSlotIds` is present and non-empty, the system SHALL populate a `static BTXT_FLAT_COVERAGE: OnceLock<HashSet<i64>>` from those IDs.

#### Scenario: pathRules provides btxtFlatSlotIds
- **WHEN** v2 manifest is loaded with `pathRules.btxtFlatSlotIds` containing 336 IDs
- **THEN** `BTXT_FLAT_COVERAGE` SHALL be initialized with a `HashSet<i64>` of those 336 IDs
- **THEN** `has_btxt_flat_coverage()` SHALL use this set

#### Scenario: pathRules has no btxtFlatSlotIds
- **WHEN** v1 manifest is loaded (no pathRules)
- **THEN** `BTXT_FLAT_COVERAGE` SHALL remain uninitialized
- **THEN** `has_btxt_flat_coverage()` SHALL fall back to `BTXT_FLAT_IDS` constant

### Requirement: pathRules access helper
The system SHALL provide a `pub(crate) fn path_rules() -> Option<&'static PathRules>` helper that returns the contents of `PATH_RULES` OnceLock.

#### Scenario: Manifest loaded with pathRules
- **WHEN** a v2 manifest has been loaded
- **THEN** `path_rules()` SHALL return `Some(&PathRules)`

#### Scenario: No manifest loaded or v1 manifest
- **WHEN** no manifest has been loaded or a v1 manifest was loaded
- **THEN** `path_rules()` SHALL return `None`

