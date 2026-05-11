---
title: "fix: Restore kcdata route-cell API parity"
type: fix
status: completed
date: 2026-05-11
---

# fix: Restore kcdata route-cell API parity

## Summary

Restore kcdata map parsing so KanColle API cell identity stays authoritative: each kcdata route becomes the API cell with the same `cell_no`, and duplicate node labels are handled in overlay logic instead of by collapsing the topology. This plan fixes the 1-3 API regression where sortie can start at the wrong API cell and then advance through a topology that does not match real `api_req_map/start` / `api_req_map/next` responses.

---

## Problem Frame

The current uncommitted kcdata parser direction builds one cell per unique graph node. That solves duplicate label indexing locally, but it violates the API contract: `api_cell_data[].api_no` is the sea-area-local cell ID, and `api_req_map/next.api_no` is the next cell ID. In the real 1-3 start capture, the map has 14 API cells `0..13`, starts from `api_from_no=0`, can land on `api_no=3`, and reports `api_bosscell_no=10`.

The generated catalog currently has 14 cells after overlays fill missing API IDs, but the first part of the topology has been renumbered as unique nodes. For 1-3 that leaves `cell 0.next_cells=[1,4]`, so runtime can return API cell 4 from start; real topology should route from start to route/API cells 1 or 3. Runtime sortie projection is mostly passing through the catalog's `cell_no`, so the bug belongs in catalog construction and overlay mapping, not in API projection.

---

## Requirements

- R1. `build_variant_from_kcdata` produces one `MapCellDefinition` per kcdata route, with `cell_no = route_id`; it must not renumber routes into sequential unique-node IDs.
- R2. A route with `from: null` is a start-source route-cell at that route ID. Source-null routing must not collapse the route target or overwrite target/API metadata with a generic synthetic start cell.
- R3. A route-cell's `next_cells` are the route IDs whose `from` node equals the current route's `to` node.
- R4. Node metadata is copied to every route-cell that targets that node. Duplicate `node_label` values are valid for route/API identity.
- R5. `boss_cell_no` matches the real API cell ID. For 1-3, the default boss cell is 10, not the unique label index for `J`.
- R6. Label-keyed wikiwiki overlays support duplicate labels without collapsing cells: enemy fleets and drops fan out by node label, while routing rules resolve through the actual route-cell topology.
- R7. Real API verification keeps cell-count, `api_no`, master-cell ID, color, and boss-cell mismatches visible instead of skipping them, and automated parity tests must build or inject fresh catalog data from tracked inputs instead of depending on a stale `.data/codex` snapshot.
- R8. The 1-3 API regression is covered at the catalog and gameplay/API boundary: start must not return D/API cell 4 from cell 0, next-step routing must use API cell IDs from the route-cell graph, and multi-start maps must not hard-code cell 0 when the active source route-cell is another API cell.
- R9. Stale plans and tests that encode the unique-node premise are superseded so future work does not reintroduce it.

---

## Scope Boundaries

- Do not change KanColle API projection semantics except for tests that prove it receives correct catalog data. `api_no` and `api_from_no` should continue to be projected from runtime `cell_no`.
- Do not change the runtime route evaluator to hide catalog topology errors. Route fallback behavior can remain for degraded or indeterminate rules, but it is not the fix for wrong `next_cells`.
- Do not regenerate or normalize wikiwiki route topology as part of this fix. The route-cell API identity must be established from kcdata first.
- Do not refactor battle, enemy composition, drop, or map-record systems beyond the overlay changes required for duplicate label parity.
- Do not leave `.data/codex/map_catalog.json` as a source-of-truth patch or automated-test dependency. It is generated output in this checkout; implementation should validate regeneration locally and commit only tracked source/test/doc changes unless the repository policy changes.

### Deferred to Follow-Up Work

