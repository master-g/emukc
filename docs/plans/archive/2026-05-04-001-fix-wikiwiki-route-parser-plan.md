---
title: fix: Recognise wikiwiki route parser unsupported conditions
type: fix
status: completed
date: 2026-05-04
---

# fix: Recognise wikiwiki route parser unsupported conditions

## Summary

Fix the wikiwiki map route parser to recognise Japanese routing keywords currently producing `Unknown`/`SourceUnknown` predicates — notably "ランダム" (random branch) on map 4-2 cell 0 — so the generated `map_catalog.json` contains no indeterminate routing rules.

---

## Problem Frame

The wikiwiki parser (`crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`) translates Japanese HTML route tables into `RoutePredicate` AST nodes. Four maps have `parse_warnings` for conditions the parser could not handle (4-2, 6-1, 6-4, 7-4). Map 4-2 cell 0 is the only cell where ALL four routing rules are `Unknown` — the parser cannot recognise "ランダム" (random-branch) conditions and produced no usable predicates, requiring the runtime fallback added in a prior fix. The remaining maps (6-1, 6-4, 7-4) have warnings but no `Unknown` rules — the parser partially succeeded, dropping sub-conditions it could not parse.

---

## Requirements

- R1. Map 4-2 cell 0 SHALL have no `Unknown` or `SourceUnknown` routing rules after regeneration
- R2. All existing parser tests SHALL continue to pass
- R3. The regenerated `wikiwiki_map_catalog.json` asset SHALL contain only resolved `RoutePredicate` variants (no `Unknown`/`SourceUnknown`) for map 4-2 cell 0

---

## Scope Boundaries

- Fix parser-level recognition of unsupported Japanese keywords in `route.rs`
- Regenerate the wikiwiki asset (`wikiwiki_map_catalog.json`)
- Regenerate the codex (`map_catalog.json`) so runtime and tests pick up the fix
- `structural_start_fallback` warnings on maps 2-4, 6-3, 7-5 are NOT addressed — they are a different class of issue (multi-source merge at cell 0)

### Deferred to Follow-Up Work

- Full resolution of all wikiwiki parsing gaps (remainder of 6-1/6-4/7-4 warnings) — separate PR
- `structural_start_fallback` resolution via independent start-cell topology source — future work
- Adding a CI check that asserts zero `Unknown`/`SourceUnknown` predicates in the generated catalog — future work

---

## Context & Research

### Relevant Code and Patterns

- **`crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`** — dispatcher `parse_route_condition_text()` (line 623) tries strategies in order; `parse_route_predicate()` (line 1979) handles atomic conditions; regexes defined at module top (lines 23-195)
- **`crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`** — `into_map_catalog()` converts parsed data to runtime catalog
- **`crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs`** — test patterns: `manifest_fixture()` for unit tests, HTML fixtures with `tempfile` for integration
- **`crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`** — compiled asset (compile-time embedded via `include_str!`)
- **`crates/emukc_model/src/codex/map/types.rs`** — `RoutePredicate` variants including `Always`, `FleetSizeWeightedRandom`

### Parsing Flow

```
parse_route_condition_text()
  ├─ parse_case_route_condition_text()  — structured 場合/のとき syntax
  ├─ parse_hardcoded_sourceunknown_block()
  ├─ parse_fleet_size_probability_clauses()
  ├─ parse_multiline_flat_route_condition_text()
  └─ various random/fallback parsers → parse_route_predicate()
```

When no sub-parser matches an atomic predicate, `unknown_predicate()` is called — which emits either `Unknown` (with warning) or `SourceUnknown` (silent, for "不明" text).

### Specific Gaps to Fix

| Map | Cell | Raw text (abbreviated) | Parse issue |
|-----|------|------------------------|-------------|
| 4-2 | 0 | `ランダム\n(駆逐+海防)が多いほどAマス寄り…\nまた、空母系の隻数も関係している` | "ランダム" keyword not recognised; multi-line bias modifiers ignored |
| 6-1 | — | `潜水艦3隻以上 かつ 潜水艦以外の艦種なしでA` / `潜水母艦(過不足なく)1隻 かつ …` | `以外の艦種なし` (no other types) not parsed as `OnlyShipTypes` conjunction |
| 6-4 | — | `戦艦級2隻(長門改二と陸奥改二のペアを除く)を含む` | Parenthetical exclusion `(…を除く)` not parsed |
| 7-4 | — | `あきつ丸 かつ 海防艦2隻 に加え 駆逐艦または海防艦1隻でA` / `…の何れかを含むとC` | Multi-clause `に加え`; `何れかを含む` (contains any of) as `Or(Contains...)` |

