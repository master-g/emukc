---
title: "Map data authority: source merge order, stat.json integration, and overlay boss cell capture"
date: 2026-06-22
category: architecture-patterns
module: emukc_bootstrap
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Modifying the map catalog assembly pipeline or its source merge order"
  - "Integrating or regenerating kcs2-mapdata stat.json"
  - "Regenerating public_map_catalog_overlays.json after capture/merge code changes"
tags: [map-data, authority-merge, stat-json, overlay, boss-cell, mapcatalog]
related_components: [emukc_model, emukc_cache]
---

# Map data authority: source merge order, stat.json integration, and overlay boss cell capture

## Context

The map catalog is assembled from multiple sources with a defined authority
order. This contract governs the "last non-zero wins" merge for cell metadata,
the integration of `stat.json` from kcs2-mapdata as the highest-authority
source, label-based matching with failure handling, and the overlay capture of
`boss_cell_no`. Migrated from `openspec/specs/map-data-authority/spec.md`.

## Guidance

### Field-authority merge for cell metadata

The assembly pipeline SHALL use "last non-zero wins" merge for cell metadata
fields (`color_no`, `event_id`, `event_kind`) and variant-level
`boss_cell_no`. Routing fields (`next_cells`, `node_label`, `routing_rules`,
`enemy_fleets`, `ship_drops`) SHALL continue using fill-missing semantics.

- A later non-zero metadata value overwrites the current value regardless of
  what earlier sources provided.
- Empty arrays do NOT overwrite routing fields (fill-missing semantics).

### Bootstrap integrates kcs2-mapdata stat.json

The bootstrap pipeline SHALL download and integrate `stat.json` from
kcs2-mapdata as a data source for cell type metadata. `stat.json` SHALL merge
LAST in the assembly order (after overlay) so its `event_id`/`event_kind`
values are highest authority.

- With network access and no cache: download from
  `https://raw.githubusercontent.com/KagamiChan/kcs2-mapdata/master/maps/stat.json`,
  cache at `.data/stat.json`.
- Without network but with `.data/stat.json`: use cache; build report records
  `stat_source: Cached`.
- No network and no cache: continue without stat data; build report records
  `stat_source: Unavailable`; event types are inferred from overlay `color_no`
  as fallback.
- Merge order: wikiwiki → overlay → stat; `stat.json`'s
  `event_id`/`event_kind` overwrite overlay's inferred values for matched
  cells.
- Build report includes `stat_map_count` (maps with stat data) and
  `stat_source` (Downloaded/Cached/Unavailable).

### stat.json label matching with failure-mode handling

`stat.json` cells are keyed by letter label (A, B, C…). Matching to
wikiwiki's `cell_no` SHALL use existing `node_label`-based remap logic.

- Unique label match: stat data applies to the single cell with that
  `node_label`.
- Duplicate label: stat data is NOT applied to either cell (ambiguous); a
  warning is logged indicating the duplicate label.
- Missing label (`node_label = None`): stat data is not applied (no warning —
  common for start/unnamed nodes).
- Label in stat but not in variant: skipped (no warning — stat may cover
  maps/variants not in catalog).

### Overlay captures boss_cell_no

The public overlay capture process SHALL extract `boss_cell_no` from real KC
API data. The overlay asset SHALL be regenerated after capture code changes.

- A map start API response with `api_bosscell_no` is recorded into the
  overlay.
- Overlay `boss_cell_no` (non-zero) overrides wikiwiki during assembly (overlay
  merges after wikiwiki, non-zero wins).
- Overlay `boss_cell_no = 0` (e.g., response-saver format lacks the field) does
  NOT overwrite wikiwiki's value.
- When `capture.rs` or `merge.rs` are modified to add `boss_cell_no` or event
  inference, `cargo run -- wikiwiki-map build-overlays` MUST be run to
  regenerate `crates/emukc_bootstrap/assets/public_map_catalog_overlays.json`,
  which is then committed.

## Why This Matters

Wrong merge authority produces cells that misidentify battle nodes as safe (or
vice versa), which cascades into broken routing and encounters on the client.
`stat.json` is the ground truth for cell types; treating it as highest
authority and overlay as second-highest keeps inference errors from
propagating. The regenerate-and-commit step is what keeps the committed overlay
asset from silently drifting from the code that produced it.

## When to Apply

- When modifying the assembly pipeline's merge order or adding a new source.
- When regenerating stat data or the overlay asset.
- When touching `capture.rs` / `merge.rs`.

## Examples

- wikiwiki `color_no = 4` + overlay `color_no = 5` → assembled `color_no = 5`.
- wikiwiki `next_cells = [2,3]` + later source `next_cells = []` → stays
  `[2,3]` (routing uses fill-missing).
- Duplicate `node_label = "A"` on two cells → stat data for "A" is skipped and
  a warning is logged.

## Related

- `docs/solutions/architecture-patterns/sortie.md` — cell metadata correctness
  consumed by the sortie loop.
- `docs/solutions/best-practices/web-asset-bootstrap.md` — bootstrap pipeline.
