---
title: "fix: Boss-Node Equivalence — Recognize All Cells Sharing the Boss Node as Boss"
type: fix
date: 2026-06-22
origin: investigation of 2 stable-failing boss-path tests (plan 003 scope, U1.2/U3.1)
---

# fix: Boss-Node Equivalence — Recognize All Cells Sharing the Boss Node as Boss

## Summary

Two stable-failing integration tests — `sortie_battle_result_advances_boss_quest_on_real_boss_node`
and `repo_wikiwiki_asset_supports_real_map_boss_progression` (both in
`crates/emukc_gameplay/tests/sortie_battle.rs`) — fail because boss detection compares a
single canonical cell number, but a boss **node** reachable from multiple routes produces
multiple boss **cells**, and only the first is recognized as boss.

The data layer is already correct: on map 1-2, node E (boss) is represented as two cells
(5 via A→E, 6 via D→E); both carry `event_id=5` (boss) and both have `enemy_fleets` entries.
The defect is purely in runtime boss detection: two sites compare `current_cell.cell_no ==
active.boss_cell_id` (exact single-cell match), so reaching cell 6 is not recognized as the
boss and the sortie does not finish / the boss quest does not advance.

## Problem Frame

KanColle maps (as decoded from kcdata) model the map as a directed graph: **routes are
edges, cells/nodes are vertices.** A node targeted by multiple incoming routes produces
multiple cells (one per targeting route), each preserving the real KanColle API numbering
(`api_no` = route id). This model is intentional and protected by 5 dedicated kcdata tests
(`convergent_routes_preserve_duplicate_route_cells`, `parallel_edges_preserve_distinct_route_cells`,
etc.) — collapsing same-node cells was attempted and reverted because it breaks real-API
numbering.

The consequence: a boss node with ≥2 incoming routes (e.g. map 1-2 node E, reachable via
A→E and D→E) yields ≥2 boss cells. `MapVariantDefinition.boss_cell_no: i64` captures only
the first (5). The remaining boss cell(s) (6) are fully provisioned (event_id, enemy fleet,
boss color) but invisible to boss detection.

**R1.** Reaching any cell that shares the boss node must be treated as reaching the boss:
sortie finishes, boss quest advances, boss battle triggers.

**R2.** The fix must not change the kcdata route→cell model (protected by tests) and must not
change the real-API cell numbering visible to the client.

**R3.** `boss_cell_no: i64` remains the canonical/representative boss cell for clients and
serialization; only the *membership test* ("is this cell the boss?") becomes label-aware.

**R4.** No behavioral regression on maps where the boss node has a single incoming route
(the common case): detection results must be byte-for-byte identical.

## Scope Boundaries

### In scope

- A label-aware "is this cell a boss cell?" equivalence helper on `MapVariantDefinition`.
- Updating the two runtime boss-detection sites in `crates/emukc_gameplay/src/game/sortie/mod.rs`.
- Updating the in-repo test helper `path_to_boss` (`tests/sortie_battle.rs`) so its DFS
  targets all boss cells.
- Regression tests pinning the equivalence behavior.

### Non-goals

- Changing the kcdata route→cell parser (reverted once already; breaks real-API numbering).
- Changing `boss_cell_no`'s type to `Vec<i64>` (over-invasive; see KTD1).
- Fixing unrelated pre-existing failures (`make_list::build_cache_list_paths_with_manifest_path_matches_repo_manifest_strategy`).
- The map 1-3 routing investigation (separate plan 003 residual); this plan is map-agnostic
  and fixes the equivalence defect for all maps at once.

## Key Technical Decisions

### KTD1. Helper on `MapVariantDefinition`, not a type change

**Decision:** Keep `boss_cell_no: i64` as the canonical boss cell. Add a helper
`boss_cell_nos(&self) -> Vec<i64>` (or `is_boss_cell(cell_no) -> bool`) on
`MapVariantDefinition` that derives the full boss-cell set from `boss_cell_no`'s
`node_label` via the **existing** `multi_label_index()` helper
(`crates/emukc_model/src/codex/map/types.rs:106`). Update only the two runtime detection
sites to call the helper.

