---
title: Fix Map System Audit Issues
type: fix
status: active
date: 2026-05-05
---

# Fix Map System Audit Issues

## Summary

Fix four correctness bugs discovered during the map system audit, all related to the wikiwiki-downgrade-to-routing-rules-only architecture: `next_cells` vs `routing_rules` mismatch in sortie lifecycle, indeterminate route fallback bypassing topology, unsafe event metadata merge, and unsupported `VisitedNodeLabel` at runtime.

---

## Problem Frame

After the refactor that made kcdata the sole source of graph topology and wikiwiki the source of routing rules only, the sortie lifecycle and data-merge paths still behave as if `next_cells` is the only routing information. This causes:

1. **P0 — Sortie termination at nodes with rules but empty `next_cells`**: `has_next`, `should_finish_sortie`, and the "no next route" guard in `next_sortie` all inspect `next_cells` exclusively. If kcdata does not emit an edge (e.g., a route-only intermediate placeholder) but wikiwiki provides `routing_rules`, the player cannot advance or the sortie ends prematurely.

2. **P1 — Client can teleport across non-adjacent cells**: The `all_source_unknown` fallback accepts any `selected_cell_id` that matches a rule target, without verifying it is a topological neighbor in `current.next_cells`.

3. **P1 — Event metadata silently overwritten by zero-value defaults**: `merge_cells` in `merge.rs` unconditionally overwrites `event_id` and `event_kind` from the secondary source, even when that source carries `0` (default / unset). This can corrupt node types during catalog assembly.

4. **P1 — `VisitedNodeLabel` predicate falls through to `Unsupported`**: `route_predicate_matches` does not handle `VisitedNodeLabel`, so any catalog that deserializes with that variant (e.g., hand-edited overlays) silently degrades to random routing.

---

## Requirements

- R1. Sortie lifecycle must treat a node as having outgoing routes when **either** `next_cells` is non-empty **or** `routing_rules` contains entries for that node.
- R2. Client-provided `selected_cell_id` must always be a valid topological neighbor (`current.next_cells` contains it) or a validated rule target, never an arbitrary rule target.
- R3. Secondary-source cell metadata merge must preserve the primary source's `event_id`/`event_kind` when the secondary value is `0` (default).
- R4. Runtime route evaluation must resolve `VisitedNodeLabel` by looking up the label in the stage's cells.
- R5. Every fix must have a regression test that fails before the fix and passes after.

---

## Scope Boundaries

- In scope: The four correctness bugs above, plus dead-code cleanup directly adjacent to the affected modules.
- Out of scope: Re-parsing or re-generating wikiwiki assets; changing the routing fallback strategy; adding new route predicates; overhauling resource-node logic or drop-filter semantics.

### Deferred to Follow-Up Work