- Broader route-evaluator policy around "rules filtered by topology, falling back to next_cells" can be reviewed separately after the catalog emits API-correct topology.
- A full event-map/multi-gauge audit can follow once default regular-map parity is restored and real-capture verification has a clean baseline.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` builds the kcdata-derived `MapVariantDefinition`. The current uncommitted version implements the incorrect unique-node model.
- `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs` merges wikiwiki label-keyed routing, enemy, and drop data using a single `label_to_cell_no` index. This is where duplicate labels need route-aware handling.
- `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` selects the kcdata topology plus wikiwiki label overlay path and runs map validation after assembly.
- `crates/emukc_bootstrap/src/map_pipeline/verify.rs` already compares embedded real start captures against `.data/codex/map_catalog.json`; it should stay strict about cell-count mismatch for manual smoke verification, while automated parity tests should rebuild or inject a fresh catalog from tracked inputs.
- `crates/emukc_model/src/codex/map/types.rs` exposes `MapVariantDefinition::label_to_cell_no()`, which deliberately excludes duplicate labels. That behavior is correct for callers that require uniqueness, but it is not sufficient for route-cell overlay fanout.
- `crates/emukc_model/src/codex/map/merge.rs` contains identity-fallback remapping for legacy cell-number overlays and tests documenting duplicate-label exclusion.
- `crates/emukc_gameplay/src/game/sortie/mod.rs` calls `evaluate_route_destination(cell_0, ...)` at start and then returns the selected `current_cell.cell_no` as the API `api_no`.
- `src/bin/net/router/kcsapi/api_req_map/projection.rs` projects sortie responses to KCSAPI fields without translating cell IDs.
- `docs/apilist.txt` defines `api_cell_data[].api_no` as the sea-area-local cell ID and `api_req_map/next.api_no` as the next cell ID.
- `docs/real_data/map_start_data/map_1-3.json` is the grounding capture for 1-3 API parity.
- `.data/temp/kc_data/_map/13.json` shows 1-3 routes `0..13`, including convergent labels `E`, `F`, and `J`.
- `docs/plans/archive/2026-05-09-001-fix-kcdata-cell-per-route-plan.md` contains the correct historical route-cell direction.
- `docs/plans/2026-05-11-001-refactor-kcdata-cell-topology-plan.md` and `docs/plans/2026-05-11-002-fix-kcdata-test-gaps-plan.md` encode the now-rejected unique-node premise.

### Institutional Learnings

- No `docs/solutions/` directory exists in this checkout. The closest institutional context is in prior map-system plans under `docs/plans/`.
- `docs/plan.md` states that `node_label` merge identity is display-oriented and `cell_no` is the canonical key. This supports preserving API cell identity and solving label duplication as overlay logic.

### External References

- No external research is needed. The authoritative sources for this bug are local API docs, real captured API data, kcdata route data, and current runtime code.

---

## Key Technical Decisions

- **Route ID is API cell identity:** `cell_no` must equal kcdata route ID because real API `api_no` tracks route cells, including multiple cells that target the same node label.
- **Duplicate labels are valid data, not parser failure:** Route cells can share `node_label` when different route IDs target the same KanColle node. Overlay code must support one label mapping to many route cells.
- **Overlay routing resolves through topology, not arbitrary label choice:** A label-keyed rule like `H -> J` should target the route cell whose source is `H` and target label is `J` (1-3 cell 13), while `F -> J` should target the route cell from `F` to `J` (1-3 cell 10). A flat label-to-one-cell map cannot represent this.
- **Enemy fleets and drops fan out by node label:** Enemy and drop overlays describe the node encounter; when the same node label has multiple route/API cells, each matching route-cell should receive the same node-level enemy/drop data unless a more specific cell-number source overrides it.
- **Runtime remains a consumer, not the correction layer:** `start_sortie`, `next_sortie`, and API projection should not translate unique-node IDs back to API IDs. If the catalog is right, those layers can keep returning `cell_no`.
- **Source-null routes are routing sources, not metadata policy:** `from: null` identifies an API source route-cell. The cell still keeps route/API identity and any target/API metadata available for that route; only missing display metadata should fall back to an explicit source/start placeholder.
- **Automated parity checks own catalog freshness:** Tests that claim parser or API parity must build or inject the catalog from tracked kcdata, repo wikiwiki assets, overlays, and real-capture fixtures during setup. `.data/codex/map_catalog.json` is acceptable only as a manual smoke artifact after `cargo run -- bootstrap`.

---

## Open Questions

### Resolved During Planning

- **Is `api_no` a graph-node ID or an API cell ID?** It is an API cell ID local to the map. `docs/apilist.txt` and the 1-3 real capture both confirm this.
- **Why does 1-3 start at D then advance to F?** The generated topology lets cell 0 route to catalog cell 4, whose master/API identity is the real D cell. From there the unique-node topology points to cell 6/F. The runtime is following bad catalog topology.
- **Should unique labels be enforced in kcdata output?** No. Enforcing unique labels destroys API cell identity. Duplicate labels must be handled by overlay fanout and route-aware rule resolution.

### Deferred to Implementation

- **Exact helper placement for multi-label overlay indexing:** Prefer a local helper in `label_overlay.rs` unless implementation shows a shared model helper is cleaner. The behavior is required; the helper boundary can be chosen during implementation.
- **Event/multi-gauge exceptions in real-capture verification:** If strict verification exposes genuine phase-specific capture mismatches, add explicit documented exceptions instead of restoring blanket skips.

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

```text
kcdata routes
  route_id = API cell_no
  route.from = source node
  route.to = target node / display label
        |
        v