---

## Key Technical Decisions

- **Map "ランダム" to combination of `Always` predicates with weight:** The raw text describes multi-fleet-size weighted random, similar to `FleetSizeWeightedRandom` but with ship-type bias. Tokens like "(駆逐+海防)が多いほどAマス寄り" and "空母系の隻数も関係している" are bias modifiers. For this fix, map the "ランダム" keyword alone to `Always` with uniform weight, keeping the bias-modifier sub-lines as deferred analysis items (emitted as `parse_warning`s, not as `RoutePredicate::Unknown` rules). This eliminates the critical gap (no `Always` fallback) while preserving the parse_warning for the bias modifiers.
- **Fix parser rather than post-processing:** Adding recognition in `parse_route_condition_text()` or `parse_route_predicate()` is the right layer — post-processing at the catalog level would not help future wikiwiki regenerations.
- **Regenerate both assets:** The fix requires re-running `wikiwiki-map normalize` (to regenerate `wikiwiki_map_catalog.json`) AND `bootstrap` (to regenerate `.data/codex/map_catalog.json`), since the codex snapshot is what most tests and the runtime load.

---

## Open Questions

### Deferred to Implementation

- Exact line(s) in `parse_route_condition_text()` where "ランダム" should be intercepted — depends on the shape of the HTML row that produces the multi-line text
- Exact regular expressions needed for 6-1/6-4/7-4 sub-conditions — these depend on whether the sub-conditions are atomic predicates or multi-line case structures

---

## Implementation Units

- U1. **Recognise "ランダム" keyword in route condition parser**

**Goal:** When the parser encounters a condition text starting with or containing "ランダム", produce `Always` predicates instead of `Unknown`. For multi-line random-with-bias text (as in 4-2 cell 0), produce `Always` for the random branch and emit a parse warning for the unhandled bias modifiers.

**Requirements:** R1, R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`
- Test: `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs`

**Approach:**
- Add a check in `parse_route_condition_text()`, before `parse_hardcoded_sourceunknown_block()`: if the condition text starts with `"ランダム\n"` (keyword followed by newline, i.e. standalone random with inline bias text), or equals `"ランダム"` alone, emit `Always` predicates for each target cell. Use a word-boundary-aware `starts_with` check, not `contains()`, to avoid intercepting `"ランダム(艦隊人数により確率変動) 6隻:55%..."` texts already handled by the fleet-size-weighted-random pipeline
- For the multi-line case (contains newlines + bias modifiers), still emit `Always` but preserve the bias text as a `parse_warning` for future refinement
- The existing `postprocess_route_probabilities` and target-grouping logic at the caller level will handle associating the `Always` rules with the correct `to_cell_no` targets

**Patterns to follow:**
- Existing hardcoded-block pattern at `parse_hardcoded_sourceunknown_block` (lines ~230-312)
- `parse_route_predicate` dispatcher pattern — try specific, fall through to general

**Test scenarios:**
- Happy path: Parser receives "ランダム" as sole condition text → produces `Always` predicate(s), no `Unknown`
- Happy path: Parser receives multi-line "ランダム\n(駆逐+海防)が多いほどAマス寄り(2隻以上の場合…)" → produces `Always` + parse_warning for the bias modifier
- Edge case: "ランダム" as sub-text of a larger condition (e.g., within parentheses) — should still match
- Edge case: Text contains "ランダム" as part of another word — should NOT false-match (e.g., "非ランダム")

**Verification:**
- `cargo test -p emukc_bootstrap -- wikiwiki_map` passes
- New test: `parse_route_condition_random_keyword_emits_always`
- After regeneration, `map_catalog.json` for map 4-2 cell 0 has zero `Unknown` rules

---

- U2. **Regenerate wikiwiki map catalog asset**

**Goal:** Re-run the wikiwiki normalisation pipeline to produce an updated `wikiwiki_map_catalog.json` with the parser fix applied.

**Requirements:** R1, R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`

