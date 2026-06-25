---
title: "fix: rashin_flg must key on branch-node out-degree, not fleet-resolved candidate count"
status: completed
created: 2026-06-24
type: fix
origin: user-reported 2-1 resource-cell compass skip + official server start/next captures (~/Downloads/kcsapi/)
---

# Fix: Compass (羅針盤) Skipped at 2-1 Resource Cells

## Problem Frame

On map 2-1, advancing to a resource cell (資源点, `color_no=2`) plays **no compass animation** — the client jumps straight to the next cell. The compass should spin whenever the departing cell is a branching node (分岐点).

`api_req_map/start` and `api_req_map/next` set `api_rashin_flg` / `api_rashin_id` from
`evaluate_route_candidate_count(...)` — the number of route candidates **after** the current fleet
and the priority routing rules are applied. At a branch node, the priority rules usually collapse to a
single winning target, so the candidate count is `1`, `rashin_flg` resolves to `false`, and the client
skips the compass. The official server instead keys `rashin_flg` purely on whether the **departing
cell is a physical branch node** (out-degree > 1), independent of which way this particular fleet is
forced to go.

The reported symptom is a special case: 2-1's resource cells (cell 2 and cell 5) are both children of
the branch node **cell 3** (`cell 3 → [2, 4, 5]`). Advancing into them must spin the compass, but the
fleet-resolved count collapses to 1, so we return `rashin_flg=0` and the compass is skipped.

note.md item 1 (CV cut-in showing one aircraft) was fixed by plan `2026-06-24-001` and is out of scope here.

---

## Evidence from Official Server Data

Captures in `~/Downloads/kcsapi/` (`start.txt`, `next.txt`, `next(2).txt`) are the oracle. Cross-referenced
against our local 2-1 topology in `.data/codex/map_catalog.json` (map 21):

| Advance | Departing cell → `next_cells` | physical out-degree | official `api_rashin_flg` | current code yields |
|---|---|---|---|---|
| start → cell 3 | cell 0 → `[3]` | 1 | 0 | 0 (incidentally correct) |
| cell 3 → cell 4 | cell 3 → `[2,4,5]` | 3 | **1** | resolves to 1 → **0 (wrong)** |
| cell 4 → boss 8 | cell 4 → `[8]` | 1 | 0 | 0 |

All three official samples match the rule **`rashin_flg = (departing cell out-degree > 1)`**. The
fleet-resolved-candidate rule fails the branch-node case.

2-1 resource cells confirmed from `start.txt` `api_cell_data`: cell 2 and cell 5 carry `color_no=2`, and
both are reached only from branch node cell 3.

**Direct-confirmation gap (honest):** none of the three captures lands *on* a resource cell, so the
official `rashin_flg` for a resource-cell advance is inferred ("its only parent is the branch node cell 3"),
not read directly. The inference is consistent with all three samples under the out-degree rule.

---

## Root Cause

`crates/emukc_gameplay/src/game/sortie/mod.rs`:

- `start_sortie` (~L333-342) sets `rashin_flg`/`rashin_id` from `evaluate_route_candidate_count(source_cell, ...)`.
- `next_sortie` (~L455-463) sets them from `evaluate_route_candidate_count(current, ...)`.

`evaluate_route_candidate_count` (`crates/emukc_gameplay/src/game/map_route.rs:90-148`) intentionally
applies the fleet's predicate/priority filtering and collapses a branch to its winning target(s). That
is the wrong signal for the compass. An earlier investigation hypothesized "compute from the *next*
cell's out-degree" — the captures **refute** that (boss cell 8 has zero out-edges yet is reached with
`rashin_flg=0`, consistent only with the *departing* cell's out-degree).

---

## Requirements

- R1. `api_rashin_flg` is `1` iff the departing cell has more than one physical outgoing edge
  (`next_cells.len() > 1`); otherwise `0`. Applies to both `start` (departing = start/source cell) and
  `next` (departing = current cell).
- R2. `api_rashin_id` stays `1` when `rashin_flg=1`, else `0` (matches `next.txt`).
- R3. Advancing into 2-1 resource cells (2, 5) from branch node cell 3 yields `rashin_flg=1`.
- R4. The three official captures (start→3, 3→4, 4→8) are reproduced exactly for `rashin_flg`/`rashin_id`.