route-cell catalog topology
  source route cells = route IDs whose from is null
  cell(route_id).node_label = route.to label or target/API metadata
                              when present, with source/start fallback only
                              when metadata is absent
  cell(route_id).next_cells = route IDs where next.from == route.to
        |
        v
wikiwiki label overlay
  enemy/drop label -> all route cells with that label
  route from_label/to_label -> source route cells with from_label,
                               filtered through each source cell's next_cells
                               whose target label is to_label
        |
        v
gameplay/API
  start chooses the active source route-cell set for the map/stage,
  then start/next return selected route-cell cell_no directly as api_no
```

For 1-3, this means route/API cell 0 has `next_cells=[1,3]`; route 3 is `1 -> C`, and route 4 is `A -> D`, so route 4 must never be a direct start candidate.

---

## Implementation Units

### U1. Restore Route-Cell kcdata Parsing

**Goal:** Replace the unique-node parser with route-cell construction where every kcdata route emits exactly one `MapCellDefinition` with `cell_no = route_id`.

**Requirements:** R1, R2, R3, R4, R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- Test: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`

**Approach:**
- Iterate routes in route-ID order and construct cells keyed by the route ID, not by node discovery order.
- Treat `from: null` routes as source route-cells. The source route's `next_cells` should be the route IDs whose `from` equals the source route's `to` node, but source-null status must not force the cell's `node_label`, color, or master-cell metadata to a generic `Start` value.
- Copy metadata from `data.cells[route.to]` when present for every route-cell, including source route-cells. If route target metadata is missing, emit a valid source/start fallback label and default non-battle metadata without losing the route ID.
- If multiple route IDs target the same label, each cell receives the same node metadata but keeps its distinct `cell_no`.
- Build `next_cells` by matching route source nodes to the current route target node.
- Set `boss_cell_no` to the first route ID whose target node metadata is boss. For 1-3 this is route/API cell 10 even though route 13 also targets `J`.
- Remove or rewrite unique-node-only tests and helpers that assume labels are one-to-one with cells.

**Execution note:** Add characterization assertions for 1-3 route IDs before rewriting the parser logic, then update the parser until those assertions pass.
Also add characterization coverage for a multi-start capture such as 6-4, where real `api_from_no=22` proves that source route-cells are not always cell 0.

