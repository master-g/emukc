---
title: "refactor: Separate map graph topology from routing rules — kcdata for topology, WikiWiki for rules only"
type: refactor
status: active
date: 2026-05-05
---

# Refactor: Separate Map Graph Topology from Routing Rules

## Summary

Refactor the map data pipeline so kcdata YAML is the sole source of directed graph topology (vertices + edges), and WikiWiki HTML provides only routing predicates (conditions, probabilities). Remove WikiWiki's BFS-based graph inference heuristic, refactor `semantic_cell_no_map` to accept a generic label map instead of consuming `MapCellDefinition` structs, and create a new `merge_routing_overlay` merge path that never touches cells or next_cells.

---

## Problem Frame

The WikiWiki HTML parser infers graph topology using a BFS heuristic ("target-only node" heuristic at `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs:414-427`). This heuristic produces spurious edges, misses intermediate nodes, and creates wrong edge targets. Audit against authoritative directed graphs for maps 1-1, 1-2, 1-3 confirmed: kcdata YAML has accurate topology for all three tested maps while WikiWiki has structural errors in all three. Full validation of all 130 kcdata maps is deferred (see Deferred to Follow-Up Work).

kcdata covers all 130 maps (regular + event). WikiWiki covers only 41. The kcdata parser (`crates/emukc_bootstrap/src/map_pipeline/kcdata.rs:417-527`) uses explicit route definitions with no inference — it should be the sole topology source.

---

## Requirements

- R1. kcdata YAML provides all vertices and edges (`cells`, `next_cells`) — no other source contributes to graph topology
- R2. WikiWiki HTML provides only routing rules (`RouteRule` with `RoutePredicate`), enemy fleets, and ship drops
- R3. WikiWiki parser's BFS heuristic and "target-only node" inference are removed
- R4. Merge logic (`merge.rs`) no longer merges WikiWiki `cells`/`next_cells` into kcdata topology
- R5. Semantic label mapping is refactored to accept a generic `BTreeMap<String, i64>` instead of `&[MapCellDefinition]`
- R6. All existing tests continue to pass

---

## Scope Boundaries

- **Out of scope:** Real-game data (`real_map_start_data`) integration — acquisition is difficult and not needed since kcdata covers everything
- **Out of scope:** Routing rule accuracy audit beyond what the previous brainstorm covered
- **Out of scope:** Changes to runtime route evaluation (`map_route.rs`)
- **Out of scope:** WikiWiki-only fallback when kcdata is absent — kcdata is now required for map topology

### Deferred to Follow-Up Work

- Validate kcdata topology against real_map_start_data vertex counts
- Add graph validation test suite for all maps (not just 1-1/1-2/1-3)
- Clean up `merge_variant_definition`'s `inferred_start` fallback logic (lines 92-107) if it becomes dead code after WikiWiki bypasses this path

---

## Context & Research

### Relevant Code and Patterns

- **kcdata parser**: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` — `build_variant_from_kcdata()` reads explicit `routes` and `cells` from YAML, resolves numeric intermediaries via `resolve_kcdata_targets()`, no inference
- **WikiWiki parser**: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` — `build_nodes()` at lines 340-460 uses BFS + "target-only node" heuristic
- **Merge logic**: `crates/emukc_model/src/codex/map/merge.rs` — `merge_variant_definition()` calls `remap_variant_to_definition_identity()` then `merge_cells()`. The `semantic_cell_no_map()` function maps WikiWiki cell numbers to kcdata cell numbers by matching `node_label`
- **Assembly**: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` — currently clears kcdata routing_rules, then merges wikiwiki (including cells/next_cells)
- **Types**: `crates/emukc_model/src/codex/map/types.rs` — `MapCellDefinition`, `MapVariantDefinition`, `RouteRule`, `RoutePredicate`

---

## Key Technical Decisions

- **TD1. Refactor `semantic_cell_no_map` to accept `BTreeMap<String, i64>`:** Instead of requiring WikiWiki to produce fake `MapCellDefinition` structs (the previous TD1), modify the remap infrastructure to accept a generic label→cell_no map. WikiWiki parser outputs only a flat `BTreeMap<String, i64>` label index instead of `Vec<MapCellDefinition>`. This is structurally cleaner — no "cells that are never really cells" pattern, and no social-contract firewall needed.

- **TD2. New dedicated merge function for WikiWiki routing overlay:** Instead of modifying `merge_variant_definition` (which is also used for public overlays and STAT.json), create a new `merge_routing_overlay` function that remaps routing rules, enemy fleets, and ship drops using the generic label map but does NOT touch cells or next_cells.

- **TD3. Remove "target-only node" heuristic from WikiWiki parser:** The BFS ordering of nodes is kept (for consistent internal numbering), but the inference of `next_cells` for non-branching nodes is removed. WikiWiki output becomes: label map + routing rules + enemy fleets + ship drops. No cells, no next_cells.

- **TD4. Remove routing_rules clearing in assemble.rs:** The existing clearing block (lines 13-20) is a no-op since kcdata has no routing_rules (empty BTreeMap). Removal is a sub-step of U2, listed here for traceability.

---

## Open Questions

### Resolved During Planning

- **kcdata coverage:** Verified — 130 files cover all maps (1-1 to 7-5, plus event maps 421-615). WikiWiki has only 41 maps.
- **WikiWiki-only maps:** 89 maps have only kcdata, no WikiWiki. These are unaffected since there's no WikiWiki data to merge.
- **Real data integration:** Confirmed out of scope per user request.
- **R6 (WikiWiki fallback):** Removed. kcdata is now required for topology. Pipeline fails explicitly if kcdata is absent rather than falling back to WikiWiki's unreliable heuristic.

### Deferred to Implementation

- **Exact WikiWiki parser simplification scope:** How much of `build_nodes()` can be removed vs repurposed. The BFS ordering logic may still be useful for consistent numbering.
- **Enemy fleet/ship drop ownership:** Whether WikiWiki's enemy fleets and ship drops should also be excluded from contributing to the final catalog, or only topology. Currently keeping them as WikiWiki-sourced since kcdata doesn't have enemy composition data.
- **Variant key matching in assembly:** Whether `merge_routing_overlay` needs to replicate the fallback variant logic from `merge_definition` (empty-key variant used as fallback for named variants).

---

## Implementation Units

- U1. **Add `merge_routing_overlay` and refactor `semantic_cell_no_map` in merge.rs**

**Goal:** Create a new merge function and supporting label-map infrastructure that overlays routing rules, enemy fleets, and ship drops from a secondary source onto kcdata's topology, without touching cells or next_cells.

**Requirements:** R2, R4, R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_model/src/codex/map/merge.rs`
- Test: `crates/emukc_model/src/codex/map/merge.rs` (inline tests)

