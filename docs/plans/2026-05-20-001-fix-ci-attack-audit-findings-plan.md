---
title: "fix: CI attack system audit findings"
type: fix
status: active
date: 2026-05-20
origin: Code review audit of feat/vibe branch CI attack implementation
---

# fix: CI attack system audit findings

## Summary

Fix 2 P1 bugs and cleanup P2/P3 findings from the CI attack code review. The P1 bugs affect correctness of flagship special attacks and carrier day CI.

---

## Problem Frame

Code review of the CI attack system (plan 2026-05-14-002) found 2 correctness bugs and several code quality issues:
- NagatoMutsuBroadside (102) flagship fires only 1 hit instead of 2
- Carrier day CI has 100% trigger rate (no roll)
- Carrier night CI missing exempt ship list and sub-type resolution
- Dead code, duplicated functions, and minor cleanups

---

## Scope Boundaries

- Fix P1 bugs (#1, #2) and safe_auto P2/P3 items
- Carrier night CI exempt ships (#6) and sub-types (#7) deferred — requires ship ID verification against codex data
- Equipment helper extraction (#4) deferred — refactor for follow-up

---

## Implementation Units

### U1. Fix NagatoMutsuBroadside 2nd hit (P1 #1)

**Goal:** NagatoMutsuBroadside (102) flagship should fire 2 hits, same as NagatoClassBroadside (101).

**Requirements:** R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/special_attack.rs` (line 703)
- Test: same file, test section

**Approach:** Add `SpecialAttackType::NagatoMutsuBroadside` to the `num_hits` condition alongside `NagatoClassBroadside`.

**Test scenarios:**
- Happy path: NagatoMutsu (102) execution produces 2 hits for flagship, 1 for companion
- Verify api_at_type = 102 in output

**Verification:** `cargo test -p emukc_battle` passes, NagatoMutsu flagship produces 2 damage entries.

### U2. Add carrier CI trigger roll (P1 #2)

**Goal:** Carrier day CI (at_type=7) should roll trigger rate like artillery spotting, not auto-trigger on detection.

**Requirements:** R2, R6

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/day_cutin.rs` (resolve_day_attack, around line 370)
- Test: same file, test section

**Approach:** After detecting carrier CI, call `day_ci_trigger_rate` with `DayAttackType::CarrierCI` and roll before returning. On failure, fall through to normal attack.

**Test scenarios:**
- Happy path: Carrier CI detection succeeds but trigger roll fails → returns normal attack
- Happy path: Carrier CI detection + trigger succeeds → returns CarrierCI
- Edge case: Verify trigger rate is reasonable (not 100% or 0%)

**Verification:** `cargo test -p emukc_battle` passes, carrier CI no longer triggers 100% of the time.

### U3. Dead code and unused import cleanup (P1 #5)

**Goal:** Remove all dead code and unused imports flagged by the audit.

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/special_attack.rs` — remove `is_ship_id_opt`, unused import `BattlePhase`
- Modify: `crates/emukc_battle/src/simulation/day_cutin.rs` — remove `day_ci_accuracy_multiplier`, `DayAttackType::api_id`
- Modify: `crates/emukc_battle/src/simulation/shelling.rs` — remove unused import `DayAttackType`

**Approach:** Mechanical deletion. Run `cargo clippy` after to confirm zero warnings.

**Test expectation:** none — pure removal of unused code.

**Verification:** `cargo clippy --workspace` shows zero warnings for these files.

### U4. Minor P2/P3 cleanups

**Goal:** Fix low-effort P2/P3 items: identical functions, magic numbers, unused variables.

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/special_attack.rs` — merge `richelieu_rate`/`qe_rate` into one function, remove unused `flag_id` variable in `check_qe_attack`
- Modify: `crates/emukc_battle/src/simulation/night.rs` — extract magic number `412` to named constant `SKILLED_LOOKOUT_ID`

**Approach:** Mechanical refactor. `richelieu_rate` and `qe_rate` are identical — rename to `three_ship_special_rate` and share. Extract `412` to `const SKILLED_LOOKOUT_ID: i64 = 412;` near `AVIATION_PERSONNEL_IDS`.

**Test expectation:** none — behavior unchanged, existing tests still pass.

**Verification:** `cargo test -p emukc_battle` passes.

---

## Deferred to Follow-Up Work

- **Carrier night CI exempt ships** (Saratoga Mk.II, 赤城改二戊, 加賀改二戊): Need to verify exact ship IDs in codex, then add exemption list to `is_cv_night_ci_eligible`
- **Carrier night CI sub-type resolution** (5 sub-types with 1.18x-1.25x): Requires night plane icon type counting per slot and priority chain logic
- **Equipment helper deduplication**: Extract shared `count_main_guns`, `count_secondary_guns`, `has_radar`, `count_equipment_type` to shared module
- **Night hougeki friendly/enemy loop deduplication**: Larger refactor, risk of regression
