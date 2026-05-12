---
title: "fix: Address audit findings from 2026-05-12 commit review"
type: fix
status: active
date: 2026-05-12
---

# fix: Address audit findings from 2026-05-12 commit review

## Summary

Fix four findings from the audit of commits 269a595..d0afeb3: stale plan doc signature, misleading radar-tier test name with incomplete coverage, silent-skip of real failures in `verify.rs`, and a weak underflow assertion plus a clippy lint.

---

## Problem Frame

The audit of commits `269a595..d0afeb3` on branch `feat/vibe` surfaced four defects:

1. `docs/plans/2026-05-12-001-fix-review-findings-plan.md` describes `multi_label_index` with the pre-refactor signature — plan shipped after the refactor but was not updated to reflect actual impl.
2. `maelstrom_radar_reduces_fuel_loss_per_tier` claims "per tier" but only exercises 0-radar and 1-radar cases. The actual formula has 6 discrete reduction tiers (0, 1, 2, 3, 4, 5, 6+ ships) in `crates/emukc_gameplay/src/game/sortie/mod.rs:1316`.
3. `verify.rs` `load_catalog` swallows manifest parse errors and catalog build errors as `WARNING` + empty-catalog skip. Downstream tests early-return on empty catalog, so a broken parser or broken catalog builder passes CI silently. Violates fail-loud.
4. `maelstrom_zero_resource_ship_skips_loss_without_underflow` uses `if let Some(h) = happening { assert_eq!(h.amount, 0); }` — the `None` branch is unasserted, hiding intent drift. Also clippy warns on `ship_zero.clone()` (Copy type) at `sortie_tests.rs:745`.

---

## Requirements

- R1. Plan doc signature reference matches the landed implementation.
- R2. Radar reduction test covers all 7 branch arms of `match radar_ship_count` (0, 1, 2, 3, 4, 5, 6+) with asserted expected-loss values.
- R3. `verify.rs` fails loudly when manifest parse or catalog build fails. Missing `.data/temp` or `kc_data` still skips with a warning.
- R4. `maelstrom_zero_resource_ship_skips_loss_without_underflow` asserts the shape of `happening` unconditionally.
- R5. No new clippy warnings introduced by this plan; clear the `clone_on_copy` + `cloned_ref_to_slice_refs` warnings at `sortie_tests.rs:745`.

---

## Scope Boundaries

- Not refactoring the radar reduction formula or introducing a fixture layer
- Not touching pre-existing clippy warnings outside the 4 audit findings
- Not reworking `verify.rs` beyond the fail-loud change (no new test harness)
- Not rewriting `main-decoder` or battle assets
- Not updating the 5b539c2 commit message retroactively

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_gameplay/src/game/sortie/mod.rs:1308-1316` — radar_ship_count tier match: `0→0.0, 1→0.25, 2→0.40, 3→0.50, 4→0.55, 5→0.58, 6+→0.60`
- `crates/emukc_gameplay/src/game/sortie/mod.rs:1332` — loss formula `floor(stock * 0.30 * (1 - reduction))`
- `crates/emukc_gameplay/src/game/sortie_tests.rs:606-635` — existing `equip_radar_on_ship` helper
- `crates/emukc_gameplay/src/game/sortie_tests.rs:637-703` — current partial radar test (baseline for refactor)
- `crates/emukc_bootstrap/src/map_pipeline/verify.rs:30-60` — `load_catalog` error handling
- `crates/emukc_model/src/codex/map/types.rs:110` — actual `multi_label_index` signature

### Institutional Learnings

- `docs/plans/2026-05-12-001-fix-review-findings-plan.md` is a direct prior on the same surface; its signature field is the object of U1.

---

## Key Technical Decisions

- Test radar tiers with a N-ship fleet (N=0..=6) rather than N radars on 1 ship, because the formula keys on `radar_ship_count` (per-ship any-radar check), not radar count. This matches the source.
- Use `ship.fuel = 1000` via `ActiveModel` to make all 7 tiers produce distinguishable loss values. With fuel_max=20 the tiered values collide (3 & 3, 2 & 2 & 2). Expected values with stock=1000: `0→300, 1→225, 2→180, 3→150, 4→135, 5→126, 6→120`. Loss is computed per-ship then summed.
- Rename the test to `maelstrom_radar_reduces_fuel_loss_across_all_tiers` to match expanded coverage.
- Keep the missing-file skip in `verify.rs` as a warning (opt-in data dependency), but convert `ApiManifest::from_str` failure and `build_final_map_catalog_from_repo_assets` failure to `panic!` with the error message. A real parse error is a regression, not an environmental absence.
- For U4 `happening` shape: assert `happening.is_none()` — the spec is "no loss happening emitted when stock is 0", not "emit zero-amount happening". If impl actually emits `Some(0)`, revisit during work.

---

## Implementation Units

### U1. Correct stale `multi_label_index` signature in prior plan

**Goal:** Align `docs/plans/2026-05-12-001-fix-review-findings-plan.md:87` with the landed impl at `crates/emukc_model/src/codex/map/types.rs:110`.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `docs/plans/2026-05-12-001-fix-review-findings-plan.md`

**Approach:**
- Replace `pub fn multi_label_index(&self, label: &str) -> Vec<usize>` with `pub fn multi_label_index(&self) -> BTreeMap<String, Vec<i64>>`
- Update the one-line description so it still matches the method doc: "multi-valued index from `node_label` to matching `cell_no`s; preserves duplicate labels"
- Update the "Edge case: duplicate labels return multiple indices" test scenario to reference the returned `Vec<i64>` shape

**Patterns to follow:**
- Existing `MapVariantDefinition` method docstring style in `crates/emukc_model/src/codex/map/types.rs:103-130`

**Test scenarios:**
- Test expectation: none — documentation-only

**Verification:**
- `grep 'multi_label_index' docs/plans/2026-05-12-001-fix-review-findings-plan.md` shows the corrected signature
- No mention of `label: &str` parameter remains

---

### U2. Expand radar reduction test to cover all 7 tiers

**Goal:** Replace the partial `maelstrom_radar_reduces_fuel_loss_per_tier` with a test that exercises every arm of the `radar_ship_count` match in `sortie/mod.rs:1316`.

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie_tests.rs`

