---
title: "fix: Sortie State Cleanup and Map 1-3 Routing — Defensive Start, Practice Leak Prevention, Directed-Graph Routing"
status: active
type: fix
date: 2026-06-22
origin: openspec/changes/fix-sortie-state-and-routing (translated during openspec sunset)
---

# fix: Sortie State Cleanup and Map 1-3 Routing

## Summary

Two related sortie defects, one shared root cause and one data/logic question:

1. **State leak at sortie start.** Starting a new sortie does not defensively
   clear residual state from a previous sortie or practice session. If the
   previous session did not cleanly exit (client crash, missing `goback_port`),
   stale `SortieStore` entries persist — causing corrupted enemy compositions
   (practice opponents bleeding into sortie battles) and skipping ahead to boss
   nodes.
2. **Practice → sortie cross-contamination.** The practice battle result path
   may leave `pending_battle`/`pending_result` in the store after processing,
   so a subsequent sortie inherits practice enemy data.
3. **Map 1-3 routing drift.** 1-3 routing does not follow the directed graph
   edges defined in codex map data — either the codex `next_cells`/
   `routing_rules` are wrong, or the fallback in `select_route_from_cells`
   picks `next_cells[0]` deterministically instead of following valid edges.

This plan fixes all three: defensive cleanup at sortie start, a practice
cleanup audit, and a data-vs-code investigation of 1-3 routing.

## Reconciliation (2026-06-22)

**Read-only verification of all 10 checkboxes against current code.**

| Unit | Total | Done | Not Done |
| --- | --- | --- | --- |
| U1 Investigation | 2 | 1 | 1 |
| U2 Sortie State Cleanup | 4 | 3 | 1 |
| U3 Map 1-3 Routing | 4 | 1 | 3 |
| **Total** | **10** | **5** | **5** |

**Key structural change since plan was written:** the `PracticeRepository` extraction (plan 004) moved practice sessions into a separate `GLOBAL_PRACTICE_STORE`, making practice → sortie cross-contamination structurally impossible. This obsoletes the root concern behind U2.2 and U2.4.

**Genuinely remaining work (5 tasks):**

1. **U1.2** — Investigate 1-3 codex map data vs authoritative source. Likely moot since 3.2 is done, but worth confirming the data is correct.
2. **U2.4** — Practice → sortie leak test. Low value as a regression guard since the concern is structurally eliminated, but still missing as test coverage.
3. **U3.1** — Fix 1-3 codex data if investigation finds it wrong. Depends on U1.2.
4. **U3.3** — Add a test asserting 1-3 routing follows valid directed-graph edges. This is the most valuable remaining task.
5. **U3.4** — Run integration test pass. Trivial once U3.3 is added.

**Estimated remaining effort: small.** The heavy lifting (defensive cleanup, practice leak prevention, routing fallback fix) has already shipped. What remains is mostly test coverage + a data sanity check.

The `SortieStore` (and its repository traits) hold state keyed by `profile_id`:
the active sortie, pending battles, and pending results. Practice and sortie
sessions share the store. When `start_sortie` runs, it inserts into the active
slot **without first clearing any existing entry**. A previous session that
exited uncleanly leaves stale state behind, and the new sortie inherits it.

Two failure modes follow:

- **Enemy composition corruption.** A leftover practice `pending_battle` (which
  carries the practice opponent fleet) can surface as the enemy fleet of a new
  sortie. This is the strongest reported symptom that practice sessions leak
  into sorties.
- **Node skipping.** A leftover active sortie (with its current cell) can make
  the new sortie resume mid-map rather than starting at the origin cell,
  producing the reported "skips ahead to boss node" behavior.

For routing, `evaluate_route_destination` consumes `routing_rules` from codex
map data. When rules are absent it falls back to `select_route_from_cells`,
which reads `current.next_cells`. On 1-3, the fallback appears to select
`next_cells[0]` deterministically — either because `next_cells` is mis-ordered
in the codex (data bug) or because the fallback ignores multi-edge semantics
(code bug). The investigation (U1) determines which.

