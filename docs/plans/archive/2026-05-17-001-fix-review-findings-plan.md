---
title: "fix: Code review findings — session drop, dead exports, missing tests, style nits"
type: fix
status: completed
date: 2026-05-17
origin: ce-code-review audit of feat/vibe recent 5 commits
---

# fix: Code review findings — session drop, dead exports, missing tests, style nits

## Summary

Fix 5 findings from the ce-code-review audit of the practice store migration, aerial combat restriction, and cfg_select! refactoring commits. One P0 bug (session silently dropped), two P1 gaps (missing integration test, dead export), and two P2/P3 nits.

---

## Problem Frame

The practice store migration (`af09672`) introduced a `take_pending_battle` call in `run_night_battle` that removes the session from the store. When `can_midnight` is false, the function returns `None` without re-inserting — the session is permanently lost. The old code used `get_mut` (in-place) so this path was safe.

Additionally, the audit found a dead prelude export (`TestPracticeStore`), missing practice lifecycle integration tests, an incomplete ship type list for aerial combat, and minor style issues.

---

## Requirements

- R1. `run_night_battle` preserves the battle session when `can_midnight` is false
- R2. Practice day→night→result lifecycle has integration test coverage
- R3. No dead exports in `emukc_gameplay` prelude
- R4. Hard tabs removed from start.rs test code
- R5. `PracticeRepository` has symmetric convenience methods

---

## Scope Boundaries

- AIR_COMBAT_SHIP_TYPES completeness (SSV, LHA) deferred — requires game data verification, not a code bug
- `sortie_store.rs` module extraction deferred — P2 advisory, not blocking. Extract `PracticeStore`/`TestPracticeStore`/`GLOBAL_PRACTICE_STORE` into `practice_store.rs`
- take+reinsert TOCTOU risk documented but not redesigned — acceptable under KanColle's serialized request model
- MSRV 1.95.0 CI verification deferred — infrastructure concern, not code
- Add `with_profile_lock` to `PracticeStore` for panic safety — deferred

### Deferred to Follow-Up Work

- Verify SSV/LHA participation in aerial combat against KanColle client data

---

## Context & Research

### Audit Findings Reference

Full audit results are in the ce-code-review run artifacts. Key cross-reviewer findings:

- **correctness + testing** both flagged the session drop bug at `orchestrate.rs:188` (confidence 100)
- **testing + maintainability** both flagged `TestPracticeStore` dead export (confidence 100)
- **maintainability + project-standards** both flagged take+reinsert TOCTOU (advisory, not blocking)

### Relevant Patterns

- `SortieRepository` in `crates/emukc_gameplay/src/game/battle/repository.rs` — reference trait pattern
- `SortieStore` in `crates/emukc_gameplay/src/game/sortie_store.rs` — reference store pattern
- Existing practice unit tests in `crates/emukc_gameplay/src/game/battle/practice/mod.rs` use `PracticeStore::new()` directly

---

## Key Technical Decisions

- **TD1. Re-insert on early return, not redesign to get+update.** The take+reinsert pattern is already established for the successful path. Adding a re-insert on the early return is the minimal fix. The old code used `HashMap::get_mut` directly (in-place). The new trait-based design only exposes `take` (removes) and `insert`. Redesigning to `get_mut` would require adding an `update_pending_battle` method to the trait and all impls — overkill for this bug.

- **TD2. Keep `TestPracticeStore` export for now, add usage.** Rather than removing the export (which would require re-adding it when a future `(Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore)` tuple `HasContext` impl lands — see `docs/plans/2026-05-05-007` U3 for the planned design), add a test that exercises it. The export is intentional scaffolding.

---

## Implementation Units

### U1. Fix session drop in run_night_battle

**Goal:** Re-insert the session before returning `None` when `can_midnight` is false.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`

**Approach:** After `take_pending_battle` and the `can_midnight` check, re-insert the session before returning `None`. This matches the re-insert pattern already used on the successful path at the end of the function.

**Test scenarios:**
- Unit test: call `run_night_battle` with a manually-constructed `PracticeBattleSession` where `can_midnight=false`, verify session is still retrievable from the store afterward. Use `TestPracticeStore::new()` for test isolation — each instance is independent so parallel runs don't share state. Pass `&store` as `&dyn PracticeRepository` to `run_night_battle`.
- This test should go in `crates/emukc_gameplay/src/game/battle/practice/mod.rs` tests

**Verification:** `cargo test -p emukc_gameplay practice` passes

---

### U2. Add practice day→night→result integration test

**Goal:** Cover the full practice battle lifecycle with the new trait-based store.

**Requirements:** R2

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_gameplay/tests/practice_battle.rs` (append new tests to existing file)