---

## Key Technical Decisions

- **KTD1 — Key `rashin_flg` on departing-cell physical out-degree.** Use `cell.next_cells.len() > 1`
  rather than the fleet-resolved candidate count. This is the canonical KanColle 分岐点 semantic and is
  the only rule consistent with all three official captures. A one-line expression; no helper needed,
  though a small `cell_is_branch(cell)` predicate may be added in `map_route.rs` for readability/reuse.
- **KTD2 — Remove `evaluate_route_candidate_count` once unused.** It was added specifically to drive
  `rashin_flg` (per its own doc comment) and has no other production caller (verified by grep — only the
  two rashin sites and its own unit tests reference it). Leaving it invites re-wiring the refuted logic.
  This deletion is caused directly by KTD1, so it is in-scope, not opportunistic cleanup.

---

## Implementation Units

### U1. Key `rashin_flg` / `rashin_id` on departing-cell out-degree

**Goal:** Replace the fleet-resolved candidate-count signal with physical branch detection in both sortie entry points.

**Requirements:** R1, R2, R3, R4

**Dependencies:** none

**Files:**
- `crates/emukc_gameplay/src/game/sortie/mod.rs` (modify `start_sortie` and `next_sortie`)
- `crates/emukc_gameplay/src/game/map_route.rs` (optional: add `cell_is_branch` predicate)

**Approach:**
- In `start_sortie`, compute the flag from `source_cell.next_cells.len() > 1`.
- In `next_sortie`, compute the flag from `current.next_cells.len() > 1`.
- Keep `rashin_id` shape: branch → `1`, else `0`.
- If adding `cell_is_branch(&MapCellDefinition) -> bool` in `map_route.rs`, both call sites use it.

**Patterns to follow:** mirror the existing `cell_has_routing_outgoing(...)` helper already used for `has_next` in the same module — same shape, reads off `next_cells`.

**Test scenarios:** covered by U3 (integration). No unit-level test added here beyond U3.

**Verification:** `next_sortie`/`start_sortie` return `rashin_flg=true` exactly when the departing cell has >1 `next_cells`; the three official 2-1 advances reproduce (see U3).

### U2. Remove the now-dead `evaluate_route_candidate_count`

**Goal:** Delete the superseded function and its tests so the refuted logic cannot be re-wired.

**Requirements:** KTD2 (housekeeping caused by U1)

**Dependencies:** U1

**Files:**
- `crates/emukc_gameplay/src/game/map_route.rs` (delete fn at ~L90-148; delete its unit tests `evaluate_route_candidate_count tests` ~L2081-2195)
- `crates/emukc_gameplay/src/game/sortie/mod.rs` (drop the import at ~L61)

**Approach:** Remove function, its `#[cfg(test)]` cases, and the unused import. Confirm no remaining
references with a workspace grep before deleting.

**Test scenarios:** Test expectation: none — pure removal of now-dead code; coverage comes from the
existing `emukc_gameplay` suite staying green after the import/fn are gone.

**Verification:** `cargo build -p emukc_gameplay` and `cargo clippy --workspace -- -W warnings` clean (no
unused-import/dead-code warnings); `cargo test -p emukc_gameplay` green.

### U3. Regression test against official compass semantics

**Goal:** Lock in branch-node compass behavior using the 2-1 captures as the oracle.

**Requirements:** R1, R3, R4

**Dependencies:** U1

**Files:**
- `tests/gameplay_tests/map/compass.rs` (new)
- `tests/gameplay_tests/map/mod.rs` (register the module)