**Patterns to follow:**
- Existing inline kcdata parser tests in `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`.
- The route-cell direction in `docs/plans/archive/2026-05-09-001-fix-kcdata-cell-per-route-plan.md`.

**Test scenarios:**
- Happy path: 1-3 kcdata produces 14 cells with `cell_no` values `0..13`, not 10 unique-node cells.
- Happy path: 1-3 cell 0 has `next_cells=[1,3]`; cell 1/A has `[4,5]`; cell 3/C has `[6]`; cell 4/D has `[2]`.
- Happy path: 1-3 cells 5 and 11 both have label `E`, cells 6 and 12 both have label `F`, and cells 10 and 13 both have label `J`.
- Happy path: 1-3 `boss_cell_no` is 10.
- Edge case: a route whose target metadata is missing still emits a route-cell with default non-battle metadata and valid outgoing topology.
- Edge case: multiple `from: null` routes keep their own route IDs and are not collapsed into a single synthetic start.
- Edge case: a nonzero source route-cell such as 6-4 route/API cell 22 preserves its route/API identity and available real-capture metadata instead of being normalized to cell 0 or a generic `Start` node.
- Regression: all repo kcdata maps produce cells whose `next_cells` point to existing route-cell IDs.

**Verification:**
- The kcdata test suite proves route count, route IDs, duplicate labels, and graph edges for representative maps.
- No warning about an unused `route_id` remains in `kcdata.rs`.

---

### U2. Make Label Overlay Duplicate-Label Aware

**Goal:** Merge label-keyed wikiwiki data onto route-cell topology without relying on `label_to_cell_no()` returning a unique cell.

**Requirements:** R4, R6

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs`
- Modify if needed: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Modify if shared helper is justified: `crates/emukc_model/src/codex/map/types.rs`
- Test: `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs`

**Approach:**
- Build a multi-label index from the target variant: `label -> all route-cell IDs`.
- For enemy fleets and ship drops, apply the overlay item to every cell in the multi-label index for that label.
- For routing rules, expand from all source cells with the `from_label`, then resolve `to_label` by looking at each source cell's `next_cells` and selecting only next cells whose `node_label` equals `to_label`.
- Convert `VisitedNodeLabel` predicates to all cell IDs for each referenced label, deduplicated and stable.
- Keep unmatched-label and impossible-edge warnings, but distinguish "label absent" from "label present but no route edge from this source to this target".
- Keep `MapVariantDefinition::label_to_cell_no()` semantics for callers that require unique labels; do not make it silently choose one duplicate.

**Patterns to follow:**
- Existing `merge_label_overlay` tests in `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs`.
- Existing duplicate-label exclusion tests in `crates/emukc_model/src/codex/map/merge.rs`.

**Test scenarios:**
- Happy path: an enemy overlay for label `E` on a variant with cells 5/E and 11/E creates enemy fleet entries at both 5 and 11.
- Happy path: a drop overlay for duplicate label `J` attaches to both 10/J and 13/J.
- Happy path: route rule `F -> J` on 1-3-style topology maps to target cell 10 from every F source cell, and does not target 13.
- Happy path: route rule `H -> J` maps to target cell 13 and does not target 10.
- Edge case: `VisitedNodeLabel { node_labels: ["E"] }` converts to both E route cells.
- Edge case: a label exists but there is no edge from the source cell to a target with that label; the rule is dropped with a specific warning.
- Regression: unique-label overlays still merge exactly once and preserve existing behavior.

**Verification:**
- Overlay tests demonstrate duplicate labels no longer cause enemy fleets, drops, or routing rules to be silently dropped.
- Assembly report drop counts decrease for duplicate-label maps without accepting invalid route edges.

---

### U3. Strengthen Real API Parity Verification

**Goal:** Turn the 1-3 capture and embedded real start captures into durable API-cell parity checks.

**Requirements:** R5, R7

**Dependencies:** U1, U2

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/verify.rs`
- Test: `crates/emukc_bootstrap/src/map_pipeline/verify.rs`