**Approach:** Add an integration test that runs practice day battle → midnight battle → battle result through `PracticeOps` trait methods using `PracticeStore`. Verify:
- Day response has valid fields
- Midnight response reflects night battle outcome
- Result snapshot contains correct win rank and exp after night battle
- Pending battle is cleaned up after result

Also add `TestPracticeStore` unit tests in a `#[cfg(test)] mod tests` block in `crates/emukc_gameplay/src/game/sortie_store.rs` (alongside the struct definition): insert→get→take cycle, empty take returns `None`, two instances are isolated.

**Test scenarios:**
- Happy path: day→night→result cycle with `PracticeStore::new()`
- Edge case: `can_midnight=false` — construct a `PracticeBattleSession` with `outcome.can_midnight = false` and call `run_night_battle` directly at the orchestration layer (not via `PracticeOps`). This avoids needing to control the battle simulation outcome. Verify the session remains in the store.
- Edge case: `practice_battle_result` without prior battle → `EntryNotFound` error
- Unit: `TestPracticeStore` insert→get→take returns correct values
- Unit: `TestPracticeStore` empty take returns `None`
- Unit: Two `TestPracticeStore` instances don't share state

**Verification:** `cargo test -p emukc_gameplay practice` passes; new test file runs green

---

### U3. Remove dead TestPracticeStore prelude export or add usage

**Goal:** Eliminate the dead export in the prelude.

**Requirements:** R3

**Dependencies:** U2 (if keeping export and adding tests that use it)

**Files:**
- Modify: `crates/emukc_gameplay/src/lib.rs`

**Approach:** If U2 adds `TestPracticeStore` usage, keep the export. Otherwise remove it from the prelude re-export. Per TD2, prefer keeping it and adding usage in U2.

**Test scenarios:**
- Existing tests compile and pass

**Verification:** `cargo test -p emukc_gameplay` passes; `rg "TestPracticeStore" crates/emukc_gameplay/` shows usage

---

### U4. Fix hard tabs in start.rs test

**Goal:** Replace hard tabs with soft tabs per project style.

**Requirements:** R4

**Dependencies:** None

**Files:**
- Modify: `src/bin/net/router/kcsapi/api_req_map/start.rs`

**Approach:** Replace hard tab characters on the test string literals (lines ~43-45) with soft tabs (4 spaces). These are continuation-aligned strings inside `serde_urlencoded::from_str` calls.

**Test scenarios:**
- Test expectation: none — whitespace-only change, existing test assertions unchanged

**Verification:** `rg $'\t' src/bin/net/router/kcsapi/api_req_map/start.rs` returns no hits

---

### U5. Add clear_pending_result convenience method to PracticeRepository

**Goal:** Symmetric API with `clear_pending_battle`.

**Requirements:** R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/battle/practice_repository.rs`

**Approach:** Add `fn clear_pending_result(&self, profile_id: i64) { self.take_pending_result(profile_id); }` as a default impl, mirroring the existing `clear_pending_battle`.

**Test scenarios:**
- Test expectation: none — trivial default method delegation

**Verification:** `cargo test -p emukc_gameplay` passes

---

## System-Wide Impact

- `PracticeRepository` trait gains a new default method — no breaking change, existing impls inherit it automatically
- `run_night_battle` behavior change: session preserved on early return (fix, not regression)
- No API surface changes — all fixes are internal to gameplay/battle layer

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Session re-insert on early return masks a logic issue where `can_midnight` is unexpectedly false | The re-insert is correct — the old code also kept the session; this restores parity |
| Integration test requires Codex data (`.data/codex`) | Same requirement as existing practice tests — not a new dependency |

---

## Sources & References

- Audit run artifacts: `/tmp/compound-engineering/ce-code-review/20260517-224019-a6ff119c/`
- Supersedes findings from: `docs/plans/2026-05-16-001-fix-practice-state-and-sortie-cleanup-plan.md`
- Reference pattern: `crates/emukc_gameplay/src/game/battle/repository.rs` (SortieRepository)
