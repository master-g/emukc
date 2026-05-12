---
title: "fix: Address code review findings from feat/vibe last 10 commits"
type: fix
status: active
date: 2026-05-12
---

# fix: Address Code Review Findings

## Summary

Fix actionable findings from multi-agent code review of the last 10 commits on feat/vibe: eliminate a duplicated helper function, replace unsafe `.unwrap()` calls in tests, surface silent test skips, document transaction requirements, and fill critical test coverage gaps in maelstrom and battle subsystems.

---

## Requirements

- R1. Eliminate duplicated `multi_label_index` helper between assemble.rs and label_overlay.rs
- R2. Replace `.unwrap()` on fallible I/O and parse operations in verify.rs tests
- R3. Make verify.rs topology test failure visible when repo assets are absent
- R4. Document transaction requirement on `resolve_non_battle_node_effect` maelstrom path
- R5. Add maelstrom ammo-drain test coverage (color_no == 4)
- R6. Add maelstrom radar reduction tier table test coverage (6 tiers, 1-6 ships)
- R7. Add test for kouku + shelling combined sinking protection sequence

---

## Scope Boundaries

- Pre-existing issue: `boss_cell_no=0` conflating "no boss" with "boss at cell 0" (kcdata.rs:178) — tracked separately
- `evaluate_route_destination` / `evaluate_route_candidate_count` shared logic refactor (map_route.rs:98) — deferred, existing code works
- `merge_label_overlay` priority re-indexing behavior (label_overlay.rs:98) — deferred, needs design decision on whether re-index is intentional
- Advisory items (kouku fixed seed, probabilistic test assertions, superseded plan files) — no code action needed

---

## Context & Research

### Relevant Code and Patterns

- `multi_label_index` exists identically in `crates/emukc_bootstrap/src/map_pipeline/assemble.rs:320` and `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs` — one takes `&[MapCellDefinition]`, the other `&MapVariantDefinition`
- Existing `_impl` pattern in gameplay crate: internal functions take generic `C: ConnectionTrait` for transaction participation
- Maelstrom radar reduction tiers in `sortie/mod.rs:1301`: match on `radar_ship_count` 1..=6 with float literals 0.25..0.60
- `resolve_non_battle_node_effect` receives connection `c` from caller's transaction scope (sortie/mod.rs:450-454)

### Institutional Learnings

- `display_damage()` is mandatory for all damage display paths — kouku violated this twice historically
- Maelstrom resource deductions must operate per-ship, not on profile-level shared material pool
- Fallback enemy ID must use abyssal ship 1501, never friendly ship IDs

---

## Key Technical Decisions

- Extract `multi_label_index` as a method on `MapVariantDefinition` rather than a free function in a helpers module — keeps the API on the type that owns the data
- verify.rs `.unwrap()` replacements use `match` with `MapCatalog::default()` fallback + `eprintln!` warning, consistent with existing early-return pattern in the same file
- Maelstrom tests use the same in-memory DB + codex pattern as existing `sortie_tests.rs`

---

## Open Questions

### Deferred to Implementation

- Exact maelstrom test ship configurations — depend on codex data availability
- Whether verify.rs should use `#[ignore]` attribute for data-dependent tests instead of silent fallback — implementation-time judgment call

---

## Implementation Units

### U1. Extract shared `multi_label_index` helper

**Goal:** Remove code duplication between assemble.rs and label_overlay.rs by consolidating `multi_label_index` into a single impl on `MapVariantDefinition`.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Modify: `crates/emukc_bootstrap/src/map_pipeline/label_overlay.rs`
- Modify: `crates/emukc_model/src/codex/map/types.rs` (add method to `MapVariantDefinition`)

**Approach:**
- Add `pub fn multi_label_index(&self) -> BTreeMap<String, Vec<i64>>` method to `MapVariantDefinition` that iterates `self.cells` and builds a multi-valued index from `node_label` to matching `cell_no`s, preserving duplicate labels
- Replace both call sites in assemble.rs and label_overlay.rs with calls to the new method
- Verify both existing test suites still pass

**Patterns to follow:**
- Existing methods on `MapVariantDefinition` in `crates/emukc_model/src/codex/map/types.rs`

**Test scenarios:**
- Happy path: existing `test_fanout_happy_path_no_drops` and `happy_path_all_labels_match` still pass after refactor
- Edge case: duplicate labels produce a `Vec<i64>` with multiple `cell_no` entries for that key

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- `cargo test -p emukc_gameplay` passes

