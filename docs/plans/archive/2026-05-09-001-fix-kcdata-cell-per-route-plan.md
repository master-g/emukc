---
title: "fix: kcdata cell-per-route topology"
type: fix
status: active
date: 2026-05-09
---

# fix: kcdata cell-per-route topology

## Summary

Rewrite `build_variant_from_kcdata` to create one cell per route/edge (where route_id = game cell_no) instead of one cell per named node. The game assigns unique cell numbers to each edge, so when multiple routes target the same node, there are multiple game cells. The current parser collapses these into single cells, causing 54 "unreachable" warnings from overlay-merged cells and incorrect cell numbering throughout the catalog.

---

## Problem Frame

The kcdata YAML format defines map topology with `routes` (edges, keyed by integer ID) and `cells` (node metadata, keyed by label like "A", "B"). Through investigation of real game API responses (`api_req_map/start`), we confirmed that **route ID = game cell_no**. Each route is a distinct game cell, not each named node.

For example, map 2-3 has:
- 14 named cells (A-N) in kcdata
- 21 routes (0-20) in kcdata
- 21 cells (0-20) in the game API

Routes 4 (A→D) and 15 (C→D) both end at node D, but in the game they are **separate cells** (cell 4 and cell 15). The current parser creates one cell for D, losing this per-route identity. The overlay then adds the "missing" cells, which appear unreachable because the kcdata-derived topology has no edges to them.

---

## Requirements

- R1. `build_variant_from_kcdata` must produce one `MapCellDefinition` per route, where `cell_no = route_id`
- R2. Routes with `from: null` are start cells; each gets `cell_no = route_id` and `next_cells` = IDs of routes whose `from` matches this route's `to` node
- R3. Each route-cell's `next_cells` are the IDs of routes whose `from` node matches this route's `to` node
- R4. Cells that correspond to the same named node share that node's metadata (boss flag, name, color)
- R5. `boss_cell_no` is the cell_no of the first route whose `to` node is a boss cell
- R6. All existing overlay merge, wikiwiki merge, and API response generation continue to work correctly
- R7. The 54 "Unreachable" warnings at codex load time are eliminated

---

## Scope Boundaries

