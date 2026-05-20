---
title: "fix: Migrate practice battle state to trait-based store and remove dead formation_id"
type: fix
status: completed
date: 2026-05-16
supersedes: docs/plans/2026-05-05-007-fix-practice-state-and-sortie-cleanup-plan.md
---

# fix: Migrate practice battle state to trait-based store and remove dead formation_id

## Summary

Remove the unused `formation_id` parameter from `start_sortie`, then migrate practice battle pending-state from process-global `LazyLock<Mutex<HashMap>>` statics to a `PracticeRepository` trait + `PracticeStore` mirroring the existing `SortieRepository` pattern. Add test context support for isolated practice stores so tests can run in parallel without shared mutable state.

---

## Problem Frame

Practice battles still use two process-global `LazyLock<Mutex<HashMap>>` statics (`PENDING_PRACTICE_RESULTS`, `PENDING_PRACTICE_BATTLES`), while sortie battles were already migrated to the `SortieRepository` trait + `SortieStore` pattern. This creates architectural inconsistency and prevents test isolation — tests using tuple `HasContext` impls share mutable practice state across parallel runs. Additionally, `start_sortie` accepts an unused `formation_id` parameter (explicitly ignored with `let _ = formation_id;`).

---

## Requirements

- R1. `start_sortie` no longer accepts an unused `formation_id` parameter
- R2. Practice battle pending state uses a trait-based `PracticeRepository` with instance-scoped and test-isolated concrete implementations
- R3. All existing tests pass; new tests cover the migrated practice store
- R4. Tuple `HasContext` impls support injected test stores for both sortie and practice

---

## Scope Boundaries

- No changes to battle core logic (damage formulas, attack resolution)
- No changes to sortie routing or map topology
- `EnemyShipSunk` dispatch in practice battles deferred — requires game design decision on whether practice sinks should count toward sink-count quests
- `SlotItemImproved` documentation deferred — will be added naturally when improvement system is implemented
- `locked_enemy_composition` clone optimization deferred — P3, current `.clone()` is correct if not optimal

### Deferred to Follow-Up Work

- `EnemyShipSunk` quest event dispatch in practice battle result (design decision needed)
- `locked_enemy_composition` `.take()` optimization in sortie battle flow
- `SlotItemImproved` hook-stub documentation

---

## Context & Research

### Relevant Code and Patterns

- **`SortieRepository` trait** (`crates/emukc_gameplay/src/game/battle/repository.rs`): 9-method trait for sortie state access. Implemented by `SortieStore` (production) and `TestSortieStore` (tests). This is the reference pattern for `PracticeRepository`.
- **`SortieStore`** (`crates/emukc_gameplay/src/game/sortie_store.rs`): Holds `active_sorties`, `pending_results`, `pending_battles` as `Mutex<HashMap>` maps. `GLOBAL_SORTIE_STORE` provides process-global fallback for tuple `HasContext` impls.
- **`HasContext`** (`crates/emukc_gameplay/src/gameplay.rs`): Central trait with `db()`, `codex()`, `sortie_store()`. Four tuple impls fall back to `GLOBAL_SORTIE_STORE`. Will gain `practice_store()` method.
- **`PENDING_PRACTICE_BATTLES`** (`crates/emukc_gameplay/src/game/battle/practice/mod.rs:162`): Process-global static to be replaced.
- **`PENDING_PRACTICE_RESULTS`** (`crates/emukc_gameplay/src/game/practice.rs:43`): Process-global static to be replaced.
- **`State`** (`src/bin/state/mod.rs`): Holds `Arc<SortieStore>`, will gain `Arc<PracticeStore>`.

### Institutional Learnings

- **SortieStore migration precedent**: The `SortieStore` → `SortieRepository` migration established the reference pattern this plan copies for practice battles.
- **SortieStore transaction ordering**: Store mutations happen before `tx.commit()` — practice migration should preserve the same convention.

---

## Key Technical Decisions

