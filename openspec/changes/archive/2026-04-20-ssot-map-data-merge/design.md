## Context

EmuKC's map catalog is assembled from three sources during bootstrap:

1. **wikiwiki catalog** — parsed from wikiwiki.jp HTML. Provides routing rules, node labels, enemy compositions, ship drops, and (often wrong) cell metadata. Loaded via `include_str!` from repo-tracked assets.
2. **public overlay catalog** — built from real KC API response captures (`crates/emukc_bootstrap/assets/real_map_start_data/*.json`), compiled into `crates/emukc_bootstrap/assets/public_map_catalog_overlays.json`. Provides `color_no`, `master_cell_id`, `distance` per cell. Loaded via `include_str!`.

The assembly order in `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` is: `wikiwiki.merge_missing_from(kcdata)` → `catalog.merge_missing_from(overlay)`. Because `merge_missing_from` only fills zero/empty fields, wikiwiki's wrong `color_no`/`event_id`/`event_kind` values block overlay's correct ones.

**kc_data is being removed**: kc_data YAML (`data/kc_data/_map/*.json`) provides node labels, simple route topology, boss flags, and inferred color/event values. Every one of these fields is already covered by a higher-authority source:
- Node labels + route topology: wikiwiki provides richer data including routing predicates
- Boss flags: overlay provides authoritative `boss_cell_no` from real API
- Color/event inference: stat.json provides authoritative `event_id`/`event_kind`; overlay provides authoritative `color_no`

Removing kc_data simplifies the pipeline to three non-overlapping authority sources with clear field ownership.

A third source is being added: **kcs2-mapdata `stat.json`** from KagamiChan's GitHub repo. It contains `event_id`/`event_kind` per cell label per map (e.g., `"13": {"J": {"event_id":5,"event_kind":1}}`), derived from the game client's start2 API.

## Goals / Non-Goals

**Goals:**
- Cell metadata fields use "last non-zero wins" merge — assembly order determines authority
- `event_id`/`event_kind`: stat.json is highest authority (merges last)
- `color_no`/`boss_cell_no`: overlay is authority (merges before stat)
- Routing/enemies/drops: wikiwiki is authority (fill-missing only, merges first)
- Data correct after re-bootstrap — no one-time migration scripts
- Overlay static asset includes `boss_cell_no` and inferred `event_id`/`event_kind`
- stat.json source has full plumbing: loading, caching, reporting, fallback

**Non-Goals:**
- Changing routing rules or predicates
- Dynamically fetching sources at runtime — all sources are repo-tracked assets
- Supporting maps with no authoritative data source
- Adding new Codex fields or API handlers

## Decisions

### D1: "Last non-zero wins" for metadata fields, assembly order = authority

**Decision**: In `merge_cells()` (`crates/emukc_model/src/codex/map/merge.rs`), change merge strategy for `color_no`, `event_id`, `event_kind` from fill-missing to "overwrite when source value > 0". Keep fill-missing for routing fields (`next_cells`, `node_label`, `master_cell_id`, `distance`).

Assembly order determines which source wins for a given field:

```
wikiwiki  →  overlay  →  stat
 (first)     (second)    (third/highest)
```

For `color_no`: wikiwiki(wrong=4) → overlay(correct=5, overwrites) → stat(no data, skip). Result: 5.
For `event_id`: wikiwiki(0) → overlay(inferred=4, overwrites 0) → stat(authoritative=5, overwrites 4). Result: 5.
For `boss_cell_no`: wikiwiki(wrong=7) → overlay(correct=10, overwrites) → stat(no data, skip). Result: 10.
For `next_cells`: wikiwiki([2,3]) → overlay([], empty so no overwrite) → stat([], empty so no overwrite). Result: [2,3].

This makes stat the highest authority for event types, overlay the authority for colors/boss, and wikiwiki the authority for routing — all through a single merge rule with no source-specific logic needed in the merge function itself.

**Alternative considered**: Field-specific merge methods per source — rejected because it requires the merge function to know which source it's merging, adding coupling.

### D1b: Remove kc_data map source

**Decision**: Remove `kcdata_map_count`, `kcdata_catalog` from `ResolvedMapSources`, remove `kcdata_map_count` from `MapCatalogBuildReport`, remove the kcdata merge step from `assemble_final_map_catalog`. Remove or archive `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`.

**Rationale**: kc_data YAML provides (node labels, route topology, boss flags, inferred color/event) — all redundantly covered by wikiwiki (labels, routing rules), overlay (color_no, boss_cell_no), and stat.json (event_id/event_kind). The `build_variant_from_kcdata` function produces empty `routing_rules`, `enemy_fleets`, `ship_drops` — wikiwiki provides all of these. kc_data only served as a pre-wikiwiki fallback that is no longer needed.

**Note**: The kc_data `load_map_catalog_from_cache_root` function (JSON spots format) is a separate code path for kcs2 resource cache data — that is NOT being removed, only the YAML `_map` path.