- Resource node `color_no` → resource type mapping refinement (currently oversimplified in `resolve_non_battle_node_effect`).
- Drop tag filtering beyond `limited` (e.g., `event_only`, `seasonal`).

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_gameplay/src/game/map_route.rs` — `evaluate_route_destination`, `route_predicate_matches`, and existing pure-logic unit tests (`make_cell`, `make_unknown_rule`, etc.).
- `crates/emukc_gameplay/src/game/sortie.rs` — `start_sortie`, `next_sortie`, `sortie_battle_result`, `build_sortie_cell_data`, `resolve_non_battle_node_effect`.
- `crates/emukc_model/src/codex/map/merge.rs` — `merge_cells`, `merge_variant_definition`, `merge_routing_overlay`, and rich mock-data helpers (`cell(...)`).
- `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` — `load_map_catalog_from_cache_root` (dead code) and `build_variant_definition` (dead code, only called by the former).
- `crates/emukc_model/src/codex/map.rs` — `MapVariantDefinition::cell()`, `first_progress_cell_no()`.
- `crates/emukc_model/src/codex/map/types.rs` — `MapCellDefinition`, `RouteRule`, `RoutePredicate`.

### Test Patterns

- `map_route.rs` tests are pure unit tests: construct `MapStageDefinition` + `FleetRouteContext` → call `evaluate_route_destination` → assert `cell_no`.
- `sortie.rs` tests mix pure-logic (no DB) and async DB-backed tests using `new_mem_db()` + `Codex::load_without_cache_source("../../.data/codex")`.
- `merge.rs` tests use the `cell(...)` helper with full `MapCellDefinition` fields.

---

## Key Technical Decisions

- **Centralize outgoing-route check in `map_route.rs`**: Add `pub(crate) fn cell_has_routing_outgoing(cell_no, stage) -> bool` that checks both `next_cells` and `routing_rules`. This keeps the policy in one place and makes the intent explicit.
- **Resolve `VisitedNodeLabel` at runtime via stage lookup**: Instead of pre-lowering all labels to cell numbers during catalog parsing (which `rewrite_route_predicate_labels` already does for wikiwiki output), support the raw `VisitedNodeLabel` variant at evaluation time by scanning `stage.cells` for matching `node_label`. This makes hand-edited overlays and any future source that emits `VisitedNodeLabel` work without extra preprocessing.
- **Guard `event_id`/`event_kind` merge with non-zero check**: Align with how `color_no` is already handled in the same function (`if other.color_no > 0`); treat `0` as "no data" for these fields.

---

## Implementation Units

- U1. **Add `cell_has_routing_outgoing` helper in `map_route.rs`**

**Goal:** Provide a single function that answers whether a node has outgoing routes in the current stage, considering both topology (`next_cells`) and rules (`routing_rules`).

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (existing `#[cfg(test)]` module)

**Approach:**
- Add `pub(crate) fn cell_has_routing_outgoing(cell_no: i64, stage: &MapStageDefinition) -> bool`.
- Logic: `stage.cell(cell_no).is_some_and(|c| !c.next_cells.is_empty()) || stage.routing_rules.get(&cell_no).is_some_and(|r| !r.is_empty())`.
- Add unit tests covering: node with `next_cells` only, node with `routing_rules` only, node with both, node with neither.

**Patterns to follow:**
- Existing `evaluate_route_destination` signature style.
- Existing `map_route.rs` test helpers (`make_cell`).

**Test scenarios:**
- Happy path: node has `next_cells` → true.
- Happy path: node has empty `next_cells` but non-empty `routing_rules` → true.
- Edge case: node exists but both `next_cells` and `routing_rules` are empty → false.
- Edge case: `cell_no` not found in stage → false.

**Verification:**
- New tests pass.
- Function is referenced in at least one call site (U2).

---

- U2. **Replace `next_cells.is_empty()` checks in `sortie.rs` with `cell_has_routing_outgoing`**

**Goal:** Fix the P0 bug where sortie lifecycle ignores `routing_rules` when deciding if a node has outgoing routes.

**Requirements:** R1

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie.rs`
- Test: `crates/emukc_gameplay/src/game/sortie.rs` (existing `#[cfg(test)]` module)

**Approach:**
- Replace four locations that currently check `next_cells.is_empty()` or `!next_cells.is_empty()`:
  1. `SortieStartResponse.has_next` (line ~338): use `cell_has_routing_outgoing(first_cell, stage)`.
  2. `next_sortie` guard (line ~386): use `!cell_has_routing_outgoing(active.current_cell_id, stage)`.
  3. `SortieNextResponse.has_next` (line ~434): use `cell_has_routing_outgoing(next.cell_no, stage)`.
  4. `should_finish_sortie` (line ~610–611): use `!cell_has_routing_outgoing(current_cell.cell_no, stage)`.
- Add unit test: construct a variant where `next_cells` is empty but `routing_rules` has an `Always` rule; assert `evaluate_route_destination` advances and `has_next` is true.

**Patterns to follow:**
- Existing `sortie.rs` pure-logic tests (`route_rules_prefer_executable_predicates_over_static_next_cells`).