**Approach:**
- Rename test to `maelstrom_radar_reduces_fuel_loss_across_all_tiers`
- Build a 6-ship fleet using `context.add_ship(profile_id, 951)` six times, each assigned via separate `sign_up` + `new_profile` OR a single profile with 6 ships (single profile is simpler — check profile ship cap)
- Write a helper `equip_radar_on_nth_ship` or reuse `equip_radar_on_ship` in a loop to equip N of the 6 ships with radars
- Set each ship's `fuel` to 1000 directly via `ActiveModel` so loss values don't collide across tiers
- For each N in 0..=6:
  - Reset fuel on all ships to 1000
  - Equip exactly N ships with radars (others cleared)
  - Call `resolve_non_battle_node_effect` with the full fleet slice
  - Assert `happening.unwrap().amount == expected[N]` where `expected = [1800, 1575, 1440, 1500, 1485, 1512, 1440]`
  - (Total loss = sum over 6 ships of `floor(1000 * 0.30 * (1 - r))`. Per-ship loss is uniform because stock is uniform: `1800 = 6 * 300`, `1575 = 6 * floor(225) = 6 * 225`, `1440 = 6 * 240` — **recompute during implementation; treat these values as directional**)
- Execution note: recompute the expected table during implementation by running the formula — planning values are directional, not authoritative

**Execution note:** Compute expected-loss values by running the formula in code during U2 impl. Plan values are directional.

**Patterns to follow:**
- `equip_radar_on_ship` helper at `crates/emukc_gameplay/src/game/sortie_tests.rs:606`
- Existing fuel-override pattern at `crates/emukc_gameplay/src/game/sortie_tests.rs:681-684` (`ActiveValue::Set` via `into_active_model`)
- Existing fleet-iteration pattern in `resolve_non_battle_node_effect` callers

**Test scenarios:**
- Happy path: 7 sub-cases (N=0..=6), each asserting total fleet fuel loss matches the formula `sum(floor(stock * 0.30 * (1 - reduction[N])))`
- Happy path: at N=6 reduction caps at 0.60 (the `_ => 0.60` arm); verify adding a 7th radar produces identical loss to N=6 *(skip if fleet cap blocks 7-ship construction; note in a comment instead)*
- Edge case: N=0 baseline equals `6 * floor(1000 * 0.30) = 1800`, confirming no reduction is applied
- Integration: ship fuel is persisted post-call (read back from DB), not only reported in `happening.amount`

**Verification:**
- `cargo test -p emukc_gameplay maelstrom_radar_reduces_fuel_loss_across_all_tiers` passes
- Test name no longer claims "per_tier" without covering tiers
- Removing any arm of the `radar_ship_count` match makes the test fail

---

### U3. Fail loud on `verify.rs` parse and catalog build errors

**Goal:** Make `load_catalog` panic on real failures (manifest parse, catalog build) while preserving skip-on-missing-data for opt-in test data.