**Approach:**
- Run `cargo run -- wikiwiki-map normalize` to regenerate the asset
- Verify the output: map 4-2 cell 0 rules should no longer contain `Unknown` predicates
- Verify parse_warnings for map 4-2 are reduced (random bias modifiers may still warn)
- Commit the regenerated asset

**Patterns to follow:**
- Existing asset regeneration workflow (no code change needed)

**Test scenarios:**
- Static check: Open regenerated `wikiwiki_map_catalog.json`, confirm map 4-2 cell 0 rules contain `Always` not `Unknown`
- Static check: `python3 -c "import json; d=json.load(open(...)); assert no Unknown rules for map 42 cell 0"` (manual verification step)

**Verification:**
- Asset file updated on disk
- Zero `Unknown` predicates for map 4-2 cell 0 in the regenerated asset

---

- U3. **Regenerate codex map catalog**

**Goal:** Re-run bootstrap to produce an updated `.data/codex/map_catalog.json` from the fixed wikiwiki asset, so runtime and `mock_context()` tests use corrected data.

**Requirements:** R1, R2

**Dependencies:** U2

**Files:**
- Modify: `.data/codex/map_catalog.json`

**Approach:**
- Run `cargo run -- bootstrap` to regenerate
- This reads the fixed `wikiwiki_map_catalog.json` (U2 output), merges with public overlays and stat.json, and writes the final codex
- The merge logic's `routing_rules.entry().or_insert()` pattern means the wikiwiki's `Always` rules will now be present and correct

**Test scenarios:**
- Static check: Confirm regenerated `map_catalog.json` map 4-2 cell 0 has `Always` rules, not `Unknown`
- Integration: `cargo test --test gameplay_tests` passes (including 4-2 sortie if tested)

**Verification:**
- Full test suite passes: `cargo test` and `cargo test --test gameplay_tests`

---

- U4. **Add regression tests for parser behaviour**

**Goal:** Add targeted unit tests to the wikiwiki parser test suite that cover the "ランダム" keyword and prevent regression.

**Requirements:** R2

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs`

**Approach:**
- Add `parse_route_predicate_supports_random_keyword` — unit test with `manifest_fixture()`
- Add `parse_route_condition_text_handles_random_with_bias` — integration test with HTML fixture simulating 4-2 cell 0's route table row
- Verify the produced predicates are `Always`, not `Unknown`

**Patterns to follow:**
- Existing test patterns in `tests.rs` — `manifest_fixture()` for unit, `parse_debug()` for integration
- Existing tests for hardcoded/edge-case conditions

**Test scenarios:**
- Unit: `parse_route_predicate("ランダム", ...)` → `RoutePredicate::Always`
- Unit: text containing "ランダム" with bias modifiers → `RoutePredicate::Always` (bias parsed as warning)
- Integration: HTML fixture with 4-2 cell 0 route table → produced `MapCatalog` has no `Unknown` rules at cell 0, `Always` rules present
- Edge case: "非ランダム" (not random) or "ランダムではない" should NOT match

**Verification:**
- `cargo test -p emukc_bootstrap -- wikiwiki_map` passes

---

## System-Wide Impact

- **Interaction graph:** The parser fix changes the wikiwiki → MapCatalog compilation pipeline. The runtime `evaluate_route_destination` path is unchanged — it already handles `Always` predicates correctly. The fallback added in the prior fix (indeterminate → `select_route_from_cells`) becomes a safety net rather than the primary path for 4-2 cell 0.
- **Unchanged invariants:** Runtime routing evaluation, merge logic, cell data structures, API response format — all unchanged.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Must have local wikiwiki HTML pages to run `wikiwiki-map normalize` | The repo includes embedded wikiwiki catalog fallback; if pages are missing, download via `cargo run -- wikiwiki-map download` first |
| Regenerating codex may change unrelated map data | Run tests after regeneration; any unintended changes are visible in git diff |
| `include_str!` embedding masks stale asset if not committed | U2 explicitly commits the regenerated asset |

---

## Sources & References

- **Prior analysis:** Debug session on map routing graph (2026-05-03)
- **Related fix:** `crates/emukc_gameplay/src/game/map_route.rs` — indeterminate rules fallback
- **Parser code:** `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`
- **Related plan:** `docs/plans/2026-05-03-002-fix-sortie-gameplay-audit-findings-plan.md` (U6 section)