**Approach:**
- Refactor `semantic_cell_no_map` to accept `BTreeMap<String, i64>` (label→cell_no) from the secondary source, instead of consuming `&[MapCellDefinition]`. Keep the existing function for backward compatibility (public overlays, STAT.json still use `merge_variant_definition`), add new overload or rename.
- Create `remap_routing_overlay` function that: (1) builds the semantic mapping using the new label-map API, (2) remaps `from_cell_no`/`to_cell_no` in routing rules, (3) remaps enemy fleet and ship drop cell references, (4) merges into the primary variant using `entry().or_insert_with()` for routing_rules/enemy_fleets/ship_drops, (5) does NOT call `merge_cells`
- Verify that `merge_cells` is never called for WikiWiki data after this change (was U4 — now folded into U1 verification)

**Test scenarios:**
- Happy path: WikiWiki routing rules remapped to kcdata cell numbers via labels, kcdata cells/next_cells unchanged
- Edge case: WikiWiki has a rule referencing a label that doesn't exist in kcdata — rule preserved with original cell_no (no remap)
- Edge case: WikiWiki and kcdata have the same map with different cell numberings — remap produces correct mapping
- Integration: Full merge cycle where kcdata provides 4 cells with edges, WikiWiki provides 6 routing rules via label map, final result has kcdata cells + WikiWiki rules remapped
- Verification (from former U4): kcdata cells with populated next_cells remain unchanged after overlay merge

**Verification:**
- New unit tests pass
- Existing merge tests continue to pass (old `merge_variant_definition` unchanged for overlays/STAT)

---

- U2. **Update assembly logic to use routing overlay**

**Goal:** Change `assemble_final_map_catalog` to use the new `merge_routing_overlay` for WikiWiki data instead of `merge_missing_from`.

**Requirements:** R1, R2, R4

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`

**Approach:**
- Remove the routing_rules clearing block (lines 13-20) — sub-step of this unit (TD4)
- Replace `kcdata.merge_missing_from(wikiwiki)` with new routing-overlay merge that:
  1. Iterates wikiwiki maps
  2. For each map that exists in kcdata: calls `merge_routing_overlay` on matching variants, using WikiWiki's label map for semantic remapping
  3. For maps NOT in kcdata: skip (kcdata is required for topology — no fallback to WikiWiki)
- Keep `merge_missing_from` for public overlays and STAT.json (these don't produce cells with BFS heuristics)
- Handle variant key matching: if kcdata has named variants and WikiWiki has empty-key variant, apply the empty-key as fallback to all named variants (same semantics as current `merge_definition`)

**Test scenarios:**
- Happy path: kcdata has 130 maps, WikiWiki has 41, all 41 get routing rules merged, kcdata topology unchanged
- Edge case: WikiWiki catalog is None — kcdata catalog returned unchanged
- Edge case: kcdata has named variants, WikiWiki has empty-key — routing rules applied to all named variants
- Integration: Full pipeline produces catalog with kcdata graph topology for all 130 maps; maps with WikiWiki data have routing_rules populated (remapped to kcdata numbering); maps without WikiWiki data have empty routing_rules

**Verification:**
- Assembly produces correct catalog
- Topology validation: spot-check that kcdata edges for maps 11/12/13 match authoritative directed graphs (lightweight integration assertion under this unit)

---

- U3. **Simplify WikiWiki parser: remove graph inference, output label map**

**Goal:** Remove the "target-only node" BFS heuristic from the WikiWiki parser. Output a flat label→cell_no map instead of `Vec<MapCellDefinition>`. Keep node labeling for routing rule references.

**Requirements:** R3, R5

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`