**Rationale:** `multi_label_index()` already returns `node_label → Vec<cell_no>` and already
preserves duplicates — the infrastructure for multi-cell-per-label exists and is used by the
merge logic. The blast radius of a helper is two call sites; the blast radius of a type
change is ~40 references across 10 files (model, merge, debug, API response types, test
fixtures). The data layer is already correct (both boss cells provisioned), so only the
membership test needs to change.

**Alternative considered — change `boss_cell_no: i64` → `boss_cell_nos: Vec<i64>`:**
rejected. Would require touching `merge.rs` (remap + last-non-zero-wins logic), `debug.rs`,
both API response structs (`sortie/mod.rs:123,159`), and ~15 test fixtures that hardcode a
scalar. Disproportionate to a two-site detection bug.

**Alternative considered — use `event_id == 5` as the boss test:** rejected as the primary
mechanism. `event_id=5` happens to coincide with boss nodes today, but `node_label`
equivalence is the semantically correct key (boss *node*, not boss *event*). `event_id`
remains a useful invariant to assert in tests (all cells sharing the boss label must have
`event_id=5`).

### KTD2. The helper is node-label-aware with a scalar fallback

**Decision:** `boss_cell_nos()` resolves the boss cell set as:

- If `boss_cell_no`'s cell has a non-empty `node_label`, return all cells sharing that label
  (via `multi_label_index`).
- Otherwise (no label — e.g. synthetic/skeleton variants), return `[boss_cell_no]`.

This guarantees R4 (single-incoming-route maps behave identically) and degrades safely when
labels are absent. The fallback also keeps existing tests that construct `MapVariantDefinition`
without labels working unchanged.

## High-Level Technical Design

```text
MapVariantDefinition::boss_cell_nos() -> Vec<i64>:
    let boss_label = self.cell(boss_cell_no).and_then(|c| c.node_label.clone());
    match boss_label.filter(|l| !l.is_empty()) {
        Some(label) => self.multi_label_index().get(&label).cloned().unwrap_or_else(|| vec![boss_cell_no]),
        None => vec![boss_cell_no],
    }
```

The two detection sites become:

- `mod.rs:596`: `let is_boss_cell = stage.boss_cell_nos().contains(&current_cell.cell_no);`
- `mod.rs:706`: `let should_finish_sortie = stage.boss_cell_nos().contains(&current_cell.cell_no) || ...;`

Note `mod.rs:596` currently compares against `active.boss_cell_id` (the per-sortie snapshot),
not `stage.boss_cell_no` directly. The snapshot is set from `stage.boss_cell_no` at start
(`mod.rs:320`); the helper should be invoked against the `stage` (which carries the full
`node_label`/`multi_label_index`), not the scalar snapshot. The implementer should confirm
whether to (a) keep comparing against the active-sortie scalar and only fix the membership
test to be label-aware, or (b) compare against `stage` directly — this is an execution-time
detail resolved by reading the two sites in context, not a planning blocker.

---

## Implementation Units

### U1. Add boss-cell equivalence helper to `MapVariantDefinition`