- Dead code cleanup (unused imports/functions from recent refactoring) is out of scope
- Wikiwiki catalog parser changes are out of scope (it already has correct cell counts)
- Overlay merge logic changes are out of scope (it works correctly; the fix is upstream in kcdata parsing)
- `validate()` warning level changes are out of scope (warnings will disappear naturally)
- `cargo run -- wikiwiki-map normalize` is out of scope — the wikiwiki parser uses BFS-based cell_nos, not game cell_nos. Running normalize after this fix would corrupt the stored wikiwiki_map_catalog.json for shared-label nodes. Do not run normalize until the wikiwiki overlay path is updated to handle duplicate labels.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` — `build_variant_from_kcdata` (line 116-226) and helpers
- `crates/emukc_model/src/codex/map/merge.rs` — `merge_cells` (line 330-366) matches overlay by `cell_no`; `semantic_cell_no_map` (line 177-199) remaps by label
- `crates/emukc_bootstrap/src/map_overlay/matching.rs` — `choose_stage_match` (line 7-84) matches captures by cell_no set equality
- `crates/emukc_bootstrap/src/map_pipeline/verify.rs` — `battle_cells_have_enemy_fleet_data` (line 117-158) checks cells by `api_no`
- `crates/emukc_gameplay/src/game/sortie/mod.rs` — `build_sortie_cell_data` (line 1142-1154) projects cells to API response
- `crates/emukc_bootstrap/assets/real_map_start_data/map_*.json` — ground truth for cell counts
- `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` — `resolve_kcdata_targets` (line ~274) resolves route target nodes to cell numbers via BFS-assigned ordering; becomes unnecessary under route-based cell_no assignment

### Key Insight: Route ID = Game Cell_no

Verified across all 14 regular maps (12-35) by comparing route IDs with real game `api_cell_data` arrays:

| Map | Routes | Named cells | Game cells | Extra (routes > named cells) |
|-----|--------|-------------|------------|------------------------------|
| 12  | 7      | 5           | 7          | +2 (same node D/E via diff edges) |
| 23  | 21     | 14          | 21         | +7 (nodes D,F,G,K,N reached via multiple edges) |
| 34  | 25     | 16          | 25         | +9 |

The "extra" cells are **not orphans** — they are the same node reached via a different edge. The game tracks which edge the fleet traversed.

---

## Key Technical Decisions

- **Cell_no = route ID**: Direct mapping from kcdata route key to `MapCellDefinition.cell_no`. No BFS-based assignment needed.
- **Null-from routes are start cells**: Any route with `from: null` is a start cell. No synthetic start cell needed. For most maps, only route 0 has `from: null`. For event maps with multiple gauges/phases (61 of 130 maps), multiple routes have `from: null` — each is an independent start cell with its own `cell_no`. No conflict: they represent different phase entry points.
- **Node metadata shared by route cells**: When routes 4 and 15 both target node D, both cells get D's `name`, `boss` flag, and derived `color_no`/`event_id`/`event_kind`. Shared labels are acceptable because wikiwiki overlay uses identity cell_no mapping (both sides already have matching game cell_nos), not label-based remap.
- **Overlay merge works by cell_no alignment**: After the fix, kcdata cell_nos = game cell_nos = wikiwiki cell_nos. The public overlay merge (`merge_cells`, cell_no-based) aligns directly. The wikiwiki overlay merge uses `remap_cell_no` which falls back to identity (`unwrap_or(cell_no)`) — since both sides use game cell_nos, identity is the correct mapping. The label-based `build_cell_no_map` / `unique_labeled_cells` path may evict shared labels, but this is harmless because the identity fallback produces correct results.

---

## Open Questions

### Resolved During Planning

- **Q: Will overlay merge break?** A: No. Overlay uses `cell_no` matching. After fix, kcdata cell_nos = game cell_nos = overlay cell_nos. The `semantic_cell_no_map` remap (label-based) becomes a fallback for edge cases.
- **Q: Will wikiwiki merge break?** A: No. The stored `wikiwiki_map_catalog.json` (full pipeline output, loaded at runtime) already uses game cell_nos — verified for map 23 (both 0-20). The wikiwiki HTML parser itself uses BFS-based cell_nos, but the runtime path loads the pre-built JSON, not re-parsed HTML. After the fix, kcdata cell_nos will also match game cell_nos. The `remap_cell_no` identity fallback (`unwrap_or(cell_no)`) produces correct mappings. Shared labels from multi-route nodes are harmless — `unique_labeled_cells` may evict them, but identity mapping is correct since cell_nos align.
- **Q: Will the matching logic in `map_overlay/matching.rs` work?** A: Yes — it will work *better*. Currently it falls through to superset/subset matching because kcdata cell_nos don't match game cell_nos. After fix, exact match should succeed for single-variant maps.

### Resolved After Review

- **Exact `node_label` for route-cells**: Use the node label (e.g., "D") for all routes targeting that node. No disambiguator needed. Shared labels are acceptable because wikiwiki overlay uses cell_no identity mapping, not label-based lookup.
- **Handling of route 0**: Route 0 IS the start cell (no synthetic needed). More generally, any route with `from: null` is a start cell. For event maps with multiple gauges, multiple null-from routes exist — each is an independent start cell at its own cell_no. No cell_no=0 conflict because route 0 is one of potentially several start cells.
- **Multi-start maps (61 of 130)**: Each null-from route is an independent start cell. Only the route 0 start is used for the default gauge. Other null-from routes have no incoming edges and may appear "unreachable" in the default gauge — this is correct behavior since they belong to other gauge/phase variants.

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

### Current flow (node-based)

```
kcdata YAML → nodes = cells.keys() (A-N)
            → assign cell_no by BFS order (1,2,3,...)
            → build cells: one per node, next_cells from route graph
            → result: N+1 cells (0=Start, 1..N=nodes)
```

### New flow (route-based)

```
kcdata YAML → for each route (key=ID):
                cell_no = route_key
                target_node = route.to
                label = target_node (if in cells map) or ""
                next_cells = IDs of routes whose from == target_node
              → null-from routes are start cells (no synthetic needed)
              → result: R cells (one per route, including start routes)
