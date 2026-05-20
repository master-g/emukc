---
title: "fix: Migrate practice battle state to trait-based store, add EnemyShipSunk coverage, and clean up dead formation_id parameter"
type: fix
status: superseded
date: 2026-05-05
---

# fix: Migrate practice battle state to trait-based store, add EnemyShipSunk coverage, and clean up dead formation_id parameter

## Summary

Migrate practice/exercise battle pending-state from process-global `LazyLock<Mutex<HashMap>>` statics to a trait-based `PracticeRepository` + `PracticeStore` architecture mirroring the existing `SortieRepository` pattern. Add `EnemyShipSunk` quest event dispatch to practice battle result (currently only sortie fires it). Remove the unused `formation_id` parameter from `start_sortie`. Optimize excessive `locked_enemy_composition` cloning in the sortie flow. Add test context support for isolated state. Document `SlotItemImproved` as intentionally deferred hook-stub.

---

## Problem Frame

A code review of the `refactor/map` branch found six issues in the gameplay layer:

1. Practice battles still use old-style process-global `LazyLock<Mutex<HashMap>>` statics (`PENDING_PRACTICE_RESULTS`, `PENDING_PRACTICE_BATTLES`), while sortie battles have already been migrated to the `SortieRepository` trait + `SortieStore`/`TestSortieStore` pattern. This creates inconsistency and prevents test isolation.
2. `EnemyShipSunk` quest events are dispatched in sortie battle result but not in practice battle result, creating a feature gap for sink-count quests that may be completable in exercises (verification needed).
3. The `formation_id` parameter in `start_sortie` is accepted, stored as `let _ = formation_id;`, and never used — formation is provided separately by clients at each battle endpoint.
4. `GLOBAL_SORTIE_STORE` is used as a fallback for tuple-based `HasContext` impls in tests, but as a process-global singleton it can leak state across parallel tests.
5. `locked_enemy_composition` on `ActiveSortieState` is cloned multiple times per sortie lifecycle when the value could be taken and restored.
6. `SlotItemImproved` quest event is defined and fully tested but has no gameplay dispatch site because slot-item improvement is not yet implemented — this is intentional (hook-stub).

---

## Requirements

- R1. Practice battle pending state uses a trait-based `PracticeRepository` with instance-scoped and test-isolated concrete implementations
- R2. `EnemyShipSunk` quest events fire in practice battle result when enemy ships are sunk
- R3. `start_sortie` no longer accepts an unused `formation_id` parameter
- R4. Tuple context `HasContext` impls support injected test stores for both sortie and practice
- R5. `GLOBAL_SORTIE_STORE` is deprecated with migration guidance
- R6. `locked_enemy_composition` cloning in sortie battle flows is reduced
- R7. All existing tests continue to pass, and new tests cover the practice `EnemyShipSunk` path
- R8. `SlotItemImproved` is documented as intentionally deferred

---

## Scope Boundaries

- Modifications are limited to `crates/emukc_gameplay/src/game/` (gameplay layer), `crates/emukc_gameplay/tests/` (tests), `src/bin/` (HTTP handlers), and `tests/gameplay_tests/` (integration tests)
- No changes to battle core logic (damage formulas, attack resolution, display_damage)
- No changes to sortie routing or map topology

### Deferred to Follow-Up Work

- Complete `SlotItemImproved` dispatch once slot-item improvement gameplay is implemented — add event construction at the completion handler
- Full audit of whether `EnemyShipSunk` should also fire for enemy ships sunk by friendly AI/escort in combined fleet scenarios (currently only sortie single-fleet sinks are dispatched)

---

## Context & Research

### Relevant Code and Patterns

- **`SortieRepository` trait** (`crates/emukc_gameplay/src/game/battle/repository.rs`): 9-method trait for sortie state access (active sorties, pending battles, pending results). Implemented by `SortieStore` (production) and `TestSortieStore` (tests). Mirrored by this plan for practice.
- **`SortieStore`** (`crates/emukc_gameplay/src/game/sortie_store.rs`): Three `Mutex<HashMap<i64, T>>` maps — `active_sorties`, `pending_results`, `pending_battles`. Also holds the `GLOBAL_SORTIE_STORE` fallback.
- **`HasContext`** (`crates/emukc_gameplay/src/gameplay.rs`): Central trait with `db()`, `codex()`, and `sortie_store()`. Binary-crate `State` overrides `sortie_store()` with an instance-scoped store. Tuple impls fall back to `GLOBAL_SORTIE_STORE`.
- **Practice battle orchestration** (`crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs`): `run_day_battle`/`run_night_battle` construct `PracticeBattleSession`/`PracticeBattleResultSnapshot` and interact with global statics directly.
- **Quest event matcher** (`crates/emukc_model/src/thirdparty/quest/matcher.rs`): `QuestActionEvent::EnemyShipSunk { ship_stype }` is matched by `Kc3rdQuestCondition::Sink(_, count)`. Dispatch pattern: for each enemy ship with `hp <= 0`, fire event.
- **`EnemyComposition`** (`crates/emukc_model/src/codex/map/types.rs`): Small struct — `ship_ids: Vec<i64>`, a few other fields. Candidate for `Copy` derive or `.take()` accessor.