- **TD1. Separate `PracticeRepository` trait, not lumped into `SortieRepository`.** Practice and sortie battle session/result types are different (`PracticeBattleSession` vs `SortieBattleSession`). Trait separation keeps each focused. `HasContext` gets a new `practice_store()` method with `GLOBAL_PRACTICE_STORE` fallback.
- **TD2. Remove `formation_id` from `start_sortie`.** KanColle clients send `formation_id` at each battle endpoint, not at sortie start. The parameter is dead code.
- **TD3. Add `(Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore)` tuple impl.** Existing `(Arc<DbConn>, Arc<Codex>)` impl is left intact for backward compat. New 4-tuple impl injects both test stores for full isolation.

---

## Open Questions

### Resolved During Planning

- **Should practice use `SortieRepository` or a separate trait?** → Separate `PracticeRepository` (TD1). Types differ, concerns are distinct.
- **Should `formation_id` be removed or implemented?** → Removed (TD2). KanColle protocol sends it at each battle endpoint, not at start.

### Deferred to Implementation

- **Exact method names for `PracticeRepository`** — follow `SortieRepository` convention with practice-specific types
- **Whether `practice_midnight_battle` needs store method for in-place mutation** — current code does `.get_mut()` on the results map; the trait may need a `update_pending_result` method or the function can take+reinsert

---

## Implementation Units

### U1. Remove unused `formation_id` from `start_sortie`

