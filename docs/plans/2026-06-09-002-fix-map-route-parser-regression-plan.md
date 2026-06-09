---
title: "fix: Repair wikiwiki route-parser regression introduced by c5f9fb6 (probability complement + enemy guard)"
type: fix
status: completed
date: 2026-06-09
---

# fix: Wikiwiki route-parser regression (c5f9fb6)

## Summary

Commit `c5f9fb6 fix(map): … generalize probability complement, relax enemy formation guard`
introduced two parser regressions in `emukc_bootstrap`'s wikiwiki map parser. While
generalizing the probability-complement derivation from 2-way to N-way junctions, it
broke the simple 2-way complement case; and while relaxing the enemy-table formation
guard, it over-relaxed and now emits spurious warnings on truly non-battle rows.

This plan fixes both **forward** (preserving the legitimate features `c5f9fb6` added),
restoring 3 failing fixture tests, then re-measures whether the downstream 7-3 sortie
tests recover as a cascade.

**Scope is Group A only** — the sharp, code-only, fixture-reproducible regression. The
two `map_pipeline::verify` capture failures (Group B) and the data-coupled parts of the
7-3 gameplay failures (Group C) are out of scope (see Scope Boundaries).

---

## Problem Frame

`cargo test -p emukc_bootstrap` has 5 failures on `feat/vibe`. Verification (running the
suite at `c5f9fb6~1` vs `c5f9fb6`) partitions them:

- **Pinned to `c5f9fb6`** (passed at `c5f9fb6~1`, fail at `c5f9fb6` and HEAD):
  - `parser::wikiwiki_map::tests::parse_fixture_catalog_with_probability_routes`
  - `parser::wikiwiki_map::tests::parse_fixture_catalog_with_probability_routes_ignores_route_footnote_anchor`
  - `parser::wikiwiki_map::tests::parse_enemy_table_skips_non_battle_rows_without_formations`
- **Predate `c5f9fb6`** (already failing at `c5f9fb6~1`; data/capture-sensitive, out of scope):
  - `map_pipeline::verify::tests::real_game_cells_match_catalog_cell_no_and_color`
  - `map_pipeline::verify::tests::map_1_3_real_start_capture_matches_route_cell_topology`

Two distinct `c5f9fb6` mechanisms:

1. **Probability complement.** `postprocess_route_probabilities` derives the
   complementary probability for an unknown route target (e.g. an explicit
   `FleetSize 6 → cell 3 @ 55%` should yield a derived `FleetSize 6 → cell 2 @ 45%`).
   `c5f9fb6` changed the junction guard `targets.len() != 2` → `< 2` and rewrote the
   placeholder/`prob_sum` derivation to handle N-way junctions. The rewrite no longer
   produces the 2-way complement the fixtures expect. The failing assertions are
   original (added in `5be5151`), so they are the canonical spec.

2. **Enemy formation guard.** `c5f9fb6` deleted the `enemy.rs` guard
   `if formation.is_none() && normalized_pattern.is_empty() { continue; }` to "retain
   enemy rows with ships but blank formation". It over-relaxed: truly non-battle rows
   (no ships *and* no formation) now fall through and emit a warning, tripping
   `assert!(warnings.is_empty())`.

These regressions also plausibly feed the 7-3 (map 73) sortie failures (Group C), whose
P-unlock route variants depend on probability-route parsing — to be re-measured after
the fix rather than assumed.

---

## Requirements

- **R1** — The 2-way probability complement is restored: an explicit probability on one
  target of a 2-target junction derives the complementary probability on the other.
  `parse_fixture_catalog_with_probability_routes` and `…_ignores_route_footnote_anchor`
  pass, with their original assertions unchanged.
- **R2** — The N-way junction complement behavior `c5f9fb6` added is preserved. No
  currently-passing parser/variant-key test regresses (e.g. gauge-3+ variant keys,
  executable-predicate and case-AST fixtures stay green).
- **R3** — Truly non-battle enemy rows (no ships and no formation) are skipped without
  warnings, while rows with ships but blank formation are still retained (the `c5f9fb6`
  intent). `parse_enemy_table_skips_non_battle_rows_without_formations` passes.
- **R4** — No net-new failures: every test passing in `cargo test -p emukc_bootstrap`
  before this change still passes after.
- **R5** — After R1–R3 land, the 7-3 gameplay tests (Group C) and the two verify tests
  (Group B) are re-run and the cascade outcome is recorded (which recover, which remain).
  Fixing B and the data-coupled part of C is **not** required by this plan.

---

## Key Technical Decisions