**Approach:** Reuse the existing `start_sortie` / `next_sortie` integration harness (see
`tests/gameplay_tests/map/non_boss_pending.rs`, `unlock.rs`). Sortie 2-1 (unlock prerequisite first, per
`unlock.rs`'s pattern). Assert the flag is a pure function of the departing cell's out-degree.

**Test scenarios:**
- Covers R4. `start_sortie` on 2-1 lands on cell 3 (from start cell 0, out-degree 1) → `rashin_flg == false`, `rashin_id == 0`.
- Covers R3. Advancing one step from branch node cell 3 (out-degree 3) → `rashin_flg == true`, `rashin_id == 1`, **regardless** of which child {2,4,5} the routing picks — this directly exercises the resource-cell case (cell 2 / cell 5).
- Covers R4. Advancing from cell 4 (out-degree 1, `→[8]` boss) → `rashin_flg == false`.
- Edge: a cell with exactly one `next_cells` entry never yields `rashin_flg=true`.

**Fallback (surface explicitly if used):** if unlocking 2-1 in-test proves heavy, assert the same
out-degree rule on the first available branch-containing map and state in the PR why 2-1 was not used
(per the project's "no silent skips" rule).

**Verification:** `cargo test --test gameplay_tests` green, including the new `compass` module.

### U4. Capture the rashin_flg semantic as institutional knowledge

**Goal:** Prevent regression and permanently refute the "compute from next cell" misconception.

**Requirements:** documents R1

**Dependencies:** U1

**Files:**
- `docs/solutions/architecture-patterns/sortie-compass-rashin-flag.md` (new)

**Approach:** Record: `rashin_flg` keys on the *departing* cell's physical out-degree (分岐点), not on the
fleet-resolved candidate count and not on the *next* cell's out-degree. Cite the three official captures
and the 2-1 topology table. Cross-link to the sortie/map_route source and to plan
`2026-06-24-001` (the sibling map-data-vs-official-capture fix).

**Test scenarios:** Test expectation: none — documentation.

**Verification:** doc renders, frontmatter matches the `docs/solutions/architecture-patterns/` convention, links resolve.

---

## Scope Boundaries

### In Scope
- `rashin_flg` / `rashin_id` computation in `start_sortie` and `next_sortie`.
- Removal of the superseded `evaluate_route_candidate_count`.
- Regression test + learning capture.

### Out of Scope
- Map routing *destination* logic (`evaluate_route_destination`) — unchanged; only the compass flag is wrong.
- Resource-gain / non-battle node effects (`resolve_non_battle_node_effect`) — already produce `itemget`/`happening` correctly.
- `api_rashin_id` values beyond {0,1} (official captures only show 0/1).

### Deferred to Follow-Up Work
- **`api_ration_flag` response-field gap.** Official `next(2).txt` carries `api_ration_flag:0`; our
  `KcApiMapNext` (`crates/emukc_model/src/kc2/api/map.rs:84-118`) does not emit it (the field exists only
  on the sortie-battle *request* side). Semantics (給糧艦/ration nodes; when present vs. omitted) are not
  yet understood, so it is kept separate from this clean fix.
- **Direct resource-cell capture.** To convert the R3 inference into a directly-read fact, capture one
  `api_req_map/next` that lands on 2-1 cell 2 or 5 and confirm its `api_rashin_flg`.

---

## Risks & Dependencies

- **R-Risk1 — out-degree ≠ branch for some maps.** A node could carry `next_cells.len() > 1` where one
  edge is unreachable for all fleets. Real KC still spins the compass at such nodes (it points to the
  forced direction), so `next_cells.len() > 1` is the intended rule. Mitigation: U3's assertion is on the
  topological out-degree, matching the captures.
- **R-Risk2 — `next_cells` completeness.** The rule assumes `next_cells` is the authoritative physical
  out-edge set. Confirmed populated for 2-1 (cell 3 → `[2,4,5]`). If any map relies solely on
  `routing_rules` with empty `next_cells`, that map would never spin — note as a watch item; out of scope
  unless a capture contradicts it.
- No external dependencies; change is internal to `emukc_gameplay` plus a model-side doc.

---

## Verification

```bash
cargo test --test gameplay_tests        # includes new map::compass regression (U3)
cargo test -p emukc_gameplay            # green after evaluate_route_candidate_count removal (U2)
cargo fmt --all --check
cargo clippy --workspace -- -W warnings # no dead-code/unused-import warnings from U2
```

End-to-end (optional): `cargo run -- serve`, sortie 2-1, advance from cell 3 to a resource cell (2/5),
confirm the compass spins. Oracle: `~/Downloads/kcsapi/{start,next,next(2)}.txt`.