**Requirements:** R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/verify.rs`

**Approach:**
- Keep the missing-file early-return with `WARNING` prefix for both `manifest_path` not readable and `kcdata_root` not existing (opt-in data dependency — absent `.data/temp` is expected on fresh checkouts)
- Replace the `match` on `ApiManifest::from_str(&manifest_raw)` with `.expect(...)` or `unwrap_or_else(|e| panic!(...))` so a real parse regression fails the test
- Replace the `match` on `build_final_map_catalog_from_repo_assets(...)` similarly — if data is present but catalog build fails, that's a regression, not a skip
- Keep `MapCatalog::default()` only on the two file-absence skips

**Patterns to follow:**
- Original pre-5b539c2 pattern (`.unwrap()` was correct posture here — this unit partially reverts 5b539c2 but keeps the improved WARNING formatting)

**Test scenarios:**
- Happy path: with `.data/temp/start2.json` + `.data/temp/kc_data/` present and parseable, `load_catalog` returns a non-empty `MapCatalog`
- Error path: with `start2.json` present but malformed, `load_catalog` panics with a message mentioning "manifest parse failed" (not tested via a unit test — exercised by CI when the fixture breaks)
- Environmental skip: with `.data/temp/start2.json` absent, `load_catalog` returns `MapCatalog::default()` and prints a WARNING, and downstream tests early-return without failing

**Verification:**
- `cargo test -p emukc_bootstrap` still passes when `.data/temp` is absent (skip behavior preserved)
- `cargo test -p emukc_bootstrap` still passes when `.data/temp` is populated and valid (happy path)
- Injecting a corrupt `start2.json` causes `load_catalog` to panic rather than return default

---

### U4. Strengthen zero-resource assertion and clear clippy warnings

**Goal:** Remove the ambiguous `if let Some(h) = happening` branch and silence the clippy warnings the new tests introduced.

**Requirements:** R4, R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie_tests.rs`

**Approach:**
- In `maelstrom_zero_resource_ship_skips_loss_without_underflow`, replace:
  ```
  if let Some(h) = happening {
      assert_eq!(h.amount, 0);
  }
  ```
  with a direct assertion on the expected shape. Read `resolve_non_battle_node_effect` to determine whether a 0-amount `MaterialHappening` is emitted or whether `happening` is `None` for zero-stock ships. Assert whichever shape the impl guarantees — no ambiguous branching.
- Replace `&[ship_zero.clone()]` at `sortie_tests.rs:745` per clippy: `std::slice::from_ref(&ship_zero)` (also drops the `clone_on_copy` warning since `Model` is `Copy`)
- If other new-code clippy warnings exist in the maelstrom/kouku tests added by 09fb011, fix them in the same unit (scope: only warnings introduced by 09fb011, not pre-existing)

**Execution note:** Read `resolve_non_battle_node_effect` before choosing the assertion shape — do not guess.

**Patterns to follow:**
- Existing strict-assertion style in `maelstrom_drains_ship_resource_without_touching_profile_materials` (`sortie_tests.rs:481`)

**Test scenarios:**
- Happy path: 0-fuel ship at maelstrom cell produces `happening = None` *(or `Some(MaterialHappening { amount: 0, .. })` — whichever the impl emits; assert exactly one shape)*
- Happy path: ship fuel remains 0 after the call (no underflow, no negative wrap)
- Edge case: unchanged from current test — retain the 0-fuel underflow guard

**Verification:**
- `cargo test -p emukc_gameplay maelstrom_zero_resource_ship_skips_loss_without_underflow` passes
- `cargo clippy -p emukc_gameplay --tests` no longer reports `clone_on_copy` or `cloned_ref_to_slice_refs` on `sortie_tests.rs:745`
- Removing the underflow guard in `resolve_non_battle_node_effect` makes the test fail

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Fleet ship cap blocks 6-ship maelstrom fleet construction | Verify during U2 impl. If cap is hit, use the max allowed (likely 6 anyway — KC fleets are ≤6) and document the ceiling |
| U3 panic behavior makes CI harder to run without `.data/temp` | Missing-file branches remain as skip+WARNING. Only parse/build errors now panic — those indicate actual regressions |
| U4 impl-shape read (None vs Some(0)) drifts from current behavior | Execution note: read `resolve_non_battle_node_effect` before writing the assertion. If impl emits Some(0), assert that instead — but assert exactly one shape |
| Recomputed expected-loss values in U2 differ from plan directional values | Execution note: compute during impl. Planning values are directional, not binding |

---

## Sources & References

- Audit output in prior conversation turn (5 commits: 269a595, 5b539c2, df2658b, 09fb011, d0afeb3)
- Prior plan: `docs/plans/2026-05-12-001-fix-review-findings-plan.md`
- Radar tier source: `crates/emukc_gameplay/src/game/sortie/mod.rs:1308-1340`