**Approach:**
- Build the catalog under test from tracked inputs during setup rather than reading a pre-existing `.data/codex/map_catalog.json`. Prefer an in-process helper using `build_final_map_catalog` / `build_final_map_catalog_from_repo_assets`, matching the existing gameplay-test pattern that rebuilds `codex.maps` from repo assets.
- Keep the current strict behavior that records cell-count mismatch as a failure.
- Add a focused 1-3 assertion block that checks real-capture essentials: 14 cells, `api_no` set `0..13`, `api_from_no=0`, captured start `api_no=3`, and `api_bosscell_no=10`.
- Add focused 1-3 topology assertions alongside the capture checks: cell 0's outgoing route cells are 1 and 3, and cell 4 is not a start candidate.
- Compare catalog cells by API `api_no` rather than vector position.
- Keep master-cell ID and color checks against real captures. If generated fallback master IDs differ from API captures, treat that as a catalog overlay/parity issue to fix, not as a reason to skip the comparison.
- If event or phase captures require exceptions, encode them as named cases with comments and narrow assertions.

**Patterns to follow:**
- Existing `real_game_cells_match_catalog_cell_no_and_color` structure in `crates/emukc_bootstrap/src/map_pipeline/verify.rs`.
- Existing `parse_capture` helper for extracting map ID, cell data, and boss cell.

**Test scenarios:**
- Happy path: 1-3 catalog has cells `0..13`, `boss_cell_no=10`, and colors/master IDs matching the real capture.
- Regression: a catalog with 10 unique-node cells plus 4 overlay-filled cells fails because the focused 1-3 topology check catches `0 -> 4`.
- Regression: the automated parity test fails if it is pointed at a stale `.data/codex/map_catalog.json` instead of the freshly built in-process catalog.
- Edge case: a map missing from catalog reports a mismatch with map ID instead of being silently skipped.
- Edge case: a real capture with no `api_cell_data` is ignored as malformed fixture input, matching current helper behavior.

**Verification:**
- The verification suite fails on the current unique-node parser output and passes only when route-cell API parity is restored.
- Failure output identifies the map and cell ID responsible for a mismatch.
- Manual smoke verification may still use `.data/codex/map_catalog.json`, but only after explicitly regenerating it with `cargo run -- bootstrap`.

---

### U4. Add 1-3 Gameplay/API Regression Coverage

**Goal:** Prove the public sortie path cannot reproduce the user-reported 1-3 start-to-D-to-F behavior once the catalog is corrected.

**Requirements:** R8

**Dependencies:** U1, U2, U3

**Files:**
- Modify if needed: `crates/emukc_gameplay/src/game/sortie/mod.rs`
- Modify: `src/bin/net/router/kcsapi/api_req_map/mod.rs`
- Or modify: `crates/emukc_gameplay/tests/sortie_battle.rs`
- Or modify: `tests/gameplay_tests/map/mod.rs` plus a focused new map test file if that is the local convention chosen during implementation.

**Approach:**
- Define the runtime start-source policy before adding assertions: when a map variant has multiple `from: null` route-cells, `start_sortie` must evaluate the active source route-cell set for that map/stage instead of always calling the route evaluator from cell 0. The selected route-cell `cell_no` remains the API `api_from_no` / `api_no` source identity.
- Add a public API or gameplay-level test for map 1-3 that inspects `api_cell_data` and the first returned `api_no`.
- Build or inject a fresh catalog in test setup from tracked inputs, following the `mock_context_with_repo_wikiwiki_maps` pattern in `crates/emukc_gameplay/tests/sortie_battle.rs`; do not rely on whatever `.data/codex/map_catalog.json` happens to contain.
- Set up the test profile so 1-3 is unlocked, either by using existing map-record helpers or by driving the prerequisite unlock path deliberately.
- Assert that `api_cell_data` exposes API cell IDs `0..13` and includes boss cell 10.
- Assert that a start from cell 0 returns only a valid route-cell start candidate for 1-3, especially not API cell 4.
- If first-cell selection remains random, make the assertion set-based rather than seed-dependent: valid start candidates are 1 and 3 for the default 1-3 topology.
- Add a next-step assertion for a deterministic branch where practical. For example, if the test starts at cell 3/C, the next route should go through route/API cell 6/F with `api_from_no=3`; it should not infer this path through unique-node cell 4/D.
- Add a multi-start regression using a captured nonzero source such as 6-4 (`api_from_no=22`) so future changes cannot satisfy the 1-3 case by preserving a hidden cell-0 assumption.