**Test scenarios:**
- Happy path: node with empty `next_cells` but valid `routing_rules` allows `next_sortie` to proceed.
- Integration: `start_sortie` returns `has_next = true` when the first node has rules but no static edges.
- Edge case: `sortie_battle_result` does **not** finish the sortie when the current node has rules but no static edges.

**Verification:**
- All new and existing `sortie.rs` tests pass.
- `cargo test -p emukc_gameplay` passes.

---

- U3. **Fix `all_source_unknown` fallback to validate `selected_cell_id` against `next_cells`**

**Goal:** Prevent the client from selecting a cell that is not a topological neighbor when all rules are `SourceUnknown`.

**Requirements:** R2

**Dependencies:** None (can parallelize with U1/U2)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (existing `#[cfg(test)]` module)

**Approach:**
- In `evaluate_route_destination`, in the `all_source_unknown` branch (line ~92–104), change the acceptance condition from `targets.contains(&selected_cell_id)` to `targets.contains(&selected_cell_id) && current.next_cells.contains(&selected_cell_id)`.
- The existing test `source_unknown_with_selected_cell_in_targets_not_in_next_cells` currently asserts that this teleport **should succeed**; that test must be updated to assert failure or to verify the new stricter behavior.

**Patterns to follow:**
- Existing `source_unknown_*` tests in `map_route.rs`.

**Test scenarios:**
- Error path: `all_source_unknown` rules target cell 10, but `current.next_cells` only contains 7 and 8; client sends `selected_cell_id = 10` → should fall back to `select_route_from_cells`, not return 10.
- Happy path: `all_source_unknown` rules target cell 10, and `current.next_cells` contains 10; client sends `selected_cell_id = 10` → should return 10.
- Edge case: no `selected_cell_id` provided → existing `select_route_from_cells` behavior unchanged.

**Verification:**
- Updated test asserts the new stricter behavior.
- `cargo test -p emukc_gameplay` passes.

---

- U4. **Guard `merge_cells` against zero-value `event_id`/`event_kind` overwrite**

**Goal:** Prevent secondary-source catalog merges from corrupting node event metadata with default zeros.

**Requirements:** R3

**Dependencies:** None (can parallelize)

**Files:**
- Modify: `crates/emukc_model/src/codex/map/merge.rs`
- Test: `crates/emukc_model/src/codex/map/merge.rs` (existing `#[cfg(test)]` module)

**Approach:**
- In `merge_cells`, change:
  ```rust
  cell.event_id = other.event_id;
  cell.event_kind = other.event_kind;
  ```
  to:
  ```rust
  if other.event_id != 0 {
      cell.event_id = other.event_id;
  }
  if other.event_kind != 0 {
      cell.event_kind = other.event_kind;
  }
  ```
- Add unit test in `merge.rs` that merges a primary cell with `event_id = 4, event_kind = 1` against a secondary cell with `event_id = 0, event_kind = 0`; assert the primary values are preserved.

**Patterns to follow:**
- Existing `color_no` guarded merge in the same function (`if other.color_no > 0`).
- Existing `merge_routing_overlay_remaps_rules_without_touching_cells` test structure.

**Test scenarios:**
- Happy path: secondary has non-zero `event_id`/`event_kind` → primary is overwritten.
- Edge case: secondary has zero `event_id`/`event_kind` → primary is preserved.
- Edge case: primary is zero and secondary is non-zero → secondary wins.

**Verification:**
- New test passes.
- `cargo test -p emukc_model` passes.

---

- U5. **Add runtime support for `VisitedNodeLabel` in `route_predicate_matches`**

**Goal:** Make `VisitedNodeLabel` work at runtime by resolving labels against the stage's cell definitions.

**Requirements:** R4

**Dependencies:** None (can parallelize)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (existing `#[cfg(test)]` module)

**Approach:**
- Change the `VisitedNodeLabel` arm in `route_predicate_matches` from returning `Unsupported` to:
  1. Collect `cell_nos` by looking up each `node_label` in `stage.cells` (matching `node_label` field).
  2. Evaluate as `VisitedNode { cell_nos, visited }`.
