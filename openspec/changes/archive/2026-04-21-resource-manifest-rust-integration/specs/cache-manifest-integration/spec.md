## ADDED Requirements

### Requirement: Manifest deserialization
The make_list module SHALL deserialize `crates/emukc_bootstrap/assets/resource_manifest.json` into typed Rust structs covering all entry kinds (ship, slotitem, texture-provider, explicit-path).

#### Scenario: Valid manifest loaded
- **WHEN** `resource_manifest.json` exists and contains valid JSON
- **THEN** the deserializer SHALL produce a `ResourceManifest` struct with all entries parsed
- **THEN** each entry SHALL contain its kind, source expressions, and module provenance

#### Scenario: Manifest file missing
- **WHEN** `resource_manifest.json` does not exist
- **THEN** the Manifest strategy SHALL return an error with a clear message indicating the file is missing
- **THEN** the message SHALL suggest running `bun run decode -- --sync-resource-manifest`

#### Scenario: Manifest version mismatch
- **WHEN** the manifest's `version` field is not the expected version (currently 1)
- **THEN** the system SHALL emit a warning
- **THEN** processing SHALL continue (forward-compatible)

### Requirement: Ship ID resolution via Codex
For each ship manifest entry, the system SHALL resolve the `shipMstIdSource` expression to a set of concrete ship IDs by iterating over the Codex `api_mst_ship` data.

#### Scenario: Universal source expression (all ships)
- **WHEN** a ship entry has `shipMstIdSource` matching a known "all ships" pattern (e.g., `"self.shipModel.mstID"`, `"vo.ship.api_id"`)
- **THEN** the resolver SHALL produce all friendly ship IDs from the Codex (ships with `api_aftershipid` or `api_sortno`)

#### Scenario: Unknown source expression
- **WHEN** a ship entry has an unrecognized `shipMstIdSource` expression
- **THEN** the resolver SHALL emit a warning with the expression string
- **THEN** the entry SHALL be skipped (no paths generated)

#### Scenario: Damaged variant resolution
- **WHEN** a ship entry has `damagedSource = "false"`
- **THEN** only the normal (non-damaged) variant path SHALL be generated
- **WHEN** a ship entry has `damagedSource = "true"`
- **THEN** only the damaged variant path SHALL be generated

### Requirement: Slotitem ID resolution via Codex
For each slotitem manifest entry, the system SHALL resolve `slotMstIdSources` expressions to concrete equipment IDs from `api_mst_slotitem`.

#### Scenario: Universal slotitem source
- **WHEN** a slotitem entry has a known "all slotitems" source expression
- **THEN** the resolver SHALL produce all slotitem IDs from the Codex

#### Scenario: Unknown slotitem source expression
- **WHEN** a slotitem entry has an unrecognized `slotMstIdSources` expression
- **THEN** the resolver SHALL emit a warning and skip the entry

### Requirement: Ship resource path generation
The system SHALL generate cache list paths for resolved ship entries using `SuffixUtils` and the same path templates as the existing `make_list/source/kcs2/resources/ship.rs`.

#### Scenario: Ship full graph
- **WHEN** a ship entry has `targetType = "full"` and resolved ID 500
- **THEN** the path SHALL match the pattern `kcs2/resources/ship/full/{id:04}_{suffix}.png` with correct hash suffix

#### Scenario: Ship banner
- **WHEN** a ship entry has `targetType = "banner"` and resolved ID 500
- **THEN** the path SHALL match the pattern `kcs2/resources/ship/banner/{id:04}_{suffix}.png`

### Requirement: Explicit path passthrough
Explicit-path entries from the manifest SHALL be added to the cache list directly without ID resolution.

#### Scenario: Explicit path added
- **WHEN** the manifest contains an explicit-path entry with path `kcs2/resources/battle/banner/001_abc.png`
- **THEN** the cache list SHALL include that path verbatim

### Requirement: Manifest cache list strategy
The `CacheListMakeStrategy` enum SHALL include a `Manifest` variant that generates the cache list from the resource manifest instead of hardcoded ranges.

#### Scenario: Manifest strategy produces ship paths
- **WHEN** `CacheListMakeStrategy::Manifest` is selected
- **THEN** ship resource paths SHALL be derived from manifest entries resolved via Codex
- **THEN** no HTTP HEAD checks SHALL be performed for manifest-covered resources

#### Scenario: Manifest strategy produces explicit paths
- **WHEN** `CacheListMakeStrategy::Manifest` is selected
- **THEN** explicit-path entries from the manifest SHALL be included in the output

#### Scenario: Manifest strategy falls back for uncovered types
- **WHEN** a resource type (e.g., BGM, furniture, map) has no manifest coverage
- **THEN** the Manifest strategy SHALL NOT generate paths for that type
- **THEN** the holes report SHALL note which types had no manifest coverage