## Requirements

**State cleanliness**

- **R1.** Starting a sortie for a profile removes any existing active sortie,
  pending battle, and pending result for that profile from the store before
  creating new state, regardless of whether the previous session exited
  cleanly.
- **R2.** A practice battle that completes has its `pending_battle` and
  `pending_result` removed from the store after result processing, so no
  practice enemy data survives into a subsequent sortie.

**Routing correctness**

- **R3.** Map 1-3 cell transitions correspond only to valid directed-graph
  edges in the codex map data; the fleet never skips cells or jumps to
  non-adjacent cells.
- **R4.** When a cell has no `routing_rules` and multiple `next_cells`, the
  fallback selects from valid adjacent cells only and does not deterministically
  pick the first cell when multiple valid options exist.

**Verification**

- **R5.** Integration tests prove a sortie started after an incomplete previous
  sortie carries no stale state, and a sortie started after a practice carries
  no practice enemy data.

## Non-goals

- Changing the routing algorithm itself (`evaluate_route_destination` core
  logic) — only the fallback `select_route_from_cells` multi-edge handling, and
  only if the investigation (U1) rules out a data bug.
- Changing `SortieStore` architecture or its repository-trait design.
- Map data for maps other than 1-3, unless the investigation reveals the
  problem is in data generation rather than 1-3-specific.
- Re-implementing `start_sortie`'s fleet/map validation (unavailable fleet,
  invalid map, sunk ships, locked map) — those already exist and stay.

## Key Technical Decisions

- **KTD1. Defensive cleanup at sortie start (design D1).** In the
  `start_sortie` path (`crates/emukc_gameplay/src/game/sortie/mod.rs`,
  `start_sortie_impl`), call `remove_active_sortie`, `take_pending_result`, and
  `take_pending_battle` for the profile **before** creating the new active
  sortie state. This is defense-in-depth: even if `goback_port` should have
  cleaned up, the start handler guarantees no stale state survives. This
  matches real server behavior where each sortie is independent. An unclaimed
  result from a previous sortie is already invalid, so discarding it is correct.
- **KTD2. Practice cleanup audit (design D2).** Audit the practice battle
  result handler (`practice_battle_result` in
  `crates/emukc_gameplay/src/game/practice.rs`) to confirm it clears
  `pending_battle` and `pending_result` after processing. A
  `clear_pending_battle` call already exists at `practice.rs`; the audit
  verifies the result path is symmetrically cleaned. If the result-side
  cleanup is missing, add it. The reported practice-opponents-in-sortie symptom
  is the evidence that practice sessions leave residual data in the shared
  store.
- **KTD3. 1-3 routing investigation before fix (design D3).** First inspect
  the codex map data for 1-3 (`next_cells`, `routing_rules`) against an
  authoritative source (wikiwiki). If the data is correct, the bug is in the
  fallback logic `select_route_from_cells`
  (`crates/emukc_gameplay/src/game/map_route.rs`); if the data is wrong, the
  fix targets the codex/bootstrap pipeline. The fix location depends on the
  diagnosis — the plan does not presuppose a code-only or data-only fix.

## High-Level Technical Design

```
start_sortie(profile, fleet, map)
  ├── [NEW] remove_active_sortie(profile)      // discard stale sortie
  ├── [NEW] take_pending_result(profile)        // discard unclaimed result
  ├── [NEW] take_pending_battle(profile)        // discard stale battle
  ├── validate fleet / map / unlocked
  ├── consume fuel + ammo
  └── add_active_sortie(new state)              // clean insert

practice_battle_result(profile)
  ├── process result, compute rewards/exp
  ├── [AUDIT] clear_pending_battle(profile)     // already present
  └── [AUDIT] clear/take_pending_result(profile) // verify present, add if missing

evaluate_route_destination(map, current_cell, ...)
  ├── routing_rules present?  → apply rule
  └── absent → select_route_from_cells(next_cells)   // U3: fix if fallback at fault
```