- **Goal:** Expose the full set of boss cells (all cells sharing the boss node's label) so
  callers can do a label-aware membership test.
- **Requirements:** R1, R3, R4.
- **Files:**
  - `crates/emukc_model/src/codex/map/types.rs` — add `boss_cell_nos()` next to
    `multi_label_index()` (line ~106) and `label_to_cell_no()` (line ~121).
- **Approach:** Implement per KTD2: resolve `boss_cell_no`'s `node_label`, return the
  multi-index bucket for that label, or `[boss_cell_no]` when the label is absent/empty.
  Keep the helper pure and cheap (the index is rebuilt per call; the callers are
  sortie-resolution paths, not hot loops).
- **Patterns to follow:** Mirror `multi_label_index()`'s style and doc comment. Both helpers
  already iterate `&self.cells`; `boss_cell_nos()` can call `multi_label_index()` rather than
  re-iterating.
- **Test scenarios:**
  - Happy path: a variant with two cells sharing label "E" (one of which is `boss_cell_no`)
    returns both cell numbers.
  - Single-boss variant (common case): a variant whose boss node has one cell returns a
    single-element `Vec` — and it equals `boss_cell_no` (R4 invariant).
  - Fallback: a variant whose `boss_cell_no` cell has `node_label = None` returns
    `[boss_cell_no]`.
  - Fallback: a variant whose `boss_cell_no` cell has `node_label = Some("")` (empty) returns
    `[boss_cell_no]`.
  - Ordering: the returned set is deterministic (sorted by `cell_no`) so callers can assert
    stable output.
- **Verification:** `cargo test -p emukc_model` green; new unit tests assert each scenario.

### U2. Make sortie boss detection label-aware

- **Goal:** Reaching any boss cell (not just `boss_cell_no`) is recognized as reaching the
  boss: sortie finishes, boss battle triggers, boss quest advances.
- **Requirements:** R1.
- **Dependencies:** U1.
- **Files:**
  - `crates/emukc_gameplay/src/game/sortie/mod.rs` — the two detection sites:
    - `is_boss_cell` (line ~596): `current_cell.cell_no == active.boss_cell_id` →
      `stage.boss_cell_nos().contains(&current_cell.cell_no)`.
    - `should_finish_sortie` (line ~706): same substitution in the first disjunct.
- **Approach:** Per KTD1 the helper is invoked against the `stage` (the
  `MapVariantDefinition` that carries `node_label`/`multi_label_index`). The
  `active.boss_cell_id` scalar stays for the client-facing response (`boss_cell_no` field)
  and any non-equivalence uses; only the membership test changes. Read both sites in context
  to confirm `stage` is in scope at each (it is resolved earlier in both functions).
- **Patterns to follow:** Both sites already resolve `stage` and `current_cell` via
  `stage.cell(...)`; follow the existing `ok_or_else`/`?` shape.
- **Test scenarios:**
  - Integration (existing, currently failing):
    `sortie_battle_result_advances_boss_quest_on_real_boss_node` — reaching cell 6 (node E)
    on map 1-2 now advances boss quest 204 to `Completed`. (This test is the regression
    target; it must turn green.)
  - Integration (existing, currently failing):
    `repo_wikiwiki_asset_supports_real_map_boss_progression` — same map, boss node reached
    via the cell-6 branch finishes the sortie and advances the quest.
  - Regression: a single-boss-node map (e.g. map 1-1) still finishes the sortie on the boss
    and does not finish it on a non-boss cell (R4 — no behavior change for the common case).
  - Boss battle still triggers: reaching either boss cell produces a boss battle response
    (enemy fleet resolved from the cell's own `enemy_fleets` entry; both boss cells are
    already provisioned).
- **Verification:** the two previously-failing tests pass; `cargo test --test sortie_battle`
  is fully green (15/15); `cargo test -p emukc_gameplay` green.

### U3. Update the in-repo `path_to_boss` test helper

- **Goal:** The test helper's DFS targets the boss *node*, not just the first boss cell, so
  it can find a boss-reachable path when the boss node is entered via any incoming edge.
- **Requirements:** R1.
- **Dependencies:** U1.
- **Files:**
  - `crates/emukc_gameplay/tests/sortie_battle.rs` — `path_to_boss` (line ~244) and the
    `while current != boss` loop in `advance_sortie_to_boss` (line ~306).
- **Approach:** Replace the single-`boss` target with the boss-cell set from U1:
  `let boss_cells = variant.boss_cell_nos();` and terminate the walk when
  `boss_cells.contains(&current)`. The candidate-sort heuristic
  (`path_to_boss(...).is_none()`) already calls `path_to_boss`, which itself does a DFS to a
  single `boss`; update its target to "any boss cell" the same way.
- **Test scenarios:**
  - `start_sortie_with_boss_path` no longer panics on map 1-2 (the helper succeeds when the
    walk enters node E via cell 6).
  - The returned `path.last()` is a boss cell (assert it's in `boss_cell_nos()`), preserving
    the `assert_eq!(start.boss_cell_no, *path.last().unwrap())` invariant — note this
    invariant may need to relax to "path ends on a boss cell" rather than the exact scalar;
    the implementer should confirm whether `boss_cell_no` (5) is always the *last* cell or
    whether the walk can legitimately end on cell 6.
- **Verification:** `start_sortie_with_boss_path` returns within its retry budget on map 1-2;
  the two tests in U2 that depend on it pass.

### U4. Regression test pinning equivalence semantics

- **Goal:** A dedicated unit test asserting that all cells sharing the boss node's label are
  recognized as boss, independent of the integration tests (which exercise the full sortie
  stack).
- **Requirements:** R1, R4.
- **Dependencies:** U1.
- **Files:**
  - `crates/emukc_model/src/codex/map/types.rs` — extend the test module with a
    `boss_cell_nos_returns_all_cells_sharing_boss_node_label` test using a hand-built variant
    mirroring map 1-2's node-E topology (two cells, label "E", one marked `boss_cell_no`).
- **Approach:** Construct a minimal `MapVariantDefinition` with two same-label boss cells and
  assert `boss_cell_nos()` returns both, sorted. Also assert that a single-boss-cell variant
  returns exactly `[boss_cell_no]` (R4).
- **Test scenarios:**
  - Two cells share boss label → both returned.
  - One boss cell (no duplicate) → single-element result equal to `boss_cell_no`.
  - Invariant assertion: every cell in `boss_cell_nos()` has `event_id == 5` (boss event),
    documenting the event/label coincidence without depending on it for detection.
- **Verification:** `cargo test -p emukc_model boss_cell_nos` green.

### U5. Verification sweep

- **Goal:** Confirm no regression from the equivalence change across the workspace.
- **Requirements:** R4.
- **Dependencies:** U1, U2, U3, U4.
- **Files:** n/a.
- **Approach:** Run the full workspace gates.
- **Test scenarios:**
  - `cargo test --workspace` — all suites green, including the 5 kcdata route→cell tests
    (proves the parser model was not touched — the revert lesson).
  - `cargo clippy --workspace -- -D warnings` — clean (use the strict gate, not the default;
    per the plan-004 U7 lesson).
  - `cargo fmt --all -- --check` — clean.
  - The `make_list` pre-existing failure is explicitly excluded (documented in
    PROJECT_MEMORY; unrelated to this work).
- **Verification:** all gates green; `tests/sortie_battle.rs` is 15/15.

## Risks & Dependencies

- **Risk: over-broad equivalence.** If a non-boss node legitimately shares a label with the
  boss node (data error or an unusual map), the helper would mark it boss. Mitigated by
  KTD2's derivation from `boss_cell_no`'s label (the equivalence is anchored to a known boss
  cell, not to any cell with a boss event). The U4 invariant assertion (`event_id == 5` for
  all returned cells) would catch such a data anomaly in tests.
- **Risk: the active-sortie scalar vs stage ambiguity (KTD1 note).** The detection sites
  compare against `active.boss_cell_id`; the helper needs `stage`. Resolved in U2 by reading
  both sites in context — `stage` is already resolved in both functions.
- **Risk: the `path_to_boss` last-cell invariant (U3).** `assert_eq!(start.boss_cell_no,
  *path.last().unwrap())` may break if the walk legitimately ends on the second boss cell.
  The implementer should relax this to a set-membership check if needed.
- **Dependency:** none blocking; U2/U3/U4 depend on U1 only. This plan supersedes the
  "codex topology error" diagnosis recorded earlier in PROJECT_MEMORY (that diagnosis was
  wrong — see the corrected entry).