**Patterns to follow:**
- Existing API tests in `src/bin/net/router/kcsapi/api_req_map/mod.rs`.
- Existing gameplay setup helpers in `crates/emukc_gameplay/tests/sortie_battle.rs` and `tests/gameplay_tests/map/`.

**Test scenarios:**
- Integration: `api_req_map/start` for 1-3 returns `api_cell_data[].api_no == 0..13`.
- Integration: `api_req_map/start` for 1-3 returns `api_bosscell_no == 10`.
- Regression: repeated fresh 1-3 starts never return `api_no=4` from `api_from_no=0`.
- Integration: when current cell is 3/C, `api_req_map/next` returns from cell 3 and advances to a valid route-cell target in that cell's `next_cells`.
- Regression: a 6-4-style start can originate from source route/API cell 22 and does not route through cell 0 unless the active start policy explicitly chooses cell 0.

**Verification:**
- The API/gameplay test fails against the current unique-node topology or stale-catalog setup and passes after route-cell catalog generation plus start-source policy correction.

---

### U5. Supersede Stale Unique-Node Plans and Test Intent

**Goal:** Prevent the already-written unique-node plans from acting as live instructions after this correction.

**Requirements:** R9

**Dependencies:** U1

**Files:**
- Modify: `docs/plans/2026-05-11-001-refactor-kcdata-cell-topology-plan.md`
- Modify or remove before commit: `docs/plans/2026-05-11-002-fix-kcdata-test-gaps-plan.md`
- Modify if needed: `docs/plans/archive/2026-05-09-001-fix-kcdata-cell-per-route-plan.md`

**Approach:**
- Mark plan 001 as superseded or add an explicit warning near the top that its unique-node premise is incorrect for API parity.
- Do not leave plan 002 as an active untracked plan that strengthens unique-node tests. Either delete it before commit or mark it superseded with a pointer to this plan.
- Keep the archived 2026-05-09 route-cell plan as historical support, but do not make implementers reconcile both documents manually; this plan is the current source of truth.

**Test scenarios:**
- Test expectation: none -- documentation hygiene only.

**Verification:**
- `docs/plans/` has only one active plan for the kcdata route-cell/API parity fix.
- Grepping active plans for "one cell per unique node" does not surface a live implementation directive.

---

## System-Wide Impact

- **Interaction graph:** kcdata parsing feeds `assemble_final_map_catalog`, which feeds `.data/codex/map_catalog.json`, which is loaded by `Codex` and consumed by sortie start/next, battle selection, routing rules, and KCSAPI projection.
- **Error propagation:** Parser or overlay mismatches should surface as bootstrap/test warnings or verify failures. Runtime should not silently translate bad topology into plausible API responses.
- **State lifecycle risks:** Existing active sortie state stores `current_cell_id`. Changing catalog topology while a sortie is active can make stale state invalid, but this is a development/bootstrap data change rather than a live migration in this local emulator context.
- **API surface parity:** `api_req_map/start.api_cell_data[].api_no`, `api_req_map/start.api_no`, `api_req_map/start.api_from_no`, `api_req_map/next.api_no`, and `api_req_map/next.api_from_no` all depend on route-cell identity.
- **Integration coverage:** Parser-only tests are insufficient because the bug manifests after overlay merge and API projection. Verification must span kcdata, label overlay, generated catalog, and sortie API behavior.
- **Unchanged invariants:** `cell_no` remains the canonical map cell key. `node_label` remains display/semantic metadata and may be non-unique.