- **KTD-1 — Fix forward, do not revert `c5f9fb6`.** That commit also shipped wanted
  features: gauge-3+ / arabic-numeral variant keys (`extract_gauge_variant_key`),
  N-way junction complement, and `verify.rs` hardening. A revert loses them. Repair the
  over-generalization in place.
- **KTD-2 — Characterization-first; the failing assertions are the spec.** They were
  added in `5be5151` and passed until `c5f9fb6`. Make the code satisfy them; do not edit,
  weaken, or delete the assertions to go green. Confirm the expected behavior against the
  fixture HTML in `tests.rs`, not by reshaping the test.
- **KTD-3 — Narrow the enemy guard, don't restore it wholesale.** Re-introduce a skip
  that fires only when a row has **neither** ships **nor** formation (truly non-battle),
  preserving `c5f9fb6`'s "ships + blank formation are retained" behavior. Restoring the
  original `formation.is_none() && pattern.is_empty()` guard verbatim may re-break what
  `c5f9fb6` intended to fix — verify against both the failing test and any enemy-retention
  test added in `c5f9fb6`.

---

## Implementation Units

### U1. Restore 2-way probability complement without losing N-way

**Goal** — Repair `postprocess_route_probabilities` so a 2-target junction with one
explicit probability derives the complement on the other target, while the N-way
generalization keeps working.

**Requirements** — R1, R2

**Dependencies** — none

**Files**
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/route_condition.rs`
  (`postprocess_route_probabilities`)
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs` (existing fixtures — do not
  weaken assertions; add an N-way preservation case only if one is not already covered)

**Approach**
- Characterize first: run the two failing fixture tests and trace why the derived
  `FleetSize 6 → cell 2 @ 45%` rule is no longer produced. Likely candidates: the
  cell-2 target is not flagged as a `random_placeholder`, so `placeholders.len() != 1`
  short-circuits at the current guard; or `prob_sum` now aggregates across rules that
  the 2-way path previously isolated; or the derived rule's predicate diverges from the
  expected `FleetSize{6}`.
- Compare the current logic against the `c5f9fb6~1` version of the function
  (`git show c5f9fb6~1:…/route.rs`) to see what the 2-way path did before the rewrite.
- Fix forward so both the 2-way and N-way (single-unknown) cases derive the complement
  with the placeholder's predicate preserved. Keep `prob_sum >= 100` and
  `placeholders.len() != 1` as the genuine no-derive guards.

**Execution note** — Characterization-first: the two failing fixture tests are the
canonical spec; make them pass without changing their assertions.

**Patterns to follow** — the existing `RouteRuleDraft` derivation already in
`postprocess_route_probabilities`; mirror how `predicate` and `from_label`/`to_label`
are cloned onto the derived rule.

**Test scenarios**
- 2-way junction, one explicit `55%` + one placeholder → derived complement `45%` on the
  other target, predicate preserved (`FleetSize{6}`). (the two failing fixtures — restore)
