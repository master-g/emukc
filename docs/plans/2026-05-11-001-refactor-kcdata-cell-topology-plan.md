---
title: "Refactor kcdata parser: one cell per unique node"
type: refactor
status: active
date: 2026-05-11
---

# Refactor kcdata parser: one cell per unique node

## Summary

Rewrite `build_variant_from_kcdata` to produce one `MapCellDefinition` per unique graph node instead of one per route. Currently 129/130 maps have duplicate node labels because multiple routes converge on the same target node. The dedup logic in `label_to_cell_no()` excludes these labels, silently dropping enemy fleet, ship drop, and routing rule overlays for affected nodes.

---

## Problem Frame

kcdata YAML models maps as a list of directed edges (routes), each with a `from` node and a `to` node. Nodes are referenced by label (e.g. `A`, `E`, `J`). The parser currently creates one cell per route, using `route_id` as `cell_no`. When two routes converge on node `E` (e.g. `A→E` via route 5, `B→E` via route 11), two cells are created, both labeled `E` but with different `cell_no` values.

`label_to_cell_no()` detects this and excludes `E` from the label index. The downstream overlay merge cannot find `E` and drops all enemy fleet, routing, and drop data for that node. In map 1-3, this causes cells E, F, and J (boss) to have no enemy fleets.

129 of 130 kcdata maps have at least one convergent node. Map 1-1 is the only one with a purely linear graph.

---

## Requirements

- R1. Each unique graph node in a kcdata YAML produces exactly one `MapCellDefinition`.
- R2. `cell_no` values are sequential starting from 0 (Start = 0), matching the real game's `api_no` convention.
- R3. `next_cells` references cell_nos of unique nodes (not route IDs).
- R4. `boss_cell_no` is the cell_no of the boss node (not a route ID).
- R5. No duplicate `node_label` values in the produced variant's cells.
- R6. The verify.rs topology test passes without skipping cell-count mismatches for maps with real-game captures.
- R7. Existing overlay merge paths (label_overlay, legacy routing overlay) work unchanged because they receive a clean label→cell_no map.
- R8. The 6 existing unit tests in kcdata.rs pass (after updating assertions to match the new topology model).

---

## Scope Boundaries

- **In scope**: kcdata parser (`build_variant_from_kcdata`), its tests, and the verify.rs topology test.
- **Out of scope**: wikiwiki parser, overlay merge logic, `MapCellDefinition` / `MapVariantDefinition` struct definitions, runtime sortie code, stat.json parsing. These consumers work correctly once they receive clean input.

### Deferred to Follow-Up Work

- Re-bootstrap `.data/codex/map_catalog.json` after this fix lands and verify all maps produce correct enemy fleet assignments.
- Verify that the real-game captures in `real_map_start_data/` now match topology for all maps (currently skipped for cell-count mismatch).

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` — parser to rewrite. `build_variant_from_kcdata` at line 121 is the core function. 6 existing tests validate route-based cell creation.
- `crates/emukc_model/src/codex/map/types.rs:109` — `label_to_cell_no()` excludes duplicate labels. No changes needed here; this function works correctly when given non-duplicate input.
- `crates/emukc_model/src/codex/map/merge.rs:259` — `build_cell_no_map()` uses `unique_labeled_cells()` with same dedup logic. No changes needed.
- `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` — orchestrates source assembly. No structural changes, but the kcdata variant it receives will have correct topology.
- `crates/emukc_bootstrap/src/map_pipeline/verify.rs` — topology verification test that currently skips cell-count mismatches.

### Audit Findings

- 129/130 maps have convergent routes producing duplicate labels.
- 1,201 total excess route-target entries across affected maps.
- Worst offender: map 545 with 26 excess entries and 23 duplicated labels.
- Map 1-3 (the triggering case): E, F, J duplicated; enemy fleets missing at cells 5, 6, 10.

---

## Key Technical Decisions

- **KD1. Cell numbering: sequential from 0.** The real game uses `api_no` starting at 0 for Start. The new parser assigns 0 to Start, then increments for each unique node discovered in route order. Rationale: matches real-game convention, keeps label-based mapping stable, avoids arbitrary numbering schemes.
- **KD2. Node discovery: iterate routes, deduplicate by label.** Walk routes in ID order. For each route's target, check if the label is already assigned a cell_no. If not, assign the next sequential number. Rationale: preserves route-order determinism, handles the `from: null` Start route naturally.
- **KD3. Edge construction: per-node outgoing edges.** After all nodes are numbered, iterate routes again to build `next_cells`: for each route, add the target's cell_no to the source's `next_cells`. Deduplicate in case of parallel edges. Rationale: clean separation of node discovery and edge construction.

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification.*

```
Input: KcDataMapData { routes: {id: {from, to}}, cells: {label: meta} }

Phase 1 — Discover unique nodes:
  label_to_cell_no = {}
  cell_no_counter = 0

  for route in routes_sorted_by_id:
    if route.from is None:   // Start route
      if "Start" not in label_to_cell_no:
        label_to_cell_no["Start"] = 0
        cell_no_counter = 1
    target = route.to
    if target not in label_to_cell_no:
      label_to_cell_no[target] = cell_no_counter
      cell_no_counter += 1

Phase 2 — Build cells:
  for (label, cell_no) in label_to_cell_no:
    meta = cells.get(label)
    cells.push(MapCellDefinition { cell_no, node_label: label, ... })