### Institutional Learnings

- **SortieStore transaction consistency** (audit.md #4): Store mutations happen before `tx.commit()` — if commit fails, in-memory state is already advanced but DB didn't persist. Practice migration should preserve the same ordering convention for now; a separate plan should address this across both stores.
- **Route predicate keying** (audit.md #1): Routing rule predicate grouping has a correctness bug where same-predicate-kind rules with different values are incorrectly merged. This is orthogonal to the practice migration and should be addressed separately.
- **Global state migration precedence**: The `SortieStore` → `SortieRepository` migration (commit history: `d4de3ff`, `d3bc6d4`) established the reference pattern this plan copies for practice battles.

### External References

- **KanColle API protocol**: `formation_id` is sent by the client at each battle endpoint (`api_req_sortie/battle`, `api_req_practice/battle`), not predetermined at sortie/start. The `start_sortie` parameter is genuinely unused on the emulator side.
- **KanColle quest mechanic**: Sink-count quests (`撃沈任務`) increment per enemy ship sunk. Whether practice/exercise battles should count is a design decision — the plan adds the dispatch capability; the condition matcher already supports it.

---

## Key Technical Decisions

- **TD1. Create `PracticeRepository` as a separate trait, not lumped into `SortieRepository`.** The practice and sortie battle session/result types are different (`PracticeBattleSession` vs `SortieBattleSession`), and the repositories serve different lifecycle owners. Trait separation keeps each focused. `HasContext` gets a new `practice_store()` method with a `GLOBAL_PRACTICE_STORE` fallback.
- **TD2. Add `enemy_nowhps` and `enemy_ship_types` to `PracticeBattleResultSnapshot`.** These fields are required for `EnemyShipSunk` dispatch and mirror the sortie `SortieBattleResultSnapshot` structure. The practice battle orchestration already calculates enemy HP after battle — these values just need to be captured.
- **TD3. Remove `formation_id` from `start_sortie` rather than implementing it.** Research confirms KanColle clients send `formation_id` at each battle endpoint, not at sortie start. The parameter is dead code. Remove from trait, blanket impl, and all call sites (HTTP handler, tests).
- **TD4. Optimize via `.take()` pattern, not `Copy` derive.** `EnemyComposition` contains a `Vec<i64>` — deriving `Copy` would require making `EnemyComposition` fields all `Copy`, which is fragile for future changes. The `.take()` + restore pattern is safer and communicates intent clearly.
- **TD5. Add second test tuple context, not deprecation facade.** Rather than `#[deprecated]` on `GLOBAL_SORTIE_STORE`, add a new tuple context variant `(Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore)` with a `HasContext` impl that injects the test stores. The existing `(Arc<DbConn>, Arc<Codex>)` impl is left intact for backward compat but documented as using shared global state.

---

## Open Questions

### Resolved During Planning

- **Should practice use `SortieRepository` or a separate trait?** → Separate `PracticeRepository` trait (TD1). The types differ and the concerns are distinct.
- **Should `formation_id` be removed or implemented?** → Removed (TD3). KanColle protocol sends it at each battle endpoint, not at start.
- **Can `EnemyComposition` derive `Copy`?** → No, it contains `Vec<i64>`. Use `.take()` pattern instead (TD4).
- **Is `SlotItemImproved` a real bug?** → No. Confirmed as intentional hook-stub from `openspec/changes/archive/2026-04-19-quest-trigger-coverage/design.md`. The improvement system is not yet implemented.

### Deferred to Implementation

- **Exact naming of `PracticeRepository` methods** — follow `SortieRepository` convention but with practice-specific types
- **Whether `enemy_ship_types` values are stype or mst_id** — the sortie code uses `stype` from `KcApiShip.api_stype`; verify practice enemy ship data has the same shape
- **Exact location of `EnemyShipSunk` dispatch in practice result flow** — after result stats update, before quest progress call; same ordering as sortie

---

## Implementation Units

- U1. **Remove unused `formation_id` from `start_sortie`**

**Goal:** Eliminate the dead `formation_id` parameter from the `start_sortie` signature and all call sites.

**Requirements:** R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/gameplay.rs` (trait signature)
- Modify: `crates/emukc_gameplay/src/game/sortie.rs` (blanket impl, remove `let _ = formation_id;`)
- Modify: `src/bin/net/router/kcsapi/api_req_map/start.rs` (handler call site, remove from params)
- Modify: `crates/emukc_gameplay/tests/sortie_battle.rs` (test calls)
- Modify: `tests/gameplay_tests/map/unlock.rs` (test calls)

**Approach:**
- Remove `formation_id: i64` from the `start_sortie` method in `SortieOps` trait
- Remove `formation_id` from the blanket impl function signature and the `let _ = formation_id;` line
- Remove `api_formation_id` field from the HTTP handler's `Params` struct (and its `default_formation_id()` helper)
- Update all test call sites to remove the `formation_id` argument

**Patterns to follow:**
- Existing trait method removal pattern — search for `formation_id` across all crate files to find every reference

**Test scenarios:**
- Happy path: Sortie starts successfully without formation_id parameter (existing tests pass after arg removal)
- Edge case: `api_formation_id` is still present in incoming HTTP requests — `serde_urlencoded` will ignore unknown fields silently (axum Form deserialization is non-strict by default), so backward-compatible with clients still sending the field
- Test expectation: All existing sortie start tests pass with the updated signature

**Verification:**
- `cargo build` succeeds with no `formation_id` references in the `start_sortie` call chain
- Existing sortie tests pass unchanged except for parameter removal

---

- U2. **Create `PracticeRepository` trait and `PracticeStore`**

**Goal:** Introduce a trait-based interface for practice battle state management, mirroring `SortieRepository`.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Create: `crates/emukc_gameplay/src/game/battle/practice_repository.rs`
- Modify: `crates/emukc_gameplay/src/game/sortie_store.rs` (add `PracticeStore` and `GLOBAL_PRACTICE_STORE`)
- Modify: `crates/emukc_gameplay/src/gameplay.rs` (add `practice_store()` to `HasContext`)
- Modify: `crates/emukc_gameplay/src/game/mod.rs` (register new module)
- Modify: `crates/emukc_gameplay/src/lib.rs` (if needed)
- Modify: `src/bin/state/mod.rs` (add practice store to `State`)

**Approach:**
- Define `PracticeRepository` trait with methods for practice-specific types:
  ```text
  trait PracticeRepository: Send + Sync {
      fn get_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession>;
      fn insert_pending_battle(&self, profile_id: i64, session: PracticeBattleSession);
      fn take_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession>;
      fn get_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot>;
      fn insert_pending_result(&self, profile_id: i64, result: PracticeBattleResultSnapshot);
      fn take_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot>;
      fn clear_pending_battle(&self, profile_id: i64);  // convenience: remove without returning
  }
  ```
- Add `PracticeStore` struct to `sortie_store.rs` with `Mutex<HashMap<i64, PracticeBattleSession>>` and `Mutex<HashMap<i64, PracticeBattleResultSnapshot>>`
- Add `TestPracticeStore` wrapper with the same delegation pattern as `TestSortieStore`
- Add `GLOBAL_PRACTICE_STORE` static for tuple context fallback
- Add `practice_store()` method to `HasContext` trait with default impl returning `&GLOBAL_PRACTICE_STORE`
- Wire `State` to hold and expose `PracticeStore`

**Patterns to follow:**
- `SortieRepository` trait (`battle/repository.rs`) — mirror method names and semantics
- `SortieStore` / `TestSortieStore` in `sortie_store.rs` — mirror struct layout, delegate pattern, and `Default` impl

**Test scenarios:**
- Happy path: `PracticeStore::new()` creates empty store; `insert_pending_battle` stores a session; `take_pending_battle` retrieves and removes it; `get_pending_battle` retrieves without removing; `clear_pending_battle` removes without returning
- Edge case: `take_pending_battle` on empty store returns `None`
- Edge case: `insert_pending_battle` overwrites existing session for same profile_id
- Test expectation: `TestPracticeStore` instances are independent (two stores don't share state)

**Verification:**
- `cargo build` succeeds with new trait and store structs
- Unit tests (inline `#[cfg(test)]` in `sortie_store.rs`) pass for all store methods

---

- U3. **Migrate practice battle code to use `PracticeRepository`**

**Goal:** Replace all direct references to `PENDING_PRACTICE_RESULTS` and `PENDING_PRACTICE_BATTLES` globals with `self.practice_store()` calls through the trait.

**Requirements:** R1

**Dependencies:** U2

**Files:**
- Modify: `crates/emukc_gameplay/src/game/practice.rs` (replace globals in `PracticeOps` blanket impl)
- Modify: `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` (replace `PENDING_PRACTICE_BATTLES` access with store parameter)
- Modify: `crates/emukc_gameplay/src/game/battle/practice/mod.rs` (remove `PENDING_PRACTICE_BATTLES` static, remove `clear_pending_practice_battle` free function)
- Modify: `crates/emukc_gameplay/tests/practice_battle.rs` (update test store usage)
- Remove: `PENDING_PRACTICE_RESULTS` static from `practice.rs`
- Remove: `PENDING_PRACTICE_BATTLES` static from `battle/practice/mod.rs`

**Approach:**
- In `PracticeOps` blanket impl, replace `PENDING_PRACTICE_RESULTS.lock().unwrap().insert(profile_id, snapshot)` with `self.practice_store().insert_pending_result(profile_id, snapshot)`
- Replace `PENDING_PRACTICE_RESULTS.lock().unwrap().remove(&profile_id)` with `self.practice_store().take_pending_result(profile_id)`
- In orchestration functions (`run_day_battle`, `run_night_battle`), accept `&dyn PracticeRepository` as a parameter instead of accessing globals
- Update `practice_battle` and `practice_battle_result` to pass `self.practice_store()` to orchestration
- Remove `clear_pending_practice_battle` free function — replaced by `practice_store().clear_pending_battle(profile_id)`

**Patterns to follow:**
- SortieOps blanket impl in `sortie.rs` — how it calls `self.sortie_store().*` methods
- Sortie orchestration in `battle/sortie/orchestrate.rs` — how it receives store as parameter

**Test scenarios:**
- Happy path: Exercise day battle → midnight battle → battle_result cycle works end-to-end with trait-based store
- Edge case: Starting a new exercise battle while a previous one has pending result clears the old state
- Edge case: Calling battle_result without a prior battle returns `EntryNotFound` error
- Test expectation: `cargo test -p emukc_gameplay practice_battle` passes

**Verification:**
- `rg "PENDING_PRACTICE"` returns no results (except in U2's new store code and comments)
- `rg "clear_pending_practice_battle"` returns no results (function removed)
- All existing practice tests pass

---

- U4. **Add `EnemyShipSunk` quest event dispatch to practice battle result**

**Goal:** Fire `QuestActionEvent::EnemyShipSunk` for each enemy ship sunk in practice battles, mirroring sortie behavior.

**Requirements:** R2

**Dependencies:** U3 (needs `PracticeBattleResultSnapshot` access pattern)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/battle/practice/mod.rs` (add `enemy_nowhps` and `enemy_ship_types` to `PracticeBattleResultSnapshot`)
- Modify: `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` (populate new snapshot fields during battle)
- Modify: `crates/emukc_gameplay/src/game/practice.rs` (add `EnemyShipSunk` dispatch loop in `practice_battle_result`)
- Test: `crates/emukc_gameplay/tests/practice_battle.rs`

**Approach:**
- Add `enemy_ship_types: Vec<i64>` and `enemy_nowhps: Vec<i64>` to `PracticeBattleResultSnapshot`
- In orchestration (`run_day_battle`, `run_night_battle`), populate these fields from the enemy battle runtime ships after battle resolution
- In `practice_battle_result` handler, after calling `update_practice_result_stats`, add the sink dispatch loop:
  ```text
  for each enemy ship i where snapshot.enemy_nowhps[i] <= 0:
      let event = QuestActionEvent::EnemyShipSunk { ship_stype: snapshot.enemy_ship_types[i] };
      update_quest_progress_for_action(&tx, codex, profile_id, &event).await?;
  ```
- This mirrors the sortie sink dispatch at `sortie.rs:596-605`

**Patterns to follow:**
- Sortie `EnemyShipSunk` dispatch in `sortie_results` handler (`sortie.rs:596-605`)
- Existing `ExerciseBattleCompleted` event dispatch in `practice_battle_result` (`practice.rs:188-189`)

**Test scenarios:**
- Happy path: Enemy ship with `hp <= 0` after practice battle → `EnemyShipSunk` event fires with correct `stype`
- Edge case: No enemy ships sunk → no `EnemyShipSunk` events fired, no progress changed
- Edge case: Multiple enemy ships sunk → one event per sunk ship
- Integration: Sink quest with `Kc3rdQuestCondition::Sink(ShipType(vec![2]), 1)` progresses by 1 when a DD-type enemy is sunk in practice
- Test expectation: New test in `practice_battle.rs` verifies `EnemyShipSunk` dispatch via the quest progress API

**Verification:**
- `cargo test -p emukc_gameplay practice_battle` includes sink dispatch assertions
- Manual verification: sink a ship in practice → quest progress decrements (if accepted quest exists)

---

- U5. **Add test context with isolated stores**

**Goal:** Provide a `HasContext` impl for a tuple context that injects `TestSortieStore` and `TestPracticeStore`, so tests can run in parallel without shared mutable state.

**Requirements:** R4, R5

**Dependencies:** U2 (needs `TestPracticeStore`), U3 (practice migration complete)

**Files:**
- Modify: `crates/emukc_gameplay/src/gameplay.rs` (add new tuple `HasContext` impl)
- Modify: `crates/emukc_gameplay/src/game/sortie_store.rs` (add deprecation docs to `GLOBAL_SORTIE_STORE`)
- Modify: `crates/emukc_gameplay/tests/all_in_one.rs` (migrate to test tuple context if applicable)
- Modify: `tests/gameplay_tests/mod.rs` (migrate test helper to use isolated store)

**Approach:**
- Add `HasContext` impl for `(Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore)`:
  ```text
  impl HasContext for (Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore) {
      fn db(&self) -> &DbConn { &self.0 }
      fn codex(&self) -> &Codex { &self.1 }
      fn sortie_store(&self) -> &dyn SortieRepository { &self.2 }
      fn practice_store(&self) -> &dyn PracticeRepository { &self.3 }
  }
  ```
- Add doc comment to `GLOBAL_SORTIE_STORE`: "Process-global fallback. Prefer instance-scoped stores via SortieStore or TestSortieStore for test isolation."
- Add `new_context_with_isolation()` test helper that creates a `(Arc<DbConn>, Arc<Codex>, TestSortieStore, TestPracticeStore)` tuple
- Update `tests/gameplay_tests/map/unlock.rs` and `tests/gameplay_tests/useitem_material_sync.rs` to use the isolated helper

**Patterns to follow:**
- Existing tuple `HasContext` impls in `gameplay.rs` (for `(Arc<DbConn>, Arc<Codex>)` and `(DbConn, Codex)`)
- `TestSortieStore` delegation pattern in `sortie_store.rs`

**Test scenarios:**
- Happy path: Two tests run in parallel with their own isolated stores → no cross-test state leakage
- Regression: Existing tests using `(Arc<DbConn>, Arc<Codex>)` context still work (backward compat)
- Test expectation: All gameplay tests pass with both old and new context creation

**Verification:**
- `cargo test -p emukc_gameplay` passes with `--test-threads=4` (parallel execution)
- No test failures caused by shared state contamination

---

- U6. **Optimize `locked_enemy_composition` cloning in sortie battle flow**

**Goal:** Reduce unnecessary cloning of `locked_enemy_composition` in `sortie_battle_impl` and `sortie_sp_midnight_battle`.

**Requirements:** R6

**Dependencies:** None (independent of other units)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie.rs` (two battle functions)

**Approach:**
- In `sortie_battle_impl` and `sortie_sp_midnight_battle`, replace:
  ```text
  let enemy_composition = active
      .locked_enemy_composition
      .clone()
      .or_else(|| select_random_enemy_composition(&enemy_fleet))
      .unwrap_or_else(|| fallback_enemy_composition(current_cell.cell_no));
  ```
  with:
  ```text
  let enemy_composition = active
      .locked_enemy_composition
      .take()
      .or_else(|| select_random_enemy_composition(&enemy_fleet))
      .unwrap_or_else(|| fallback_enemy_composition(current_cell.cell_no));
  ```
- After the battle resolution code that uses `enemy_composition`, restore it:
  ```text
  let store = self.sortie_store();
  if let Some(mut active) = store.get_active(profile_id) {
      if active.locked_enemy_composition.is_none() {
          active.locked_enemy_composition = Some(enemy_composition);
          store.insert_active(profile_id, active);
      }
  }
  ```
- Evaluate whether the `sortie_battle_impl` code path can also use this pattern (it reads `active` early and may need the composition later)

**Execution note:** Take care with the `.take()` pattern — verify that `enemy_composition` is not needed again after the initial read in each code path. If it is needed again, keep `.clone()` but document why.

**Patterns to follow:**
- Existing `.take()` usage in the codebase (grep for `.take()` in sortie store methods)

**Test scenarios:**
- Happy path: Sortie battle uses locked enemy composition without error; composition remains set after battle
- Edge case: Sortie with no locked composition falls back to random selection, then restores `None`
- Edge case: Night battle (sp_midnight) re-accesses enemy composition correctly after day battle restored it
- Test expectation: `cargo test -p emukc_gameplay sortie_battle` passes

**Verification:**
- `cargo build` with `--release` — no performance regression introduced
- Existing sortie battle tests pass with no behavioral change

---

- U7. **Document `SlotItemImproved` as deferred hook-stub**

**Goal:** Add documentation clarifying that `SlotItemImproved` is intentionally unimplemented.

**Requirements:** R8

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_model/src/thirdparty/quest/matcher.rs` (add doc comment near `SlotItemImproved` variant)
- Modify: `crates/emukc_gameplay/src/game/slot_item.rs` (add TODO comment at improvement-adjacent code if applicable)

**Approach:**
- Add a doc comment to the `QuestActionEvent::SlotItemImproved` variant: `/// Hook-stub for slot-item improvement system (not yet implemented). When slot-item improvement gameplay is added, dispatch this event from the completion handler.`
- No code changes — documentation only

**Test expectation:** None — documentation change only

**Verification:**
- `cargo doc` builds without warnings for the documented variant

---

## System-Wide Impact

- **Interaction graph:** `HasContext` gains `practice_store()` — every type that implements `HasContext` (tuple contexts, `State`, test contexts) must provide an impl. The default impl falls back to `GLOBAL_PRACTICE_STORE` so existing types are not broken.
- **Error propagation:** Practice store methods use `parking_lot::Mutex` (no `Result` from lock acquisition). No new error paths introduced.
- **State lifecycle risks:** Practice pending state is now tied to the store's lifetime. If `State` is dropped while practice battles are in-flight, those states are lost (same as before with globals, but now the scoping is explicit). No new risks.
- **API surface parity:** `SortieOps::start_sortie` signature changes — all callers must be updated. This is an internal trait with a single blanket impl and ~5 call sites (HTTP handler + tests).
- **Integration coverage:** Practice battle + sink quest interaction is the key cross-layer scenario requiring an integration test (covered in U4 test scenarios).
- **Unchanged invariants:** Battle damage calculation, display_damage logic, quest progress calculation, and fleet composition validation are untouched by this plan.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Practice store migration introduces a regression in the exercise battle lifecycle | U3 includes end-to-end test coverage of day→night→result cycle; U4 adds sink quest interaction tests |
| `formation_id` removal breaks clients still sending the field | HTTP handler uses `serde_urlencoded` deserialization with `#[serde(default)]` — unknown fields are silently ignored by default |
| `.take()` optimization in U6 introduces a bug where enemy composition is not restored after battle | Keep the `.clone()` path if `.take()` restoration proves fragile; the optimization is P3 |
| `EnemyShipSunk` in practice creates quest progress that shouldn't count (game design question) | The condition matcher already supports sink events — if practice sinks should NOT count, the fix is in the matcher, not the dispatch. Adding the dispatch is the conservative choice (mirrors sortie) |

---

## Documentation / Operational Notes

- `GLOBAL_SORTIE_STORE` and new `GLOBAL_PRACTICE_STORE` should be documented as process-global fallbacks, not the recommended pattern for new code
- The `practice_repository.rs` module should include a doc comment explaining the trait's role and how it mirrors `SortieRepository`

---

## Sources & References

- Origin: code review of `refactor/map` branch (session context)
- Reference pattern: `crates/emukc_gameplay/src/game/battle/repository.rs` (SortieRepository trait)
- Reference pattern: `crates/emukc_gameplay/src/game/sortie_store.rs` (SortieStore + TestSortieStore)
- Reference pattern: `crates/emukc_gameplay/src/game/practice.rs` (current practice battle impl)
- Related plan: `docs/plans/2026-05-05-004-fix-map-refactor-audit-and-sortie-state-plan.md`
- Related audit: `docs/audit.md`