```

### Edge construction

For route 4 (A→D):
- `cell_no` = 4
- `node_label` = "D" (target node's key)
- `next_cells` = [IDs of routes with from=D] = [7 (D→G), 16 (D→F)]

For route 0 (null→1):
- This IS the start cell
- `cell_no` = 0
- `node_label` = "Start"
- `next_cells` = [IDs of routes with from=1] = [1, 2, 3]

For multi-start maps (e.g., route 15 also has from:null):
- `cell_no` = 15 (its own route_id)
- `node_label` = "Start"
- `next_cells` = [IDs of routes from its target node]
- No incoming edges from other cells — reachable only in its specific gauge/phase

---

## Implementation Units

### U1. Rewrite `build_variant_from_kcdata`

**Goal:** Change from node-based to route-based cell creation. Each kcdata route becomes a cell with `cell_no = route_id`.

**Requirements:** R1, R2, R3, R4, R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`

**Approach:**
- Replace the current node-iteration loop with a route-iteration loop
- Null-from routes are start cells (cell_no = route_id, node_label = "Start"). No synthetic start cell needed
- For multi-start maps (61 of 130), each null-from route is an independent start cell
- For each route, look up the target node's metadata from `data.cells`
- Build `next_cells` by finding all routes whose `from` matches this route's `to`
- Remove `ordered_kcdata_nodes`, `resolve_kcdata_targets` (resolves route targets to BFS-assigned cell numbers), `assigned_numbers`/`used_numbers` machinery (no longer needed for cell assignment)
- `boss_cell_no` = first route_id whose `to` node has `boss: true`
- Keep `collect_kcdata_nodes` and `route_node_key` if still needed for graph traversal

**Patterns to follow:**
- Existing `MapCellDefinition` construction pattern (lines 169-212)
- Existing `KcDataMapData`/`KcDataRoute`/`KcDataCell` struct access

**Test scenarios:**
- Happy path: map 12 YAML (7 routes, 5 cells) produces 7 cells with correct cell_nos (0-6), correct next_cells, boss at cell 5
- Happy path: map 23 YAML (21 routes, 14 cells) produces 21 cells with correct cell_nos (0-20), correct next_cells, boss at cell 14
- Edge case: map with single null-route produces 1 start cell with correct next_cells
- Edge case: node with no metadata in `data.cells` (intermediate node like "1") gets default metadata (color_no=0, no boss)
- Edge case: multiple routes targeting same boss node — all get boss color/event
- Edge case: numeric node keys in `data.cells` preserved as-is (not remapped)
- Edge case: multi-start map (e.g., map 421) with multiple null-from routes — each gets its own cell_no, both are start cells

**Verification:**
- All 4 existing kcdata tests pass (with updated assertions for cell counts)
- `cargo run` produces 0 Unreachable warnings

### U2. Update kcdata tests for route-based cell model

**Goal:** Update existing tests to expect route-count cells instead of node-count cells.

**Requirements:** R1, R7

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (test module)

**Approach:**
- `build_variant_from_kcdata_skips_route_only_numeric_placeholders`: Rewrite to test route-based cell model. Expected cells = routes count (one cell per route, null-from routes included). Verify cell_nos match route IDs.
- `build_variant_from_all_repo_kcdata_maps_keeps_real_cell_count_and_valid_edges`: Change assertion from `data.cells.len() + 1` to `data.routes.len()`. No +1 — null-from routes are counted among the routes. Verify all next_cells are valid route IDs.
- `build_variant_from_all_repo_kcdata_maps_preserves_real_numeric_cell_keys`: Adjust to verify route-based numbering. Numeric keys in cells map are no longer directly used as cell_nos (route IDs are). May need to verify that cells with numeric keys get correct metadata when targeted by routes.
- Add new test: verify that route 4 (A→D) and route 15 (C→D) in map 23 both produce cells with label "D" and correct next_cells.

**Test scenarios:**
- Happy path: updated assertions match new cell count model
- Edge case: empty routes map produces only start cell
- Edge case: map with numeric cell keys still resolves correctly

**Verification:**
- `cargo test -p emukc_bootstrap --lib -- map_pipeline::kcdata` — all tests green

### U3. Verify overlay and merge compatibility

**Goal:** Confirm that the overlay merge, wikiwiki merge, matching, and verify.rs all work with route-based cell_nos.

