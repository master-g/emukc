---
module: gameplay
tags: [sortie, map-routing, rashin-flg, compass, branch-node]
problem_type: behavioral_contract
created: 2026-06-24
---

# Sortie Compass (`api_rashin_flg`) Keys on Departing-Cell Out-Degree

## Context

`api_req_map/start` and `api_req_map/next` carry `api_rashin_flg` / `api_rashin_id`,
which drive the compass (羅針盤) spin animation on the client. The animation must
play whenever the **departing** cell is a branch node (分岐点).

An earlier implementation set the flag from `evaluate_route_candidate_count(...)` —
the number of route candidates left **after** the fleet's predicate/priority routing
rules collapse a branch to its winning target(s). At a real branch node the priority
rules usually resolve to a single forced target, so the count is `1`, the flag is
`false`, and the client skips the compass.

User-reported symptom: on map 2-1, advancing into a resource cell (資源点,
`color_no=2`) played no compass animation. The resource cells (2, 5) are both children
of branch node cell 3 (`cell 3 → [2, 4, 5]`); the fleet-resolved count collapsed to 1
and suppressed the spin.

## The Rule

```
api_rashin_flg = (departing_cell.next_cells.len() > 1)   // physical out-degree
api_rashin_id  = if rashin_flg { 1 } else { 0 }
```

- `start`: departing cell = the start/source cell.
- `next`: departing cell = the current cell (NOT the destination).

The flag is a pure function of the departing cell's **physical** out-degree. It does
**not** depend on which way this particular fleet is forced to go, and it does **not**
depend on the *next* cell's out-degree.

## Why (oracle)

Official captures in `~/Downloads/kcsapi/` (`start.txt`, `next.txt`, `next(2).txt`),
cross-referenced against local 2-1 topology in `.data/codex/map_catalog.json`:

| Advance | Departing cell → `next_cells` | out-degree | official `rashin_flg` |
|---|---|---|---|
| start → cell 3 | cell 0 → `[3]` | 1 | 0 |
| cell 3 → cell 4 | cell 3 → `[2,4,5]` | 3 | 1 |
| cell 4 → boss 8 | cell 4 → `[8]` | 1 | 0 |

All three match `rashin_flg = (departing out-degree > 1)`. Two refuted hypotheses:

- **fleet-resolved candidate count** — fails the `cell 3 → cell 4` branch case (collapses to 1).
- **compute from the *next* cell's out-degree** — fails `cell 4 → boss 8`: boss cell 8
  has zero out-edges yet is reached with `rashin_flg=0`, consistent only with the
  *departing* cell's out-degree.

## Where

- `crates/emukc_gameplay/src/game/sortie/mod.rs` — `start_sortie` and `next_sortie`
  compute `departing_is_branch = <cell>.next_cells.len() > 1`.
- The old `evaluate_route_candidate_count` in `map_route.rs` was deleted with this fix;
  it existed only to drive the (refuted) candidate-count rule.

Regression coverage: `tests/gameplay_tests/map/compass.rs`. It uses map 1-1's branch
node cell 1 (`→[2,3]`) as the stand-in for 2-1's branch node (unlocking 2-1 in-test is
heavy); advancing from cell 1 reproduces the resource-cell-from-branch structure.

## Related

- `map-data-authority.md` — map topology comes from the codex catalog; sibling fix
  plan `2026-06-24-001` (map-data vs official-capture).
- Known residual gap: `api_ration_flag` (present in `next(2).txt`) is not yet emitted
  by `KcApiMapNext`; semantics (給糧艦/ration nodes) are not understood — deferred.
