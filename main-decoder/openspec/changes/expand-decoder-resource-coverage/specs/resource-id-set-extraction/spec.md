## ADDED Requirements

### Requirement: Extract directly observable ship ID subsets
The system SHALL extract ship MST IDs only when their membership in a resource subcategory is directly observable in decoded `main.js` source. Valid evidence includes numeric literals in control flow, inline arrays, inline object tables, preload manifests, and other literal enumerations tied to a ship resource category. The extractor SHALL NOT synthesize IDs from Rust baselines, `start2` data, or CDN checks.

#### Scenario: Collects literal ship IDs with provenance
- **WHEN** the extractor processes modules that literally enumerate ship IDs for categories like `special` or `sp_remodel/*`
- **THEN** the output records those ship IDs and the modules that contained the literals

#### Scenario: Leaves runtime-driven categories partial
- **WHEN** a ship resource category is selected through runtime values such as `this._mst_id`, API payloads, or other non-literal data flow without an inline enumerable set
- **THEN** the extractor does not fabricate an exhaustive ship ID set and marks that category unresolved

### Requirement: Extract directly observable slotitem ID subsets
The system SHALL extract slotitem MST IDs only when their membership is literally enumerable in decoded `main.js`. Categories that only load resources through runtime `slotId` parameters remain unresolved rather than being backfilled from external sources.

#### Scenario: Captures literal slotitem IDs when present
- **WHEN** the extractor processes modules containing inline slotitem ID lists, switch cases, or tables associated with a resource category
- **THEN** the output records those slotitem IDs with module provenance

#### Scenario: `btxt_flat` remains unresolved when only runtime IDs appear
- **WHEN** modules only call `getSlotitem(slotId, "btxt_flat")` or `loader.add(slotId, "btxt_flat")` without a literal ID enumeration
- **THEN** the extractor may leave `btxtFlatIds` empty and lists `btxtFlatIds` as unresolved instead of pretending to match `BTXT_FLAT_IDS`

### Requirement: Output makes main.js-only completeness explicit
The extractor output SHALL clearly state that it is limited to what decoded `main.js` makes directly observable.

#### Scenario: JSON structure exposes coverage metadata
- **WHEN** the JSON asset is synced
- **THEN** it contains `coverageMode`, `shipIdSets`, `slotitemIdSets`, `unresolvedKeys`, and `scriptVersion`

### Requirement: Output synced as JSON asset
The extractor output SHALL be written to `crates/emukc_bootstrap/assets/resource_id_sets.json` when `--sync-assets` flag is provided.

#### Scenario: Ship id-set keys stay explicit even when partial
- **WHEN** the JSON asset is synced
- **THEN** `shipIdSets` exposes explicit keys for `specialShips`, `spRemodelShips`, `spRemodelMessageShips`, `cardRoundShips`, and `rewardShips`

#### Scenario: Slotitem id-set keys stay explicit even when unresolved
- **WHEN** the JSON asset is synced
- **THEN** `slotitemIdSets` exposes an explicit `btxtFlatIds` key and unresolved status is conveyed through coverage metadata instead of implied completeness
