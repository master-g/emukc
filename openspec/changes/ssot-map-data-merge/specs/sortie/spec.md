## MODIFIED Requirements

### Requirement: Map cell data correctness

The Codex map catalog SHALL contain cell metadata derived from per-field authority sources. Cell type classification (battle, resource, maelstrom, boss, start) SHALL be authoritative from game client data (kcs2-mapdata `stat.json`) or real API captures (overlay), not from wiki-parsed approximations.

#### Scenario: Boss cell position matches real game
- **WHEN** a map has overlay capture data with `boss_cell_no > 0`
- **THEN** the codex `boss_cell_no` equals the overlay value (real API)
- **THEN** the client correctly identifies the boss node position

#### Scenario: Cell event types match real game
- **WHEN** stat.json provides `event_id` and `event_kind` for a cell with a unique node label
- **THEN** the codex cell has those exact values (stat.json is highest authority)
- **THEN** the client triggers the correct UI for that cell type (battle, resource, maelstrom, etc.)

#### Scenario: Battle node not misidentified as safe
- **WHEN** a cell should be a battle node (stat.json `event_id = 4, event_kind = 1`)
- **THEN** the codex has `event_id = 4, event_kind = 1`
- **THEN** the client correctly triggers battle UI when arriving at this cell

#### Scenario: Map data stable across re-bootstraps
- **WHEN** bootstrap is run multiple times
- **THEN** the assembled map catalog produces identical cell metadata each time
- **THEN** field-authority merge order (wikiwiki → overlay → stat) consistently produces the same results

### Requirement: kc_data map source removed

The map catalog assembly pipeline SHALL NOT use kc_data YAML map data as a source. kc_data's contributions (node labels, route topology, boss flags, inferred color/event) are fully covered by wikiwiki (labels, routing rules, enemies, drops), overlay (color_no, boss_cell_no), and stat.json (event_id/event_kind).

#### Scenario: kc_data not loaded during bootstrap
- **WHEN** bootstrap runs
- **THEN** kc_data `_map/*.json` files are NOT read or parsed
- **THEN** the assembly pipeline uses wikiwiki, overlay, and stat.json only

#### Scenario: kc_data removal does not regress map coverage
- **WHEN** a map exists in kc_data but not in wikiwiki
- **THEN** that map's basic structure is provided by `ensure_synthetic_variants()` (minimal fallback)
- **THEN** stat.json and overlay supplement with authoritative metadata where available

#### Scenario: Duplicate node labels handled safely
- **WHEN** a map variant has duplicate node labels and stat.json has data for that label
- **THEN** stat data is NOT applied to either cell (prevents misattribution)
- **THEN** the cell falls back to overlay color inference for event_id/event_kind