### D2: Overlay asset regeneration as explicit workflow step

**Decision**: The overlay asset (`crates/emukc_bootstrap/assets/public_map_catalog_overlays.json`) is a repo-tracked static file loaded via `include_str!`. Code changes to `capture.rs` and `merge.rs` (adding `boss_cell_no`, event inference) do NOT affect bootstrap until this file is regenerated.

Regeneration command: `cargo run -- wikiwiki-map build-overlays` (entry point: `src/bin/cli/wikiwiki_map.rs:144`).

This must be an explicit task: run the command, verify output, commit the updated asset.

**Rationale**: The overlay build pipeline reads embedded `real_map_start_data/*.json` captures and produces the compiled overlay. Changes to the build code (capture + merge) only take effect after regeneration.

### D3: stat.json as cached downloadable asset

**Decision**: stat.json is downloaded during bootstrap and cached at `.data/stat.json`. On subsequent bootstraps, the cached file is used if available. If download fails and no cache exists, bootstrap continues without stat data (overlay's color inference acts as fallback).

**Source plumbing additions** (`crates/emukc_bootstrap/src/map_pipeline/sources.rs`):
- `stat_catalog: Option<MapCatalog>` in `ResolvedMapSources` (None when unavailable)
- `stat_map_count: usize` in `MapCatalogBuildReport`
- Download logic using `emukc_network` HTTP client

**Build report additions** (`crates/emukc_bootstrap/src/map_pipeline/report.rs`):
- `stat_map_count: usize` — number of maps with stat data
- `stat_source: MapCatalogStatSource` enum — `Downloaded`, `Cached`, `Unavailable`

### D4: stat.json label matching with explicit failure modes

**Decision**: stat.json keys cells by letter label (A, B, C...). Matching to wikiwiki's cell_no requires joining on `node_label`. The `semantic_cell_no_map()` logic in `merge.rs:155` already handles unique-label remapping. Failure modes:

| Condition | Action |
|-----------|--------|
| Cell has no `node_label` | Skip stat data for that cell. No warning (common for start cells). |
| `node_label` is duplicated in variant | Skip stat data for both cells. Warn once. |
| Label exists in stat.json but not in variant | Skip. No warning (stat may cover maps/variants not in catalog). |
| Label matches uniquely | Apply stat `event_id`/`event_kind`. |

**Rationale**: Silent skip with warning on duplicates prevents stat data from being misapplied to the wrong cell. The unique-label constraint matches the existing `semantic_cell_no_map()` logic.

**Implementation**: Parse stat.json into a flat `MapCatalog` where each cell has `node_label = Some("A")` and `event_id`/`event_kind` from stat. The existing `remap_variant_to_definition_identity()` + `merge_cells()` pipeline handles the label→cell_no remap and merge. No new matching code needed — reuse existing remap logic.

### D5: Color-to-event inference in overlay build (fallback only)

**Decision**: Add `fn infer_event_from_color(color_no: i64) -> (i64, i64)` in `crates/emukc_bootstrap/src/map_overlay/merge.rs`. Mapping: `0→(0,0), 2→(2,0), 3→(3,0), 4→(4,1), 5→(5,1), n≥6→(n,1)`.

This runs during overlay BUILD (not during assembly merge). The overlay asset will contain inferred `event_id`/`event_kind` values alongside `color_no`. During assembly, stat.json's authoritative values will overwrite these inferred values via the "last non-zero wins" rule.

**Rationale**: Overlay captures don't include `event_id`/`event_kind` directly (the KC API `api_cell_data` only provides `api_color_no`). Inference provides reasonable defaults that stat.json can override.

## Risks / Trade-offs

- **[Network dependency]** stat.json download from GitHub adds network dependency to bootstrap. → Mitigation: Cache at `.data/stat.json`; skip gracefully if unavailable; build report records source status.
- **[Label mismatch]** stat.json letter labels may not match wikiwiki's `node_label` naming. → Mitigation: Duplicate-label detection skips those cells; label-free cells skip silently. Worst case: stat data doesn't apply, overlay inference acts as fallback.
- **[Merge test update]** Existing `merge_variant_definition_remaps_secondary_cells_by_node_label` test expects fill-missing behavior for `color_no`. → Mitigation: Update test to verify "last non-zero wins" behavior.
- **[Overlay coverage]** Overlay only covers maps the user has visited. → Mitigation: stat.json covers all regular maps; overlay supplements with color/boss/distance.
- **[Static asset staleness]** overlay asset must be manually regenerated after code changes. → Mitigation: Explicit task in workflow; CI could check asset freshness.
- **[kc_data removal]** Removing kc_data means wikiwiki becomes sole provider of route topology. → Mitigation: wikiwiki already provides richer data than kc_data (routing rules with predicates vs simple edges, enemy compositions vs empty, ship drops vs empty). If wikiwiki is unavailable, `ensure_synthetic_variants()` provides minimal fallback.