- Footnote-anchor variant of the same fixture parses identically (the `…_ignores_route_footnote_anchor` case).
- N-way junction (3+ targets) with a single unknown → complement still derived for the
  one unknown (preserve `c5f9fb6`'s feature; add a focused case if not already present).
- Multiple unknowns at one source → no complement derived (ambiguous; existing guard).
- `prob_sum >= 100` → no complement derived (existing guard).

**Verification** — `parse_fixture_catalog_with_probability_routes` and
`…_ignores_route_footnote_anchor` pass; no other `parser::wikiwiki_map` test regresses.

---

### U2. Narrow the enemy-table non-battle-row guard

**Goal** — Skip truly non-battle enemy rows (no ships and no formation) without emitting
warnings, while keeping `c5f9fb6`'s retention of ships-with-blank-formation rows.

**Requirements** — R3

**Dependencies** — none

**Files**
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs` (`parse_enemy_table`)
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs` (existing enemy fixtures)

**Approach**
- `c5f9fb6` removed `if formation.is_none() && normalized_pattern.is_empty() { continue; }`.
  Re-introduce a skip keyed on the row being genuinely non-battle — no ship entries **and**
  no formation — so empty/structural rows are dropped silently, but a row with ships and a
  blank formation is retained (and handled as before).
- Confirm what `parse_enemy_table_skips_non_battle_rows_without_formations` feeds in
  (the fixture rows) and what `c5f9fb6` added to justify the removal; the new guard must
  satisfy both.

**Execution note** — Characterization-first: make the failing test pass without editing
its assertions; check it does not re-break any enemy-retention test `c5f9fb6` introduced.

**Patterns to follow** — the surrounding row-iteration and `normalized_pattern` /
`formation` handling already in `parse_enemy_table`.

**Test scenarios**
- A non-battle row (no ships, no formation) → skipped, `warnings.is_empty()` holds (the failing test — restore).
- A row with ships but blank formation → retained with its ships (the `c5f9fb6` intent — preserve).
- A normal battle row (ships + formation) → parsed unchanged.

**Verification** — `parse_enemy_table_skips_non_battle_rows_without_formations` passes;
no other enemy-parsing test regresses.

---

### U3. Re-measure cascade and confirm no net regression

**Goal** — After U1+U2, establish the real post-fix state of the map test surface and
record which downstream failures cascaded to recovery.

**Requirements** — R4, R5

**Dependencies** — U1, U2

**Files**
- (no production code expected) — measurement + a short note appended to the analysis,
  e.g. in this plan's follow-up or `docs/solutions/` if a learning emerges

**Approach**
- Run the full `cargo test -p emukc_bootstrap` and confirm only the two pre-existing
  `map_pipeline::verify` failures remain (Group B), with no net-new failures (R4).
- Run the two Group C gameplay tests
  (`first_gauge_clear_switches_map_variant_without_finishing_map`,
  `start_sortie_returns_post_p_unlock_layout_after_first_gauge_clear`) and record whether
  they recover. If they recover, the 7-3 failures were pure downstream of U1. If not,
  capture the residual cause (likely the stale `.data/codex` baked 2026-05-24 and/or the
  Group B topology issue) for a follow-up decision.

**Execution note** — Verification unit; no behavior change. If a trivial cascade fix
surfaces, raise it as a new unit rather than expanding this one.

**Test expectation: none** — measurement/verification only; correctness is the observed
test outcomes, not a new assertion.

**Verification** — A recorded before/after of the map test surface: Group A green,
Group B unchanged (deferred), Group C outcome documented.

---

## Scope Boundaries

**In scope** — The two `c5f9fb6` parser regressions (probability complement, enemy guard)
and a post-fix cascade re-measurement.

### Deferred to Follow-Up Work
- **Group B — `map_pipeline::verify` capture failures.** `real_game_cells_match_catalog_cell_no_and_color`
  (long-standing; fails at `c5f9fb6~1` too) and `map_1_3_real_start_capture_matches_route_cell_topology`
  (broke after `c5f9fb6`, separate commit). Data/capture-sensitive; murkier root cause;
  needs its own investigation (possibly a `.data` re-bootstrap to separate code-bug from
  capture drift).
- **Group C residue** — any 7-3 gameplay failure that does **not** cascade-recover from U1,
  e.g. the dependency on the stale `.data/codex` snapshot. Decide after U3's measurement.

**Not a goal** — Re-bootstrapping `.data`, changing the topology/verify pipeline, or
reverting `c5f9fb6`'s feature additions.

---

## Risks & Dependencies

- **Re-breaking the N-way feature.** The whole reason to fix-forward is to keep
  `c5f9fb6`'s N-way complement; U1's test scenarios must include an N-way preservation
  case so a 2-way fix doesn't silently regress N-way.
- **Enemy guard over- or under-correction.** Restoring the original guard verbatim may
  re-break what `c5f9fb6` intended; KTD-3's narrower guard plus the retention test
  scenario guard against both directions.
- **Cascade uncertainty.** Whether the 7-3 gameplay tests recover from U1 is genuinely
  unknown until measured (U3). The plan does not assume it.
- **Local-data coupling.** Group B/C partly depend on local `.data` (gitignored, baked
  2026-05-24). This plan deliberately avoids that coupling by centering on fixture-based
  parser tests, which reproduce without `.data`.

---

## Sources & Research

- Regression pinned by bisecting the failing fixture test: passes at `c5f9fb6~1`,
  fails at `c5f9fb6` and HEAD.
- `c5f9fb6 fix(map): … generalize probability complement, relax enemy formation guard` —
  changed `route.rs` (`targets.len() != 2` → `< 2`, complement rewrite) and `enemy.rs`
  (removed the non-battle-row guard).
- Current code: `postprocess_route_probabilities` in
  `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/route_condition.rs` (moved here by
  the `dd42979` module split); `parse_enemy_table` in `…/wikiwiki_map/enemy.rs`.
- Failing assertions originate in `5be5151` (the canonical spec, predating the regression).
- `start_source_cells` / `select_start_source_cell` in
  `crates/emukc_gameplay/src/game/sortie/mod.rs` — the Group C "start source cell not
  found" surface, downstream of route parsing.