The cleanup is three calls at the top of `start_sortie_impl`, ahead of the
existing validation. The routing fix is conditional on the U1 diagnosis.

## Implementation Units

### U1. Investigation

- **Goal:** Determine whether the 1-3 routing bug is a codex data error or a
  fallback-logic error, and confirm the practice result-handler cleanup gap.
- **Requirements:** informs R2, R3.
- **Files:**
  - `crates/emukc_gameplay/src/game/practice.rs` — read
    `practice_battle_result` to verify `pending_battle`/`pending_result`
    cleanup.
  - Codex map data for map 1-3 (`.data/codex/map_catalog.json` and the codex
    `MapVariantDefinition`) — verify `next_cells` and `routing_rules` against
    wikiwiki.
- **Tasks:**
- [x] 1.1 Read practice battle result handler to verify pending_battle/pending_result cleanup *(practice.rs:172 take_pending_result, :185 clear_pending_battle; practice now uses separate PracticeStore via GLOBAL_PRACTICE_STORE — cross-contamination structurally eliminated)*
- [ ] 1.2 Inspect codex map data for map 1-3: verify next_cells and routing_rules against wikiwiki *(may be moot — code-side routing fallback was already fixed in 3.2)*

### U2. Sortie State Cleanup

- **Goal:** Guarantee clean state at sortie start and prevent practice → sortie
  cross-contamination.
- **Requirements:** R1, R2, R5.
- **Dependencies:** U1 (1.1 informs whether 2.2 needs a fix at all).
- **Files:**
  - `crates/emukc_gameplay/src/game/sortie/mod.rs` — `start_sortie_impl`
    defensive cleanup.
  - `crates/emukc_gameplay/src/game/sortie_store.rs` —
    `remove_active_sortie`/`take_pending_result`/`take_pending_battle` exist
    and are reused.
  - `crates/emukc_gameplay/src/game/practice.rs` — `practice_battle_result`
    cleanup, if the U1 audit finds it missing.
- **Tasks:**
- [x] 2.1 Add defensive cleanup at start of `start_sortie_impl`: remove_active_sortie + take_pending_result + take_pending_battle before inserting new state *(clear_pending_sortie_runtime_state at sortie/mod.rs:327, called inside with_profile_lock before insert_active)*
- [x] 2.2 Fix practice battle result handler to clear pending_battle and pending_result from SortieStore after processing (if missing) *(practice.rs:172+185 clears both; practice now uses separate GLOBAL_PRACTICE_STORE, not SortieStore — leak path eliminated)*
- [x] 2.3 Add test: start sortie after incomplete previous sortie — verify no stale state *(tests/gameplay_tests/map/non_boss_pending.rs:76 start_sortie_twice_clears_previous_state; tests/gameplay_tests/map/unlock.rs:96 start_sortie_after_incomplete_previous_sortie_succeeds)*
- [ ] 2.4 Add test: start sortie after practice — verify no practice enemy data leaks *(concern structurally eliminated by separate stores; test would be a regression guard only)*
- **Verification:** the two new tests pass; `cargo test --test gameplay_tests`
  stays green.

### U3. Map 1-3 Routing Fix

- **Goal:** Make 1-3 routing follow directed-graph edges, fixing the node-skip.
- **Requirements:** R3, R4.
- **Dependencies:** U1 (1.2 decides whether the fix is data or code).
- **Files:**
  - Codex map data for 1-3 (`.data/codex/map_catalog.json`) — corrected
    `next_cells`/`routing_rules` if data is at fault.
  - `crates/emukc_gameplay/src/game/map_route.rs` —
    `select_route_from_cells` multi-edge handling if the fallback is at fault.