**Goal:** Eliminate the dead `formation_id` parameter from `start_sortie` signature and all call sites.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie/mod.rs` (trait signature + blanket impl)
- Modify: `crates/emukc_gameplay/src/gameplay.rs` (trait signature if duplicated)
- Modify: `src/bin/net/router/kcsapi/api_req_map/start.rs` (handler, remove from Params)
- Modify: `src/bin/net/router/kcsapi/api_req_map/mod.rs` (integration tests constructing `start::Params`)
- Modify: `tests/gameplay_tests/map/unlock.rs` (test calls)
- Modify: `tests/gameplay_tests/map/non_boss_pending.rs` (test calls)
- Modify: `tests/gameplay_tests/map/sortie_battle.rs` (test calls)
- Modify: `tests/gameplay_tests/map/retreat.rs` (test calls)

**Approach:**
- Remove `formation_id: i64` from `SortieOps::start_sortie` trait method
- Remove from blanket impl and `let _ = formation_id;`
- Remove `api_formation_id` from handler `Params` struct
- Update all test call sites

**Patterns to follow:**
- Grep `formation_id` across all crate files to find every reference

**Test scenarios:**
- Happy path: existing sortie start tests pass after arg removal
- Backward compat: `serde_urlencoded` ignores unknown fields — clients still sending `api_formation_id` are unaffected

**Verification:**
- `rg "formation_id" crates/emukc_gameplay/ src/bin/ tests/` returns no hits in start_sortie chain
- `cargo test` passes

---

### U2. Create `PracticeRepository` trait, `PracticeStore`, and migrate all practice code

**Goal:** Replace `PENDING_PRACTICE_RESULTS` and `PENDING_PRACTICE_BATTLES` globals with a trait-based store mirroring `SortieRepository`. Wire into `HasContext` and `State`. Add test-isolated context.

**Requirements:** R2, R3, R4

**Dependencies:** None (independent of U1)

**Files:**
- Create: `crates/emukc_gameplay/src/game/battle/practice_repository.rs`
- Modify: `crates/emukc_gameplay/src/game/sortie_store.rs` (add `PracticeStore`, `TestPracticeStore`, `GLOBAL_PRACTICE_STORE`)
- Modify: `crates/emukc_gameplay/src/gameplay.rs` (add `practice_store()` to `HasContext`, add 4-tuple impl)
- Modify: `crates/emukc_gameplay/src/game/battle/mod.rs` (register `practice_repository` module)
- Modify: `crates/emukc_gameplay/src/game/battle/practice/mod.rs` (remove `PENDING_PRACTICE_BATTLES` static and free functions)
- Modify: `crates/emukc_gameplay/src/game/practice.rs` (replace `PENDING_PRACTICE_RESULTS` with `self.practice_store()` calls)
- Modify: `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` (accept `&dyn PracticeRepository` parameter)
- Modify: `src/bin/state/mod.rs` (add `Arc<PracticeStore>` to `State`)
- Modify: `crates/emukc_gameplay/src/game/mod.rs` (re-export `PracticeStore`)
- Modify: `crates/emukc_gameplay/src/lib.rs` (add `PracticeRepository`, `TestPracticeStore` to prelude)
- Test: `crates/emukc_gameplay/tests/practice_battle.rs`
- Modify: `tests/gameplay_tests/mod.rs` (update test helper) — actually `tests/gameplay_tests.rs` (TestContext + HasContext impl)

**Approach:**
- Define `PracticeRepository` trait with methods for practice-specific types (`PracticeBattleSession`, `PracticeBattleResultSnapshot`): get/insert/take for pending battles and pending results, plus update_pending_result for midnight battle in-place mutation
- Add `PracticeStore` to `sortie_store.rs` with two `Mutex<HashMap<i64, _>>` maps
- Add `TestPracticeStore` wrapper (same delegation pattern as `TestSortieStore`)
- Add `GLOBAL_PRACTICE_STORE` static for tuple `HasContext` fallback (matching `GLOBAL_SORTIE_STORE` pattern)
- Add `practice_store()` as a required method on `HasContext` (matching `sortie_store()` pattern), update all 6 existing impls
- Wire `State` to hold and expose `Arc<PracticeStore>`
- In `PracticeOps` blanket impl, replace all `PENDING_PRACTICE_RESULTS.lock().unwrap()` calls with `self.practice_store()` trait calls
- In orchestration functions, accept `&dyn PracticeRepository` instead of accessing globals
- Remove `PENDING_PRACTICE_BATTLES` static, `clear_pending_practice_battle`, `pending_practice_battle` free functions
- Add `HasContext` impl for `(Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore)` injecting both test stores

**Patterns to follow:**
- `SortieRepository` trait in `battle/repository.rs` — mirror method names and semantics
- `SortieStore` / `TestSortieStore` in `sortie_store.rs` — mirror struct layout and delegation pattern
- `SortieOps` blanket impl in `sortie/mod.rs` — how it calls `self.sortie_store().*` methods

**Test scenarios:**
- Happy path: `PracticeStore::new()` creates empty store; insert → get → take cycle works
- Edge case: `take_pending_result` on empty store returns `None`
- Edge case: insert overwrites existing entry for same profile_id
- Integration: Exercise day battle → midnight battle → battle_result cycle works end-to-end with trait-based store
- Edge case: battle_result without prior battle returns error
- Test isolation: Two `TestPracticeStore` instances don't share state
- Regression: existing `(Arc<DbConn>, Arc<Codex>)` context tests still pass

**Verification:**
- `rg "PENDING_PRACTICE"` returns no results (globals removed)
- `rg "clear_pending_practice_battle"` returns no results (function removed)
- `cargo test -p emukc_gameplay practice` passes
- `cargo test --test gameplay_tests` passes

---

## System-Wide Impact

- **Interaction graph:** `HasContext` gains `practice_store()` — every impl (4 tuple contexts, `State`, `TestContext`) must provide it. The current `sortie_store()` is a required method (no default impl), so `practice_store()` should follow the same pattern and require explicit impls for consistency. All 6 `HasContext` impls must be updated.
- **API surface parity:** `SortieOps::start_sortie` signature changes — all callers must be updated (internal trait, single blanket impl, ~5 call sites).
- **Integration coverage:** Practice battle day→night→result lifecycle is the key cross-layer scenario covered by integration tests.
- **Unchanged invariants:** Battle damage calculation, quest progress calculation, fleet composition validation are untouched.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Practice store migration introduces regression in exercise battle lifecycle | End-to-end test coverage of day→night→result cycle |
| `formation_id` removal breaks clients still sending the field | `serde_urlencoded` ignores unknown fields by default |
| `practice_midnight_battle` needs in-place mutation of pending result — trait may need update method | Evaluate during implementation; add `update_pending_result` if needed, or take+reinsert pattern |

---

## Sources & References

- Supersedes: `docs/plans/2026-05-05-007-fix-practice-state-and-sortie-cleanup-plan.md`
- Reference pattern: `crates/emukc_gameplay/src/game/battle/repository.rs` (SortieRepository trait)
- Reference pattern: `crates/emukc_gameplay/src/game/sortie_store.rs` (SortieStore + TestSortieStore)
- Current practice impl: `crates/emukc_gameplay/src/game/practice.rs`