- The function signature currently does not take `stage`; it takes only `predicate` and `context`. To avoid breaking all call sites, build a small lookup closure or add a `stage: &MapStageDefinition` parameter. Since this is `pub(crate)` and all call sites are in the same module, adding the parameter is safe and preferred.
- Update all call sites in `map_route.rs` to pass `stage`.

**Patterns to follow:**
- `rewrite_route_predicate_labels` in `wikiwiki_map/mod.rs` shows the label-to-cell_no lookup pattern.

**Test scenarios:**
- Happy path: `VisitedNodeLabel { node_labels: ["A"], visited: true }` with a stage that has a cell labeled "A" at `cell_no = 2`, and `context.visited_cell_ids` contains 2 → `Matched`.
- Edge case: label not found in stage → treat as `NotMatched` (or `Unsupported` if we want to be conservative; `NotMatched` is safer for deterministic routing).
- Edge case: `visited: false` and label has not been visited → `Matched`.

**Verification:**
- New tests pass.
- All existing `route_predicate_matches` tests still pass.
- `cargo test -p emukc_gameplay` passes.

---

- U6. **Remove dead code `load_map_catalog_from_cache_root` and `build_variant_definition`**

**Goal:** Eliminate unused functions that build a linear (incorrect) topology, reducing maintenance confusion.

**Requirements:** None (cleanup)

**Dependencies:** None (can parallelize)

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`

**Approach:**
- Delete `load_map_catalog_from_cache_root` (lines ~81–184) and `build_variant_definition` (lines ~310–411).
- Remove the `#[allow(dead_code)]` attribute if it was only there for these functions.
- Verify no other code references either function.

**Patterns to follow:**
- `cargo clippy --workspace` should report no unused-function warnings after removal.

**Test expectation:** none — this is pure deletion with no behavioral change.

**Verification:**
- `cargo build --workspace` succeeds.
- `cargo clippy --workspace` is clean.

---

## System-Wide Impact

- **Interaction graph:** `cell_has_routing_outgoing` becomes a new shared helper used by sortie lifecycle, battle result, and start response. No external API changes.
- **Error propagation:** The stricter `all_source_unknown` check may cause `next_sortie` to return an error when a malicious client sends an invalid `selected_cell_id`. This is the correct behavior.
- **State lifecycle risks:** `should_finish_sortie` now considers `routing_rules`, so a sortie will no longer end prematurely at a node that has rules but no static edges. This is a behavioral fix, not a risk.
- **Unchanged invariants:**
  - `next_cells` remains the sole source of graph topology; `routing_rules` never creates edges that don't exist in topology.
  - Wikiwiki parsing and catalog assembly pipelines are untouched.
  - Battle simulation (`emukc_battle`) is unaffected.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `all_source_unknown` stricter check breaks legitimate client behavior | The client already sends `selected_cell_id` that matches `next_cells` in normal play; this only rejects out-of-bounds values. Unit tests verify both paths. |
| `VisitedNodeLabel` resolution has ambiguous label-to-cell mapping if duplicates exist | `unique_labeled_cells` in `merge.rs` already handles duplicate labels by excluding them from the semantic map. Runtime resolution should match this behavior: if a label maps to multiple cells, conservatively return `NotMatched` to avoid nondeterministic routing. |
| `event_id`/`event_kind` zero-guard changes merge behavior for legitimate zero values | In the current data model, `event_id = 0` means "start/nothing" and `event_kind = 0` means "non-battle". No legitimate secondary source intentionally overwrites a non-zero primary value with zero. The change is safe. |

---

## Sources & References

- Related code:
  - `crates/emukc_gameplay/src/game/map_route.rs`
  - `crates/emukc_gameplay/src/game/sortie.rs`
  - `crates/emukc_model/src/codex/map/merge.rs`
  - `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- Related plans:
  - `docs/plans/2026-05-05-003-refactor-map-topology-routing-separation-plan.md`
  - `docs/plans/2026-05-05-004-fix-map-refactor-audit-and-sortie-state-plan.md`