- **Tasks:**
- [ ] 3.1 Fix 1-3 codex map data if next_cells/routing_rules are incorrect (depends on 1.2 findings) *(likely moot — code-side fix in 3.2 addresses multi-edge fallback; 1-3-specific data investigation still pending)*
- [x] 3.2 If routing logic is at fault, fix select_route_from_cells to handle multi-edge cells correctly *(git history shows next_cells[0] deterministic selection replaced with rng::usize(0..len) random selection; topology filtering via next_cells.contains already present)*
- [ ] 3.3 Add test: 1-3 sortie routing follows valid edges only
- [ ] 3.4 Run `cargo test --test gameplay_tests` for integration pass
- **Verification:** the 1-3 routing test passes against an authoritative edge
  list; the full integration suite is green.

## Behavioral notes

This change carries two specs deltas from its openspec origin. After the
openspec sunset migration, the living contracts now live as captured knowledge
under `docs/solutions/`:

- **Sortie state machine (MODIFIED)** — the `sortie` capability's "Sortie
  start" scenario gains the cleanup invariant (existing active sortie, pending
  result, and pending battle removed before creating new state), and a new
  "Practice session does not leak into sortie" scenario. See
  `docs/solutions/architecture-patterns/sortie.md`.
- **Map routing data validation (ADDED, pathrules-loading)** — codex `next_cells`
  SHALL reflect the real directed graph; fallback routing SHALL select from
  valid adjacent cells only. See
  `docs/solutions/best-practices/pathrules-loading.md`.
- **Map 1-3 routing follows directed graph (ADDED)** — 1-3 transitions SHALL
  correspond to valid edges only. Captured under the same
  `docs/solutions/architecture-patterns/sortie.md` (sortie/routing contract) and
  `docs/solutions/best-practices/pathrules-loading.md`.

## Acceptance / Done

- A1. U1–U3 landed; `cargo test --test gameplay_tests` green.
- A2. A sortie started after an incomplete previous sortie carries no stale
  state (U2.3 test passes).
- A3. A sortie started after a practice carries no practice enemy data (U2.4
  test passes).
- A4. 1-3 routing follows valid directed-graph edges only (U3.3 test passes).
- A5. The 1-3 fix is recorded as a data fix or a code fix per the U1 diagnosis.

## Risks & Dependencies

- **Over-cleanup at sortie start.** Clearing state at start could lose an
  unclaimed battle result. Acceptable: an unclaimed result from a previous
  sortie is already invalid once a new sortie begins.
- **1-3 data vs code.** If the issue is in codex data generation (bootstrap /
  decoder pipeline), the fix may need to extend beyond a one-line codex patch.
  Scope limit: fix the 1-3 data manually if needed, and address a broader
  pipeline problem as a separate change if the investigation reveals one.
- **Practice handler already partially cleaned.** A `clear_pending_battle` call
  already exists in `practice.rs`. U1.1 must confirm whether the result side is
  symmetrically cleaned; if it already is, U2.2 is a no-op (marked done on
  confirmation).

## Sources

- Origin openspec change: `openspec/changes/fix-sortie-state-and-routing/`
  (proposal.md, design.md, tasks.md, specs/{sortie,pathrules-loading}/).
- `crates/emukc_gameplay/src/game/sortie/mod.rs` — `start_sortie` /
  `start_sortie_impl`.
- `crates/emukc_gameplay/src/game/sortie_store.rs` — `remove_active_sortie`,
  `take_pending_result`, `take_pending_battle`.
- `crates/emukc_gameplay/src/game/practice.rs` — `practice_battle_result`,
  `clear_pending_battle`.
- `crates/emukc_gameplay/src/game/map_route.rs` — `evaluate_route_destination`,
  `select_route_from_cells`.
- `docs/solutions/architecture-patterns/sortie.md` — migrated sortie state
  machine contract.
- `docs/solutions/best-practices/pathrules-loading.md` — migrated routing data
  validation contract.
