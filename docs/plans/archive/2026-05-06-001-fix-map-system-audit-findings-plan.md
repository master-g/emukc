---
title: "fix: Resolve map system audit findings"
type: fix
status: completed
date: 2026-05-06
origin: /Users/mg/.claude/plans/map-gameplay-tidy-leaf.md
---

# fix: Resolve map system audit findings

## Summary

Address the 32 findings from the `refactor/map` systematic audit (0 P0, 9 P1, 10 P2, 13 P3, 8 architectural notes). Group fixes into 10 implementation units prioritized by severity. No P0 blocker exists, but nine P1 correctness gaps — merge-path asymmetry, LoS formula dead field, `VisitedNode` remap miss, wikiwiki fan-out without topology check, silent bootstrap parse swallowing, and gauge-index reset — must land before merging the refactor branch. P2 hardening and P3 cleanup follow. Test-coverage gap fill ends the plan.

---

## Problem Frame

The `refactor/map` branch split kcdata (topology) from wikiwiki (routing rules). Subsequent fix commits closed several gaps. A three-agent audit of data source → graph construction → routing rules → gameplay chain turned up 32 findings. No blockers, but several correctness gaps survive: merged secondary routing rules are silently dropped, a predicate field is parsed but ignored, parse failures are logged-and-continued with an ambiguous report, and a rank-select can revert gauge progress. Left unfixed, these manifest as wrong routes at runtime without any user-visible error signal — the worst failure class for a game emulator.

---

## Requirements

Groups of findings promoted to plan-local requirements (full detail in origin):