Phase 3 — Build edges:
  for route in routes:
    source_label = route.from or "Start"
    target_cell_no = label_to_cell_no[route.to]
    source_cell = cells[label_to_cell_no[source_label]]
    if target_cell_no not in source_cell.next_cells:
      source_cell.next_cells.push(target_cell_no)
```

---

## Implementation Units

### U1. Rewrite `build_variant_from_kcdata`

**Goal:** Produce one cell per unique graph node with sequential cell numbering and correct edges.

**Requirements:** R1, R2, R3, R4, R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- Test: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (inline tests)

**Approach:**
- Replace the current route-iteration loop with the two-phase design above: (1) discover unique nodes and assign sequential cell_nos, (2) build cells, (3) build edges.
- The `routes_from_node` helper changes from mapping node→route_ids to mapping node→target_cell_nos (via the label_to_cell_no index).
- `boss_cell_no` is set to the cell_no of the node whose metadata has `boss: true`.
- Remove the `Vec::with_capacity(data.routes.len())` allocation since cell count now equals unique node count, not route count.

**Patterns to follow:**
- Existing test structure in `kcdata.rs` uses inline `#[test]` functions with YAML literals. Keep this style.

**Test scenarios:**
- Happy path: linear map (1-1 style, no convergent routes) produces same cell count as route count, correct next_cells.
- Convergent routes (1-3 style): routes 5 and 11 both target E → one cell for E, both source cells have E's cell_no in next_cells.
- Multiple bosses: only the last boss cell is recorded (matches current behavior).
- No boss routes: boss_cell_no stays 0 (matches current behavior).
- Start node: from=null route creates cell_no=0 with label "Start".
- Numeric node labels: routes targeting integer nodes (e.g. `"1"`) are handled correctly.
- Full repo validation: all 130 kcdata YAML files produce variants where labels are unique and no cell count exceeds unique node count.

**Verification:**
- `cargo test -p emukc_bootstrap` passes, including the full-repo validation test.

---

### U2. Update verify.rs topology test

**Goal:** Remove the cell-count-mismatch skip so that real-game captures are validated against the corrected topology.

**Requirements:** R6

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/verify.rs`

**Approach:**
- Remove or relax the `if variant.cells.len() != real_cells.len() { continue; }` guard.
- With the fix, kcdata-derived cell counts should match real-game `api_cell_data` counts for all maps with captures.
- If any maps still mismatch after the fix, add those maps to an explicit skip list with a comment explaining why (e.g. multi-gauge maps where the capture is from a different phase).

**Test scenarios:**
- Map 1-3: cell count matches real-game capture (currently 14 vs 10 real cells → should now both be 14 or match after fix).
- All embedded real_map_start_data assets: no unexpected cell-count mismatches.

**Verification:**
- `cargo test -p emukc_bootstrap` topology verification passes.

---

### U3. Re-bootstrap and validate map 1-3 enemy fleet coverage

**Goal:** Confirm that the fix resolves the original symptom — missing enemy fleets at F, E, J in map 1-3.

**Requirements:** R5, R7

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (add integration test)
- Test: `tests/gameplay_tests/` (add map 1-3 enemy fleet coverage test if not already present)

**Approach:**
- Add a test that runs the full pipeline (kcdata + wikiwiki overlay) for map 1-3 and asserts that cells C, E, F, and J all have non-empty enemy fleets.
- This test can be an inline test in `kcdata.rs` or in `assemble.rs` using the assembled catalog.
- After re-bootstrapping, manually verify `.data/codex/map_catalog.json` has enemy fleets at the correct cells for map 1-3.

**Test scenarios:**
- Map 1-3 assembled catalog: cells labeled C, E, F, J each have at least one enemy composition.
- Map 1-3 assembled catalog: no cell has enemy fleet data from a wrong label (e.g. cell B should not have E's compositions).

**Verification:**
- `cargo test -p emukc_bootstrap` passes including the new integration test.
- Manual: `cargo run -- bootstrap` produces updated `.data/codex/map_catalog.json` with correct 1-3 data.

---

## System-Wide Impact

- **Interaction graph:** The kcdata parser output feeds into `assemble_final_map_catalog`, which produces the `MapCatalog` loaded by the `Codex` at runtime. All gameplay code that reads map topology, enemy fleets, and routing rules is downstream of this change.
- **Unchanged invariants:** `MapCellDefinition` and `MapVariantDefinition` structs are not modified. The overlay merge functions (`merge_label_overlay`, `merge_routing_overlay`) receive cleaner input but their logic is unchanged. The `label_to_cell_no()` dedup logic still exists but will never trigger for kcdata-derived topologies (no duplicates).

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Cell_no values change for all 129 maps, breaking any code that hardcodes route IDs | Grep shows cell_no is always looked up dynamically (by label, by position, or by BTreeMap key). No hardcoding found. |
| Real-game captures may have different cell counts for multi-gauge maps | verify.rs already skips these; keep the skip for genuine multi-gauge mismatches. |
| stat.json or public overlay catalogs reference old route-ID cell_nos | `merge_missing_from` fills gaps by cell_no. After re-bootstrap, the primary kcdata source provides correct cell_nos. Overlays keyed by cell_no may need updating if they reference route IDs. |
| Existing gameplay tests depend on current cell_no numbering | Tests use in-memory DB and loaded Codex. After re-bootstrap, Codex has new numbering. Tests should pass if they don't hardcode cell_nos. |