---

## Alternative Approaches Considered

- **Keep unique-node topology and translate to API IDs later:** Rejected. It creates two competing cell identities, complicates every runtime and overlay caller, and contradicts the API fields that already use route/API cell IDs.
- **Choose one duplicate label as canonical in `label_to_cell_no()`:** Rejected. It would make overlays appear to succeed while assigning route rules, enemy fleets, or drops to the wrong API cell.
- **Disable route fallback in gameplay first:** Deferred. The fallback can mask topology bugs, but removing it does not fix the generated catalog or real API parity.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Duplicate-label routing rules fan out too broadly | Resolve rule targets through each source cell's `next_cells` and target labels, not through all cells sharing the target label. |
| Enemy/drop fanout duplicates data onto cells that the game treats differently | Start with node-level fanout because wikiwiki labels describe node encounters; preserve cell-number overlays as higher-specificity data when available. |
| Strict real-capture verification exposes unrelated stale catalog data | Keep failures visible, then add narrow documented exceptions only for proven phase-specific captures. |
| Runtime tests become flaky because first-cell selection is random | Assert membership in valid route-cell candidate sets and explicitly exclude the invalid 1-3 start cell 4. |
| Multi-start maps keep a hidden cell-0 assumption | Add start-source policy coverage for a nonzero source route-cell such as 6-4 `api_from_no=22`. |
| Generated `.data/codex/map_catalog.json` hides source changes | Build or inject fresh catalog data inside automated tests; treat generated catalog diff as manual verification output unless it becomes a tracked artifact by policy. |

---

## Documentation / Operational Notes

- Automated parity tests should not depend on a pre-existing `.data/codex/map_catalog.json`; they should rebuild or inject the catalog from tracked inputs during setup.
- For manual smoke verification after implementation, regenerate the local codex catalog with `cargo run -- bootstrap` before running gameplay/API checks that intentionally load `.data/codex`.
- The implementation should preserve the diagnosis in commit or PR notes: the root cause is API cell identity loss in kcdata parsing, with duplicate-label overlay support as the companion fix.
- If plan 002 remains untracked, decide before committing whether to remove or supersede it so it does not appear as a live instruction later.

---

## Sources & References

- API field docs: `docs/apilist.txt`
- Real 1-3 start capture: `docs/real_data/map_start_data/map_1-3.json`
- Raw 1-3 kcdata route source: `.data/temp/kc_data/_map/13.json`
- Generated catalog used by runtime tests: `.data/codex/map_catalog.json`
- kcdata parser: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- Label overlay merge: `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs`
- Map catalog assembly: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Real-capture verification: `crates/emukc_bootstrap/src/map_pipeline/verify.rs`
- Map model and label indexing: `crates/emukc_model/src/codex/map/types.rs`
- Map merge/remap behavior: `crates/emukc_model/src/codex/map/merge.rs`
- Sortie start/next runtime: `crates/emukc_gameplay/src/game/sortie/mod.rs`
- Route evaluator fallback behavior: `crates/emukc_gameplay/src/game/map_route.rs`
- KCSAPI map projection: `src/bin/net/router/kcsapi/api_req_map/projection.rs`
- Current wrong active plan: `docs/plans/2026-05-11-001-refactor-kcdata-cell-topology-plan.md`
- Current stale test-gap plan: `docs/plans/2026-05-11-002-fix-kcdata-test-gaps-plan.md`
- Correct archived direction: `docs/plans/archive/2026-05-09-001-fix-kcdata-cell-per-route-plan.md`
- Recent relevant commits: `c12c3d7`, `fcb3194`, `5bd5b04`