- **R1.** `merge_variant_definition` and `merge_routing_overlay` must have symmetric accumulation semantics for routing rules. (audit #2)
- **R2.** `remap_variant_to_definition_identity` must remap cell numbers embedded inside predicate payloads (`VisitedNode.cell_nos`, recursive `And`/`Or`). (audit #3)
- **R3.** `RoutePredicate::LoS` evaluator must honor the `formula` field (式1/3/4). (audit #1)
- **R4.** `VisitedNodeLabel` must return `SourceUnknown` (not `NotMatched`) when a label fails to resolve. (audit #5)
- **R5.** `EquipmentCount` semantics must match wikiwiki text intent, confirmed with a test exercising multi-equip ships. (audit #9)
- **R6.** wikiwiki `""`-variant fan-out must validate `from_cell_no`/`to_cell_no` exist in each target variant before merge. (audit #4)
- **R7.** Bootstrap parse failures (`wikiwiki_map_catalog.json`, kcdata YAML) must surface in the report instead of silently degrading to empty. (audit #7, #8)
- **R8.** `select_eventmap_rank` must preserve `gauge_index` when HP is already set. (audit #6)
- **R9.** `ensure_synthetic_variants` must use a sentinel (`boss_cell_no: 0`) that callers can detect, instead of silently pointing at cell 1. (audit #10)
- **R10.** `postprocess_route_probabilities` must handle N-way junctions or emit an explicit `ProbabilityUnknown` marker. (audit #11)
- **R11.** `verify.rs` must iterate all variants and cover event maps when fixtures are present. (audit #12, #13)
- **R12.** wikiwiki enemy parser must retain rows with blank formation when ship list is non-empty. (audit #14)
- **R13.** `route_section_variant_key` must detect gauge 3+ and event-style gauge names. (audit #15)
- **R14.** `cell_has_routing_outgoing` name or behavior must be aligned; callers must not rely on an incorrect implication. (audit #31)
- **R15.** Mixed `weight` + `probability_pct` at the same junction must be rejected or normalized. (audit #32)
- **R16.** `rashin_flg` must reflect route determinism, not raw `next_cells` fan-out. (audit #17)
- **R17.** Dead code (`collect_kcdata_nodes`, `DEFAULT_MAP_RECORDS`), latent dead-string blocks, and edit-residue blank lines must be cleaned. (audit #20, #21, #27, #29)
- **R18.** Silent drops of API parameters must be commented; heuristic runtime setup (`Runtime::new()` inside `thread::scope`) must be made re-entry-safe. (audit #23, #28)
- **R19.** `tests/gameplay_tests/map/` must cover retreat-does-not-advance, multi-gauge clear sequence, monthly reset policy, and `sortie_battle_result` non-boss pending clear. (architectural observation #6)
- **R20.** Graph-invariant validation pass must run at bootstrap or on codex load (self-loops, unreachable cells, rule targets not in next_cells). (architectural observation #3)

---

## Scope Boundaries

- No behavior change beyond what the audit findings describe. No speculative features.
- No rework of the topology/routing separation itself — the refactor is sound.
- No migration of map asset JSONs; R20 validation only warns, does not block load.

### Deferred to Follow-Up Work

- Filling in KanColle's real weighted `engagement_for_cell` distribution (#26) — tracked as known placeholder; not in this plan.
- Adding event-map captures and semantic stat.json merge implementation (audit architectural #8) — out of scope; separate asset-generation effort.
- Sourcing kcdata node type from YAML instead of heuristic (#19) — requires upstream kcdata schema change; follow-up.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_model/src/codex/map/merge.rs` — merge pipeline; `merge_routing_overlay` at line ~236 already uses `or_default().extend(...)` pattern to mirror.
- `crates/emukc_gameplay/src/game/map_route.rs` — `RoutePredicate` enum + `evaluate_route_destination`; `route_predicate_matches` is the eval core.
- `crates/emukc_bootstrap/src/map_pipeline/` — `sources.rs` (orchestration), `kcdata.rs` (topology), `assemble.rs` (overlay merge), `verify.rs` (sanity tests).
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/` — `route.rs` (rule AST + parser), `enemy.rs`, `html.rs`, `resolver.rs`.
- `tests/gameplay_tests/map/unlock.rs` — existing pattern for gameplay-level map tests; new scenarios follow the same shape.

### Institutional Learnings

- `docs/plans/2026-05-05-005-fix-map-system-audit-issues-plan.md` — previous audit pass on this branch; mostly addressed by 40b0e54.
- `docs/plans/2026-05-05-006-fix-route-jumping-and-stale-stage-state-plan.md` — route-jumping work closed by f5379cc.
- `docs/plans/2026-05-03-002-fix-sortie-gameplay-audit-findings-plan.md` — prior sortie audit shape; this plan follows the same grouping discipline.

### External References

None. Findings are internal architecture; no new external research required.

---

## Key Technical Decisions

- **Merge symmetry over override policy.** `merge_variant_definition` will switch from `or_insert` to `or_default().extend(...)` to match its sibling. Override policy (last-write-wins) was never documented and the asymmetry looks like a bug, not a design choice. If future callers need override semantics, introduce an explicit `merge_mode` parameter rather than relying on path-dependent behavior.
- **LoS formula implemented on `FleetRouteContext`, not on the fly.** Adding formula-variant LoS to `FleetRouteContext::los_by_formula(formula: &str) -> f64` keeps eval deterministic, avoids redundant recomputation across predicates, and allows a single test covering all formulas per fleet snapshot.
- **Predicate `SourceUnknown` over `NotMatched` on resolution miss.** Rule-source ambiguity should not silently filter out a candidate route. Caller (`evaluate_route_destination`) already has the `unknown_predicate` fallback path; routing the miss there preserves parity with other predicate types.
- **Fan-out guard is skip + warn, not hard fail.** Dropping an overlay that doesn't fit the target variant is recoverable (the variant just has no rules); a hard fail would block bootstrap on any author mistake in wikiwiki asset.
- **Bootstrap parse failures surface as report variants, not hard fails.** Distinguishing `ParseFailed` from `Missing` in `MapCatalogWikiwikiSource` lets CI and dev tooling fail conditionally without preventing local iteration.
- **Graph validation pass runs at codex load with warn-only severity.** A hard fail during load would brick the server on any upstream data glitch; `tracing::warn!` with a counter gives us a CI signal without production fragility.
- **Mixed weight/pct junctions rejected at bootstrap, not runtime.** Catch at parse time — the AST knows both fields — rather than at eval time where state is already committed.

---

## Open Questions

### Resolved During Planning

- **Should the merge override be documented as intentional?** No. The symmetric sibling contradicts it; treat as bug.
- **Should bootstrap fail on corrupt kcdata YAML?** Yes for kcdata (topology authority), warn for wikiwiki (rule overlay). Topology corruption silently producing empty catalogs is the worst failure class.
- **Is `select_eventmap_rank`'s gauge reset intentional for rank downgrade?** Likely not — no origin doc supports it, and upstream clients show no UI surface for "reset from rank-switch". Treat as bug; preserve gauge.

### Deferred to Implementation

- Exact `EquipmentCount` semantics per wikiwiki text — must check a sample fixture at implementation time before committing test expectations.
- Whether N-way probability complement should distribute evenly or emit an uncertainty marker — depends on whether real wikiwiki tables ever encode "A: 40%, B/C: ?". Check corpus first.

---

## Implementation Units

- U1. **Unify variant-merge semantics and extend predicate remapping**

**Goal:** Make `merge_variant_definition` accumulate routing rules like `merge_routing_overlay` does; extend `remap_variant_to_definition_identity` to walk predicate payloads recursively.

**Requirements:** R1, R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_model/src/codex/map/merge.rs`
- Test: `crates/emukc_model/src/codex/map/merge.rs` (inline `#[cfg(test)] mod tests`) or new `crates/emukc_model/src/codex/map/merge_tests.rs`

**Approach:**
- Change `merge_variant_definition` routing-rules branch from `.entry(from_cell_no).or_insert(valid_rules)` to `.or_default().extend(valid_rules)`.
- Add a `remap_predicate` helper that recursively walks `RoutePredicate::{VisitedNode, And, Or, Not, ...}` and remaps embedded `cell_nos` via `remap_cell_no`.
- Invoke helper inside the `routing_rules` loop of `remap_variant_to_definition_identity` on each `rule.predicate`.

**Execution note:** Start with a failing test that constructs two source variants with overlapping `from_cell_no` rules, merges them, and asserts union. Then add a `VisitedNode.cell_nos` test with a non-identity `cell_no_map`.

**Patterns to follow:**
- `merge_routing_overlay` at merge.rs:236 for accumulation shape.
- `remap_cell_nos` helper for the in-place vector remap.

**Test scenarios:**
- Happy path: primary variant has rules at from_cell=5; secondary variant also has rules at from_cell=5. After merge, both sets are present.
- Happy path: predicate containing `VisitedNode { cell_nos: [3, 7] }` on secondary; `cell_no_map` remaps 3→4, 7→9. Post-merge predicate reads `[4, 9]`.
- Edge case: nested `And([VisitedNode, Or([VisitedNode, Always])])` remaps through all layers.
- Edge case: empty `cell_no_map` leaves predicate untouched (early return).

**Verification:** New tests pass. Existing merge tests still pass. `cargo test -p emukc_model codex::map` green.

---

- U2. **Implement LoS formula-aware evaluation and fix predicate unknown-vs-miss semantics**

**Goal:** Honor the `formula` field in `RoutePredicate::LoS`; return `SourceUnknown` (not `NotMatched`) when `VisitedNodeLabel` fails to resolve; document or rename `cell_has_routing_outgoing`; validate mixed weight/pct at the same junction (bootstrap-time).

**Requirements:** R3, R4, R14, R15

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` (mixed-encoding validation at parse time)
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (inline tests)

**Approach:**
- Add `FleetRouteContext::los_by_formula(formula: Option<&str>) -> f64` computing formula 1/3/4 per KanColle conventions. Default (None) → existing `los_total`.
- Replace `context.los_total` comparison in `route_predicate_matches` LoS arm with `context.los_by_formula(formula.as_deref())`.
- In `VisitedNodeLabel` arm (line ~278), change `NotMatched` on resolution miss to `SourceUnknown`.
- Add doc comment to `cell_has_routing_outgoing` clarifying it returns true when *any* rule exists at `from_cell_no`, regardless of whether its `to_cell_no` is in `current.next_cells`; caller must handle `candidate_targets.is_empty()` downstream. Or rename to `cell_has_routing_rules` if call sites allow.
- At rule-set post-processing in `parser/wikiwiki_map/route.rs`, detect junctions where some rules carry `weight` and others carry `probability_pct`, and emit a parse warning (`mixed_routing_encoding`) + normalize to one encoding per junction.

**Execution note:** Write a LoS formula-1 vs formula-3 test first with identical fleet and assert different route picks based on formula.

**Patterns to follow:**
- Existing formula computation in `emukc_model::codex::ship` if any (check for LoS-related helpers to reuse).
- `parse_warnings` mechanism in wikiwiki parser for soft-validation signals.

**Test scenarios:**
- Happy path: fleet with radars, formula 3, threshold 40 → LoS-by-formula-3 computed and compared; route selects expected branch.
- Happy path: same fleet, formula 1, same threshold → different LoS, different route.
- Edge case: `formula: None` → falls back to `los_total` (existing behavior preserved).
- Error path: `VisitedNodeLabel` references a label absent from the resolved graph → predicate returns `SourceUnknown`, fallback path taken.
- Error path: junction with one `weight: 1` rule + one `probability_pct: 50.0` rule → parse warning `mixed_routing_encoding` emitted.
- Integration: `cell_has_routing_outgoing` doc update sanity-checked against caller in `sortie.rs:402`.

**Verification:** All new tests pass. `cargo test -p emukc_gameplay map_route` green. Grep shows no callers misinterpret `cell_has_routing_outgoing` post-doc.

---

- U3. **Confirm EquipmentCount semantics and add coverage**

**Goal:** Verify `EquipmentCount` predicate semantics against wikiwiki text corpus; adjust parser or evaluator so both agree; lock with tests.

**Requirements:** R5

**Dependencies:** U2 (shares evaluator file; sequence to avoid merge churn)

**Files:**
- Modify (if parser wrong): `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` around :2564
- Modify (if evaluator wrong): `crates/emukc_gameplay/src/game/map_route.rs` around :293-307
- Test: `crates/emukc_gameplay/src/game/map_route.rs`

**Approach:**
- Open a known wikiwiki fixture (e.g., `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`) and identify a rule like "電探を装備した艦が2隻以上" — confirm literal intent: "ships carrying radar, count ≥ 2".
- If parser currently emits "items count", adjust parser to emit "ship count" (or introduce a distinct `EquipmentItemCount` variant if both forms exist in corpus).
- Add evaluator test: single fleet with one ship carrying two radars → count = 1 (not 2).
- Add evaluator test: three ships, two carrying radar → count = 2.

**Execution note:** Check corpus first before writing code changes. Likely no behavior change needed; the job is test coverage + explicit semantic documentation.

**Patterns to follow:** Existing predicate test shapes in `map_route.rs`.

**Test scenarios:**
- Happy path: fleet[ship(radar), ship(), ship(radar,radar)] → `EquipmentCount(radar) = 2` (two ships carrying at least one radar).
- Edge case: empty fleet → 0.
- Edge case: ship with empty slots → does not count.
- Integration: at least one fixture from real wikiwiki data runs through the parser and produces the predicate that matches expected route behavior.

**Verification:** Tests pass. Corpus check documented in commit message. Parser and evaluator agree on ship-count semantics.

---

- U4. **Topology-checked wikiwiki fan-out**

**Goal:** Before merging a wikiwiki `""` variant's rules onto a named kcdata variant, validate each rule's `from_cell_no` and `to_cell_no` exist in the target variant's cell set; skip + warn on mismatch.

**Requirements:** R6

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Test: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` (or a new `tests/` module if the existing file has none)

**Approach:**
- In the fan-out loop (assemble.rs:81-86), gather target variant's cell-no set into a `BTreeSet<i64>`.
- Partition incoming routing rules into (kept, dropped) based on membership. Apply only kept rules.
- Emit `tracing::warn!` with map_id, variant_key, dropped count if any dropped.
- Consider adding a `MapCatalogBuildReport` counter `fanout_rules_dropped` so bootstrap report surfaces the count.

**Patterns to follow:** `merge_routing_overlay`'s target-cell validation at merge.rs:88-97 (already drops rules whose `to_cell_no` is not in topology with a warn).

**Test scenarios:**
- Happy path: wikiwiki variant `""` with rules at from_cell=3, to_cell=5. Target variant has both cells. Rules are merged.
- Edge case: target variant missing cell 5. Rule dropped with warn. Other rules still merged.
- Edge case: target variant missing both cells in every rule. No rules merged; warn count matches dropped count.
- Integration: multi-gauge map where gauge_1 has cell 5 but gauge_2 doesn't → `""`-variant rule only merged into gauge_1.

**Verification:** Tests pass. Run `cargo test -p emukc_bootstrap map_pipeline::assemble` green. Manual inspection of a real bootstrap run shows no unexpected drops on current asset set.

---

- U5. **Surface bootstrap parse failures in the build report**

**Goal:** Stop silently swallowing `wikiwiki_map_catalog.json` JSON parse errors and kcdata YAML deserialize errors; add report variants; decide hard-fail vs warn per source.

**Requirements:** R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/sources.rs` (around :118-124, :139, :210)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (around :79)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/report.rs` (add variants if needed)
- Test: `crates/emukc_bootstrap/src/map_pipeline/sources.rs` with a fixture corrupt JSON/YAML

**Approach:**
- Extend `MapCatalogWikiwikiSource` enum with `ParseFailed { path: PathBuf, error: String }`.
- In sources.rs, replace the `warn! + continue` flow with explicit variant emission.
- In kcdata.rs, replace `Err(_) => continue` with `Err(e) => { warn!(%path, ?e, "skip"); stats.kcdata_parse_errors += 1; continue }`. If `stats.kcdata_parse_errors > 0` and bootstrap is configured for strict mode, fail; otherwise surface in report.
- Update report rendering (`report.rs`) to print the new variants.
- Review the `stat.json` loader note at sources.rs:210 — either implement the semantic-cell merge the comment describes, or trim the comment.

**Execution note:** Test-first. Add a fixture with corrupt JSON; assert `ParseFailed` variant.

**Patterns to follow:** Existing `MapCatalogWikiwikiSource` enum shape.

**Test scenarios:**
- Happy path: valid `wikiwiki_map_catalog.json` → `Filesystem` variant, correct map count.
- Error path: syntactically invalid JSON file → `ParseFailed { path, error }` variant, error non-empty, report prints path+error.
- Error path: missing file → `Missing` variant (pre-existing behavior preserved).
- Edge case: valid JSON but empty array → `Filesystem { map_count: 0 }` (not ParseFailed).
- Error path: corrupt single YAML file in kcdata tree → warn logged, counter incremented, other YAMLs still loaded.

**Verification:** Tests pass. Running `cargo run -- bootstrap` with an intentionally corrupt wikiwiki JSON shows `ParseFailed` in the report output instead of `Filesystem` + `count: 0`.

---

- U6. **Preserve gauge_index in select_eventmap_rank**

**Goal:** `select_eventmap_rank` must only reset `gauge_index` to 1 when initializing HP; if HP is already set (mid-event), preserve progress.

**Requirements:** R8

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map.rs` around :130-133
- Test: `tests/gameplay_tests/map/` (new file or extend `unlock.rs` → rename to `rank.rs`/add a `rank_switch` test)

**Approach:**
- Move the `gauge_index = 1` assignment inside the `current_hp.is_none()` branch. Outside it, leave `gauge_index` unchanged.
- Confirm design intent in a short comment: "gauge_index resets only on fresh initialization; rank changes mid-gauge preserve progress".

**Execution note:** Test-first.

**Patterns to follow:** Existing map-record update shape in `map.rs` and `map_record.rs`.

**Test scenarios:**
- Happy path: fresh event map, `current_hp = None`, call `select_eventmap_rank(甲)` → HP set, gauge_index = 1.
- Happy path: existing record, `gauge_index = 2`, call `select_eventmap_rank(甲)` → gauge_index unchanged.
- Edge case: existing record, same rank re-selected → no-op (gauge_index preserved, HP unchanged).
- Integration: full flow — clear gauge 1, switch rank from 甲 to 乙 → gauge_index still 2.

**Verification:** Tests pass. `cargo test --test gameplay_tests rank` green.

---

- U7. **Synthetic variant sentinel + graph invariant validation pass**

**Goal:** Replace silent `boss_cell_no: 1` fallback with a `0` sentinel; add a bootstrap/codex-load graph-invariant pass that warns on self-loops, unreachable cells, and rules referencing cells not in `next_cells`.

**Requirements:** R9, R20

**Dependencies:** U1 (merge semantics stable before validating the output)

**Files:**
- Modify: `crates/emukc_model/src/codex/map.rs` around :143-145 (`ensure_synthetic_variants`)
- Modify: `crates/emukc_model/src/codex/map/debug.rs` or new `crates/emukc_model/src/codex/map/validate.rs`
- Modify callers reading `boss_cell_no` (grep `boss_cell_no`): handle `0` sentinel (warn, fall back, or return error depending on call site)
- Test: new `crates/emukc_model/src/codex/map/validate.rs` inline tests

**Approach:**
- Change synthetic `boss_cell_no: 1` → `0`. Document sentinel contract.
- Add `MapDefinition::validate(&self) -> Vec<MapValidationWarning>` emitting structured warnings for: self-loops, orphan cells (no incoming edge and not cell 0), rule targets missing from topology.
- Run validator once per map at codex load; log aggregated counts per severity via `tracing::warn!`.
- `ensure_synthetic_variants` callers that dereference `boss_cell_no` unconditionally must be updated to treat 0 as "unknown" — audit call sites via `rg "boss_cell_no"`.

**Patterns to follow:** Existing `parse_warnings: Vec<String>` pattern on `MapVariantDefinition`.

**Test scenarios:**
- Happy path: synthesized fallback variant has `boss_cell_no: 0`; caller reading it detects sentinel.
- Happy path: valid map passes validator with zero warnings.
- Edge case: map with a self-loop (cell 3 lists cell 3 in next_cells) → warning `self_loop { cell: 3 }`.
- Edge case: map with cell 5 unreachable from cell 0 → warning `unreachable { cell: 5 }`.
- Edge case: rule at from_cell=2, to_cell=99, but cell 2's next_cells is [3, 4] → warning `rule_target_not_in_next_cells`.
- Integration: all shipped asset JSONs load with zero or expected warnings (record a baseline).

**Verification:** Tests pass. Bootstrap log shows the aggregated counts per map. No callers crash on sentinel.

---

- U8. **Wikiwiki parser edge cases: gauge 3+, probability complement, enemy formation-optional, verify layer hardening**

**Goal:** Close remaining wikiwiki parser and verify-layer gaps: detect `ゲージ3`/`第三ゲージ`/event-style variant keys; complement probabilities for N-way junctions; keep enemy rows with blank formation when ship list is non-empty; iterate all variants in `verify.rs` and don't skip event maps.

**Requirements:** R10, R11, R12, R13

**Dependencies:** None (can land before or after U5; independent)

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` (:267-285, :563-564)
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs` (:46-53)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/verify.rs` (:57-63, :128-130)
- Test: inline tests in each file; extend `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs`

**Approach:**
- `route_section_variant_key`: extend keyword set to include `ゲージ3`, `第三ゲージ`, `ゲージ4`, `第四ゲージ`; if none match, try a `/^ゲージ(\d+)/` regex → `gauge_N`.
- `postprocess_route_probabilities`: generalize complement derivation — when `targets.len() > 1` and sum of known probabilities < 100, distribute remainder among rules tagged `random_placeholder`; if the distribution is ambiguous, emit `ProbabilityUnknown` marker instead of `Always` fallback.
- `enemy.rs`: change guard from `formation.is_empty() && pattern.is_empty()` to `ship_names.is_empty()`. Row with non-empty ships retains `formation: None`.
- `verify.rs`: iterate `definition.variants` and report per-variant cell-count comparison; drop the `map_id > 74` event-map guard.

**Patterns to follow:** Existing parse_warnings emission for soft-validation.

**Test scenarios:**
- Happy path: section header `## ゲージ3` → variant key `gauge_3`.
- Happy path: 3-way junction, probabilities [40%, 30%, ?] → complement derived as 30%, all rules resolved.
- Happy path: 3-way junction, probabilities [40%, ?, ?] → `ProbabilityUnknown` marker emitted.
- Happy path: enemy row with ships but blank formation column → row retained, `formation: None`.
- Edge case: enemy row with empty ship list → row dropped (pre-existing behavior).
- Happy path: event map capture present in fixtures → `verify.rs` runs, asserts, passes.
- Edge case: variant with cell count mismatch → verify emits per-variant warning, not a silent skip.

**Verification:** All parser tests pass. `cargo test -p emukc_bootstrap` green. Bootstrap on the current asset set produces no new unexpected warnings.

---

- U9. **Low-priority cleanup: dead code, stale statics, blank lines, comments, runtime safety**

**Goal:** Clear the long tail of P3 findings in a single focused commit — all low-risk, file-local, mostly mechanical.

**Requirements:** R17, R18

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` (:286-293 delete `collect_kcdata_nodes`; :818-907 annotate hardcoded-block with `TODO(expiry: 2027-01)` + move to data file if cheap)
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/html.rs` (:93-140 off-by-one fix + unit test)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/sources.rs` (:139 — replace `thread::scope + Runtime::new()` with async path or `spawn_blocking`)
- Modify: `crates/emukc_bootstrap/src/wikiwiki_map_download.rs` (:95-97 log excluded area count)
- Modify: `crates/emukc_gameplay/src/game/map.rs` (:253-268 cap `build_regular_prerequisites` loop at area 7)
- Modify: `crates/emukc_gameplay/src/game/sortie.rs` (:938-947 remove 7 blank lines; :1652-1659 placeholder comment kept; :329 `rashin_flg` determinism check; :620-651 doc comment)
- Modify: `crates/emukc_gameplay/src/game/sortie_result.rs` (:399-406 gauge_count comment)
- Modify: `crates/emukc_model/src/profile/map_record.rs` (:193-395 `#[deprecated]` on `DEFAULT_MAP_RECORDS` with removal milestone)
- Modify: `src/bin/net/router/kcsapi/api_req_map/next.rs` (:25-26 comment drop) and `start.rs` (:33 comment drop)
- Test: where relevant (html rowspan unit test; rashin_flg determinism test)

**Approach:**
- Single commit per theme; rashin_flg determinism gets its own test since it changes observable behavior.
- Grep call sites of `DEFAULT_MAP_RECORDS` before `#[deprecated]` to confirm zero gameplay-critical consumers.
- `rashin_flg` fix: after evaluating routing at `start_sortie`, compare candidate target set from `evaluate_route_destination` against `cell_0.next_cells`; flag only when candidate target count > 1.

**Execution note:** Batch by file; verify build passes after each file.

**Patterns to follow:** Existing `tokio::task::spawn_blocking` usages in the codebase.

**Test scenarios:**
- Happy path: `rashin_flg` false when routing rule forces a unique target even though `cell_0.next_cells.len() > 1`.
- Happy path: `rashin_flg` true when routing is genuinely non-deterministic.
- Edge case: `table_to_grid` with rowspan 3 + pending cells in higher columns — rowspan-boundary test asserts grid shape correct.
- Build: `cargo build --release` clean, no unused-import warnings from cleanup.

**Verification:** `cargo clippy --workspace` clean. `cargo test --workspace` green. `cargo run -- bootstrap` prints the new area-filter log line.

---

- U10. **Gameplay test coverage fill**

**Goal:** Close the `tests/gameplay_tests/map/` coverage gaps called out in the audit's architectural observation #6.

**Requirements:** R19

**Dependencies:** U1–U9 (tests exercise the fixed code paths)

**Files:**
- Create: `tests/gameplay_tests/map/retreat.rs`
- Create: `tests/gameplay_tests/map/multi_gauge.rs`
- Create: `tests/gameplay_tests/map/monthly_reset.rs`
- Create: `tests/gameplay_tests/map/non_boss_pending.rs`
- Modify: `tests/gameplay_tests/map/mod.rs` (register new modules)

**Approach:**
- Each test uses in-memory DB + loaded codex, same pattern as `unlock.rs`.
- Retreat: advance two cells, `goback_port` → assert `map_record.current_map_id` not advanced, `sortie_state` cleared.
- Multi-gauge: simulate clearing gauge 1 of a 2-gauge map → assert `gauge_index` advances; clear gauge 2 → assert map clear flag set.
- Monthly reset: set map clear time to previous month, trigger `MapResetPolicy::Monthly` → assert clear state reset.
- Non-boss pending: trigger `sortie_battle_result` on a non-boss battle → assert `pending_battle_cell_id` and `pending_result` both cleared.
- sp_midnight happy path (optional, add if straightforward): full night-battle flow from `api_req_battle_midnight`.

**Execution note:** Test-first.

**Patterns to follow:** `tests/gameplay_tests/map/unlock.rs` — fixture setup, codex loading, assertion style.

**Test scenarios:**
- Retreat: happy path listed above.
- Retreat: edge case — retreat before any cell visited (cell 0 only) → state cleared, no error.
- Multi-gauge: happy path listed above.
- Multi-gauge: edge case — attempt to enter gauge 2 before gauge 1 cleared → rejected.
- Monthly reset: happy path listed above.
- Monthly reset: edge case — timestamp in current month → no reset.
- Non-boss pending: happy path listed above.
- Integration: full sortie loop across start → next → battle → battleresult → next → battle → goback_port, confirm state at each step.

**Verification:** `cargo test --test gameplay_tests` green. New tests appear in test output. Coverage of `tests/gameplay_tests/map/` now spans unlock + retreat + multi-gauge + monthly + pending-clear.

---

## System-Wide Impact

- **Interaction graph:** Merge changes (U1) affect every codex load path; LoS formula (U2) affects every routing decision at runtime; fan-out guard (U4) affects bootstrap for every multi-variant map; sentinel (U7) affects callers of `boss_cell_no`.
- **Error propagation:** Parse-error surfacing (U5) changes what bootstrap's CLI reports but not its exit code unless a new strict flag is added — intentional; keep exit behavior stable.
- **State lifecycle risks:** U6 (gauge_index preservation) changes a write path; need to confirm no downstream logic assumes gauge_index always starts at 1 per rank-select.
- **API surface parity:** No KCS API response shape changes. `rashin_flg` field value may change in edge cases (U9) — client visual-only effect.
- **Integration coverage:** U10 fills gameplay-level coverage; no changes to HTTP-level tests.
- **Unchanged invariants:** Topology/rule separation (the refactor's core contract) is preserved. No public trait signatures change. Database schema untouched. Asset file format untouched.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Merge symmetry change (U1) alters existing behavior for maps with primary+secondary rules at same `from_cell_no`. | Add test with corpus fixture before change; diff resulting codex for all shipped maps and confirm no unintended drift. |
| LoS formula change (U2) may shift route outcomes for already-clearing players if the old (wrong) LoS happened to match their loadout. | Document as a known correctness fix; flag in changelog. |
| Synthetic variant sentinel (U7) breaks callers that read `boss_cell_no` unconditionally. | Grep-and-fix all call sites; new tests cover sentinel-detection. |
| Mixed weight/pct validation (U2) could emit warnings on production wikiwiki assets. | Validation is warn-only; bootstrap still succeeds. |
| Graph invariant pass (U7) surfaces pre-existing bad data as warnings. | Warn-only; no hard fail. Establish baseline count in a docs/solutions note. |
| Test-suite expansion (U10) slows CI. | New tests use in-memory DB; expected impact < 2 seconds total. |

---

## Documentation / Operational Notes

- Update `docs/map/kancolle-map-research.md` with the LoS formula variants once implemented (U2), so the derivation is reviewable.
- No rollout concerns — all changes are local to the server binary and take effect on next restart.
- After U7 lands, create a `docs/solutions/` note recording the baseline graph-warning counts per shipped map asset, so future regressions are detectable.

---

## Sources & References

- **Origin document:** `/Users/mg/.claude/plans/map-gameplay-tidy-leaf.md` (personal plan dir, not in repo)
- Prior plans on this branch: `docs/plans/2026-05-05-005-fix-map-system-audit-issues-plan.md`, `docs/plans/2026-05-05-006-fix-route-jumping-and-stale-stage-state-plan.md`
- Refactor commit: `d3bc6d4` (topology/routing separation)
- Recent fix commits addressed: `40b0e54`, `f5379cc`, `e58b4f2`, `0a5d702`