**Requirements:** R6, R7

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/verify.rs` (if test assertions need updating)
- Verify (no changes expected): `crates/emukc_model/src/codex/map/merge.rs`, `crates/emukc_bootstrap/src/map_overlay/matching.rs`, `crates/emukc_bootstrap/src/map_overlay/merge.rs`, `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`

**Approach:**
- Run all overlay tests (`cargo test -p emukc_bootstrap --lib -- map_overlay`)
- Run all merge tests (`cargo test -p emukc_model --lib -- map::merge`)
- Run `battle_cells_have_enemy_fleet_data` test — should pass now that kcdata cell_nos match game cell_nos
- Run `cargo run` and verify 0 Unreachable warnings
- If `battle_cells_have_enemy_fleet_data` fails, investigate whether enemy fleet data is keyed by the old cell_nos (node-based) and needs re-keying

**Test scenarios:**
- Integration: overlay merge enriches route-based cells with correct master_cell_ids
- Integration: wikiwiki routing rules remap correctly to route-based cell_nos
- Integration: matching.rs finds exact match for single-variant maps (was falling through to superset before)
- Integration: `cargo run` produces 0 Unreachable warnings, 0 routing rule drops

**Verification:**
- `cargo test -p emukc_bootstrap --lib` passes (minus pre-existing failures)
- `cargo test -p emukc_model --lib` passes
- `cargo run` clean startup (no Unreachable warnings)

### U4. Verify gameplay and API response fidelity

**Goal:** Ensure sortie gameplay and API responses work correctly with route-based cell_nos.

**Requirements:** R6

**Dependencies:** U1

**Files:**
- Verify (no changes expected): `crates/emukc_gameplay/src/game/sortie/mod.rs`
- Verify: `src/bin/net/router/kcsapi/api_req_map/projection.rs`

**Approach:**
- Run `cargo test --test gameplay_tests`
- Check `build_sortie_cell_data` fallback: `map_id * 100 + cell.cell_no` — with route-based cell_nos, the fallback values change. Verify overlay provides master_cell_ids for all cells so fallback is never used in practice.
- If gameplay tests fail, trace the failure to determine if it's cell_no related or pre-existing

**Test scenarios:**
- Integration: `cargo test --test gameplay_tests` passes (minus pre-existing failures)
- Integration: sortie start generates `api_cell_data` with correct cell_nos matching real game data

**Verification:**
- `cargo test --test gameplay_tests` — no new failures
- Manual: `cargo run -- new-session` + start a sortie, verify cell data matches expected game format

---

## System-Wide Impact

- **Interaction graph:** `build_variant_from_kcdata` output feeds into overlay merge, wikiwiki merge, verify tests, gameplay sortie, and API response projection. All consumers use `cell_no` as the primary key.
- **Error propagation:** Incorrect cell_no assignment would cascade to wrong enemy fleet lookups, wrong routing, wrong API responses. Tests at each layer catch this.
- **State lifecycle risks:** The codex JSON on disk stores cell_nos. After this fix, a re-bootstrap is needed to regenerate `map_catalog.json` with correct numbering. Old catalog data is stale.
- **API surface parity:** `api_cell_data` in sortie responses must match real game data. Route-based cell_nos align with game API directly.
- **Unchanged invariants:** `MapCellDefinition` struct is unchanged. The overlay merge, wikiwiki merge, and routing rule systems are unchanged. Only the kcdata parser's cell_no assignment strategy changes.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Cell_no conflict between route 0 and synthetic start | No synthetic start cell. Null-from routes ARE start cells. No conflict. |
| Maps with no kcdata data break | `ensure_synthetic_variants()` creates synthetic start cells for empty maps. Unaffected. |
| Enemy fleet data keyed by old cell_nos | Enemy fleets come from wikiwiki overlay. Since wikiwiki cell_nos match game cell_nos, and kcdata cell_nos will match game cell_nos after fix, identity mapping is correct. |
| Multi-start maps (61 of 130) | Each null-from route is an independent start cell. Only route 0 is used for default gauge; other null-from routes have no incoming edges in the default gauge — correct behavior. |
| Existing `map_catalog.json` on disk is stale | Re-bootstrap required after fix. Document this. |
| `verify.rs` test fails because no enemy fleet data for new cell_nos | Enemy fleets are remapped via label. If a cell has label "D", fleet data follows. Verify with test run. |

---

## Sources & References

- kcdata parser: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- Map merge: `crates/emukc_model/src/codex/map/merge.rs`
- Real game data: `crates/emukc_bootstrap/assets/real_map_start_data/map_*.json`
- kcdata YAML files: `.data/temp/kc_data/_map/*.json`
- Overlay: `crates/emukc_bootstrap/src/map_overlay/`
- Previous map topology plan: `docs/plans/2026-05-05-003-refactor-map-topology-routing-separation-plan.md`
