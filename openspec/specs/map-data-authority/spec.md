## Purpose

Per-field authority merge for map cell metadata in the map catalog assembly
pipeline. Each metadata field (color_no, event_id, event_kind, boss_cell_no)
has a designated authoritative source. The assembly order (wikiwiki → overlay →
stat.json) determines which source wins, with later sources having higher
authority for metadata fields.

## Requirements

### Requirement: Field-authority merge for map cell metadata
The map catalog assembly pipeline SHALL use "last non-zero wins" merge for cell metadata fields (`color_no`, `event_id`, `event_kind`) and variant-level `boss_cell_no`. When a later source provides a non-zero value, it SHALL overwrite the current value regardless of what earlier sources provided. Routing fields (`next_cells`, `node_label`, `routing_rules`, `enemy_fleets`, `ship_drops`) SHALL continue using fill-missing semantics.

#### Scenario: Overlay color_no overrides wikiwiki color_no
- **WHEN** wikiwiki provides `color_no = 4` and overlay (merging later) provides `color_no = 5` for the same cell
- **THEN** the assembled cell has `color_no = 5` (later source wins for metadata fields)

#### Scenario: Stat.json event_id overrides overlay inferred event_id
- **WHEN** overlay provides `event_id = 4` (inferred from color_no) and stat.json (merging last) provides `event_id = 5` for the same cell label
- **THEN** the assembled cell has `event_id = 5` (stat.json is highest authority, merges last)

#### Scenario: Wikiwiki next_cells preserved when later sources have no routing
- **WHEN** wikiwiki provides `next_cells = [2, 3]` and later sources provide `next_cells = []` for the same cell
- **THEN** the assembled cell has `next_cells = [2, 3]` (routing fields use fill-missing, empty arrays don't overwrite)

#### Scenario: Stat.json unavailable falls back to overlay color inference
- **WHEN** stat.json has no entry for a cell but overlay provides `color_no = 4` with inferred `event_id = 4, event_kind = 1`
- **THEN** the assembled cell has `event_id = 4, event_kind = 1` (overlay inference stands as no higher authority overrides)

### Requirement: Bootstrap integrates kcs2-mapdata stat.json
The bootstrap pipeline SHALL download and integrate `stat.json` from kcs2-mapdata as a data source for cell type metadata. stat.json SHALL merge LAST in the assembly order (after overlay) so its `event_id`/`event_kind` values are highest authority.

#### Scenario: stat.json download during bootstrap
- **WHEN** bootstrap runs with network access and no cached stat.json
- **THEN** `stat.json` is downloaded from `https://raw.githubusercontent.com/KagamiChan/kcs2-mapdata/master/maps/stat.json`
- **THEN** the file is cached at `.data/stat.json`

#### Scenario: stat.json loaded from cache
- **WHEN** bootstrap runs without network access but `.data/stat.json` exists
- **THEN** the cached file is used without download
- **THEN** the build report records `stat_source: Cached`

#### Scenario: stat.json unavailable (no network, no cache)
- **WHEN** stat.json download fails and no cache exists
- **THEN** bootstrap continues without stat data
- **THEN** the build report records `stat_source: Unavailable`
- **THEN** event types are inferred from overlay `color_no` as fallback

#### Scenario: stat.json merged after overlay in assembly
- **WHEN** the assembly pipeline runs
- **THEN** the merge order is: wikiwiki → overlay → stat
- **THEN** stat.json's `event_id`/`event_kind` values overwrite overlay's inferred values for matched cells

#### Scenario: stat.json reported in build report
- **WHEN** bootstrap completes
- **THEN** the `MapCatalogBuildReport` includes `stat_map_count` (number of maps with stat data) and `stat_source` (Downloaded/Cached/Unavailable)

### Requirement: stat.json label matching with failure mode handling
stat.json cells are keyed by letter label (A, B, C...). Matching to wikiwiki's cell_no SHALL use existing `node_label`-based remap logic. Failure modes SHALL be handled explicitly.

#### Scenario: Unique label match applies stat data
- **WHEN** stat.json has `event_id = 5, event_kind = 1` for label "J" and the wikiwiki variant has exactly one cell with `node_label = "J"`
- **THEN** the matched cell receives `event_id = 5, event_kind = 1`

#### Scenario: Duplicate label skips stat data
- **WHEN** a variant has two cells with `node_label = "A"` and stat.json has data for "A"
- **THEN** stat data is NOT applied to either cell (ambiguous match)
- **THEN** a warning is logged indicating the duplicate label

#### Scenario: Missing label skips silently
- **WHEN** a cell has `node_label = None` and stat.json has data for some labels
- **THEN** stat data is not applied to that cell (no warning — common for start cells and unnamed nodes)

#### Scenario: Label in stat but not in variant
- **WHEN** stat.json has data for label "X" but no cell in the variant has `node_label = "X"`
- **THEN** stat data is skipped (no warning — stat may cover maps/variants not in catalog)

### Requirement: Overlay captures boss_cell_no
The public overlay capture process SHALL extract `boss_cell_no` from real KC API data. The overlay asset SHALL be regenerated after capture code changes.

#### Scenario: boss_cell_no extracted from API data
- **WHEN** a map start API response contains `api_bosscell_no`
- **THEN** the overlay capture records `boss_cell_no` from that field

#### Scenario: boss_cell_no overrides wikiwiki during assembly
- **WHEN** wikiwiki provides `boss_cell_no = 7` and overlay provides `boss_cell_no = 10`
- **THEN** the assembled variant has `boss_cell_no = 10` (overlay merges after wikiwiki, non-zero wins)

#### Scenario: boss_cell_no zero in overlay (not captured)
- **WHEN** overlay has `boss_cell_no = 0` (response-saver format lacks this field) and wikiwiki has `boss_cell_no = 10`
- **THEN** the assembled variant has `boss_cell_no = 10` (wikiwiki value preserved, zero doesn't overwrite)

#### Scenario: Overlay asset regenerated after code changes
- **WHEN** capture.rs or merge.rs are modified to add boss_cell_no or event inference
- **THEN** `cargo run -- wikiwiki-map build-overlays` MUST be run to regenerate `crates/emukc_bootstrap/assets/public_map_catalog_overlays.json`
- **THEN** the regenerated asset is committed to the repo