**Approach:**
- In `build_nodes()`: remove the "target-only node" inference block (lines 414-427)
- Replace `Vec<MapCellDefinition>` output with `BTreeMap<String, i64>` (label→cell_no mapping)
- Keep BFS ordering logic — it provides consistent numbering for routing rule references
- Remove all code that constructs `next_cells` or `MapCellDefinition` from the parser
- The WikiWiki parser now outputs: label map + routing rules + enemy fleets + ship drops
- Update `into_map_catalog()` to use the new label map structure
- Update `parse_fixture_catalog_preserves_explicit_start_routes` test — remove next_cells assertions

**Test scenarios:**
- Happy path: WikiWiki parser produces label map with correct labels for all nodes
- Edge case: Node appears only as target in routes — still gets a label→number entry (no edge inference)
- Regression: Routing rules still reference correct cell numbers matching their labels

**Verification:**
- WikiWiki parser output for maps 11/12/13 has correct label map, no cells/next_cells
- Routing rules reference cell numbers that match node labels
- Existing WikiWiki parser tests pass with updated assertions

---

- U4. **Clean up dead code and documentation**

**Goal:** Remove dead code from the BFS heuristic removal, update comments, verify no regressions.

**Requirements:** R3

**Dependencies:** U2, U3

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`

**Approach:**
- Remove the "target-only node" comment block and related dead code
- Remove routing_rules clearing comment (no longer needed)
- Update `assemble.rs` comments to reflect new data flow
- Evaluate `merge_variant_definition`'s `inferred_start` fallback logic (merge.rs lines 92-107) — if it has no live callers after WikiWiki bypasses this path, add to this unit's cleanup scope
- Verify no `clippy` warnings on changed code

**Test scenarios:**
- `cargo clippy --workspace` produces no new warnings
- `cargo test -p emukc_bootstrap` passes
- `cargo test -p emukc_model` passes

**Verification:**
- Clean clippy + test run

---

## System-Wide Impact

- **Interaction graph:** `assemble_final_map_catalog` output feeds into `Codex` which is used by all gameplay traits. Topology changes affect sortie routing, but since kcdata already provides correct topology for maps with kcdata data, runtime behavior is unchanged for the 130 kcdata-covered maps.
- **Error propagation:** WikiWiki parsing errors in routing rules are non-blocking — missing rules cause runtime fallback to `next_cells` (which now come from kcdata).
- **State lifecycle risks:** No database or persistent state changes. The refactoring affects only the bootstrap/build-time catalog assembly.
- **API surface parity:** `MapCatalog`, `MapDefinition`, `MapVariantDefinition` types are unchanged. No API surface changes.
- **Integration coverage:** Full pipeline test (`build_final_map_catalog` with both kcdata and wikiwiki sources) validates end-to-end behavior.
- **Unchanged invariants:** `merge_variant_definition` continues to work unchanged for public overlays and STAT.json. The WikiWiki path is the only one rerouted.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| WikiWiki routing rules reference cells not in kcdata (label mismatch) | `semantic_cell_no_map` returns identity mapping for unmapped cells — rules preserved but may not fire at runtime |
| 89 maps without WikiWiki data lose nothing — confirmed safe | kcdata provides full topology, routing_rules remain empty (runtime falls back to next_cells) |
| Existing tests depend on WikiWiki producing next_cells | Update test assertions in U3 — parser no longer produces cells |
| `merge_variant_definition` still used for overlays/STAT | Keep it unchanged, only bypass for WikiWiki via new `merge_routing_overlay` |
| `semantic_cell_no_map` refactor breaks overlay/STAT paths | Keep old function alongside new label-map overload, both tested |
| Pipeline fails if kcdata absent (R6 removed) | Acceptable — kcdata is a required data source, similar to how manifest is required |

---

## Sources & References

- **Audit report:** `.claude/plans/1-1-v-snoopy-forest.md` (previous brainstorm session)
- kcdata parser: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- WikiWiki parser: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`
- Merge logic: `crates/emukc_model/src/codex/map/merge.rs`
- Assembly: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Route evaluation: `crates/emukc_gameplay/src/game/map_route.rs`