---

### U2. Fix verify.rs unsafe unwrap calls and silent skip

**Goal:** Replace `.unwrap()` on fallible operations in verify.rs tests and make the topology test's silent skip visible.

**Requirements:** R2, R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/verify.rs`

**Approach:**
- Replace `ApiManifest::from_str(...).unwrap()` (line ~36) with `match` returning `MapCatalog::default()` on error + `eprintln!`
- Replace `build_final_map_catalog_from_repo_assets(...).unwrap()` (line ~42) with same pattern
- Add `eprintln!` or `#[ignore]` annotation to make the silent `MapCatalog::default()` return on missing data visible

**Patterns to follow:**
- Existing early-return pattern in same file for missing manifest/kcdata_root

**Test scenarios:**
- Happy path: test still passes when data is present
- Error path: test gracefully returns empty catalog when `ApiManifest::from_str` fails (malformed JSON)
- Error path: test gracefully returns empty catalog when `build_final_map_catalog_from_repo_assets` fails (I/O error)

**Verification:**
- `cargo test -p emukc_bootstrap battle_rules` passes

---

### U3. Document maelstrom transaction requirement

**Goal:** Add doc comment on `resolve_non_battle_node_effect` clarifying that the connection parameter must be a transaction for the maelstrom path to maintain atomicity.

**Requirements:** R4

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie/mod.rs`

**Approach:**
- Add doc comment on `resolve_non_battle_node_effect` function: "The `c` parameter must be a transaction connection when the maelstrom branch (event_id 3) is reachable. Per-ship resource deductions are applied individually; a non-transaction connection risks partial state on failure."

**Test scenarios:**
- Test expectation: none — documentation-only change

**Verification:**
- `cargo doc -p emukc_gameplay` succeeds without warnings on the annotated function

---

### U4. Add maelstrom test coverage

**Goal:** Cover the ammo-drain path (color_no == 4) and radar reduction tier table (1-6 ships with radar) with integration tests.

**Requirements:** R5, R6

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie_tests.rs`

**Approach:**
- Add test for maelstrom ammo drain: create sortie with cell color_no=4, verify each ship's ammo is reduced (not fuel)
- Add parameterized-style tests for radar reduction tiers: 1 ship with radar → 0.25, 2 → 0.40, ..., 5+ → 0.60. Verify fuel loss is reduced by the expected factor
- Use existing `sortie_tests.rs` patterns for test setup (in-memory DB, codex, sample ships)

**Execution note:** Write tests first against the existing implementation to confirm it behaves correctly.

**Patterns to follow:**
- Existing maelstrom test in `sortie_tests.rs` (single ship, no radar, fuel drain)
- `sample_ship()` and codex loading pattern used by other tests in the file

**Test scenarios:**
- Happy path: maelstrom with color_no=4 reduces ammo per ship
- Happy path: maelstrom with 1 radar ship reduces fuel by 25%
- Edge case: maelstrom with 6 radar ships → 0.60 reduction
- Edge case: maelstrom with 0 radar ships → full reduction (existing test covers this)
- Edge case: ship with 0 ammo/fuel entering maelstrom — no underflow

**Verification:**
- `cargo test -p emukc_gameplay -- sortie_tests` passes with new tests

---

### U5. Add kouku + shelling combined sinking protection test

**Goal:** Add a day-battle integration test verifying that flagship survives both kouku airstrike and shelling phases under sinking protection.

**Requirements:** R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie_tests.rs`

**Approach:**
- Create a test scenario: friendly fleet with flagship at very low HP (taiha), enemy fleet with CVL equipped with bombers
- Run full day battle simulation
- Assert flagship HP > 0 after both kouku and shelling phases complete
- Verify `api_fdam` reflects dealt damage (not raw overkill) across both phases

**Patterns to follow:**
- Existing `day_battle_all_friendly_survive_under_protection` test in `kouku.rs`
- Full sortie test setup pattern in `sortie_tests.rs`

**Test scenarios:**
- Happy path: flagship at taiha HP survives kouku + shelling, HP > 0 at battle end
- Edge case: kouku deals enough raw damage to "kill" flagship, but sinking protection caps it
- Integration: `api_fdam` across kouku and shelling phases both reflect capped damage

**Verification:**
- `cargo test -p emukc_gameplay -- sortie_tests` passes with new test
