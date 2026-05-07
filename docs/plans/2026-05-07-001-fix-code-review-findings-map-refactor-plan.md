---
title: Fix code review findings in map system refactor
type: fix
status: active
date: 2026-05-07
origin: /tmp/compound-engineering/ce-code-review/20260507-141921-70060c97/ (ce-code-review run artifact)
---

# Fix code review findings in map system refactor

## Summary

Apply fixes for 12 code review findings (1 P0, 6 P1, 5 P2) plus a compilation error surfaced by learnings research, all on the `refactor/map` branch. Mechanical safe_auto fixes first, then lock-safety fixes, then test coverage improvements. Advisory findings documented but deferred.

---

## Problem Frame

A multi-agent code review of the `refactor/map` branch (12,014-line diff across 31 core map/sortie files) identified 12 findings surviving the confidence gate. The review also surfaced a compilation error in `monthly_reset.rs` (wrong import path for `emukc_time`). These must be fixed before the branch can merge. The findings cluster around three themes: sortie state lock safety, duplicate code across crate boundaries, and insufficient test coverage for new public APIs.

---

## Requirements

- R1. `start_sortie` and `sortie_battle_result` must serialize sortie state mutations through `with_profile_lock`, matching the pattern in `next_sortie` and `sortie_battle_impl`
- R2. `clear_pending_sortie_runtime_state` in `start_sortie` must execute after DB commit, not before — prevents irrecoverable state loss on transaction failure
- R3. `split_map_id` and `extract_max_hp` must have a single authoritative definition, not two identical copies in `emukc_model` and `emukc_bootstrap`
- R4. `verify.rs` module declaration is redundant — entire file body is already `#[cfg(test)]`, so the bare `mod verify;` in `mod.rs` compiles a dead module in production
- R5. `_manifest` parameter in `sources.rs` must not use underscore prefix (it is consumed downstream)
- R6. New public API functions in `map_overlay/merge.rs` and `map_progress.rs` must have unit tests
- R7. Integration test assertions must verify concrete post-operation state, not just `is_ok`/`is_err`
- R8. `monthly_reset.rs` must compile — import `emukc_internal::time::chrono` not `emukc_time::chrono`

---

## Scope Boundaries

- Fixes apply to the `refactor/map` branch only
- Only findings with confidence ≥ 75 (plus the P0 at confidence ≥ 50) are in scope
- Tab indentation in untracked `route/` files excluded — `cargo fmt` will normalize on first build
- Advisory findings (crash-recovery persistence, wikiwiki-only catalog fallback) documented but not implemented

### Deferred to Follow-Up Work

- Crash-recovery: persist minimal sortie runtime state to DB or log warning on restart — needs product decision on recovery semantics
- Wikiwiki-only catalog path: add warning when kcdata is absent — architectural decision on fallback behavior
- Dual-ownership refactor of `stage_id`/`boss_cell_id` in `ActiveSortieState` — deferred by prior plan, `refresh_sortie_stage` is current tactical band-aid

---

## Context & Research

### Relevant Code and Patterns

- `with_profile_lock` pattern in `crates/emukc_gameplay/src/game/sortie/mod.rs` — `next_sortie` and `sortie_battle_impl` use it; `start_sortie` and `sortie_battle_result` should match
- `split_map_id` at `crates/emukc_model/src/codex/map.rs:359` is the authoritative definition; `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs:113` is the duplicate
- Test patterns: `tests/gameplay_tests/map/` uses `new_mem_db()` for in-memory DB, `TestSortieStore` for isolated sortie state

### Institutional Learnings

- Route-jumping took 3 fix passes — every `selected_cell_id` path must validate against `next_cells`. This plan's lock fixes add another layer of defense.
- Merge function accumulation asymmetry (`or_insert` vs `or_default().extend`) silently dropped rules in past — verify merge semantics when touching shared utilities.
- Sortie state cleanup must happen at all entry points — the lock fixes in this plan ensure cleanup is serialized.

---

## Key Technical Decisions

- **Group duplicate function removal with premature clear fix in one unit (U2):** all are mechanical changes touching `sortie/mod.rs` and `kcdata.rs`/`map.rs` — atomic commit; the clear reordering changes error-path behavior intentionally (R2)
- **Group lock fixes in dedicated unit (U4):** P0 TOCTOU requires careful lock-scope design; keeping it isolated from mechanical fixes ensures clean review
- **Model crate owns shared utilities:** `split_map_id` and `extract_max_hp` stay in `emukc_model`; bootstrap imports from model
- **`with_profile_lock` scope in `start_sortie`:** wrap from `clear_pending_sortie_runtime_state` through `insert_active` — minimal critical section
- **`with_profile_lock` scope in `sortie_battle_result`:** wrap the active state read-modify-write segment only — DB operations before and after remain outside the lock

---

## Implementation Units

### U1. Fix compilation error in monthly_reset.rs

**Goal:** Fix broken import so the test file compiles

**Requirements:** R8

**Dependencies:** None

**Files:**
- Modify: `tests/gameplay_tests/map/monthly_reset.rs`

**Approach:**
- Replace `use emukc_time::chrono::{Duration, Utc}` with `use emukc_internal::time::chrono::{Duration, Utc}` at line 82
- `emukc_time` is not a direct dependency of the root crate; `emukc_internal::time` is the correct re-export path

**Patterns to follow:**
- Other test files in `tests/gameplay_tests/` use `emukc_internal::prelude::*` for imports

**Test scenarios:**
- Test expectation: none — this unit fixes a compilation error; `cargo test --test gameplay_tests` passing confirms the fix

**Verification:**
- `cargo test --test gameplay_tests map::monthly_reset` compiles and passes

---

### U2. Fix premature clear ordering and deduplicate shared utilities

**Goal:** Move `clear_pending_sortie_runtime_state` after DB commit; make `split_map_id` and `extract_max_hp` single-source in model crate

**Requirements:** R2, R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie/mod.rs` (premature clear ordering)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (remove duplicate utils, import from model)
- Modify: `crates/emukc_model/src/codex/map.rs` (confirm `extract_max_hp` is already `pub` — no change needed)

**Approach:**
1. In `start_sortie`: move `clear_pending_sortie_runtime_state` call from before `tx.commit().await?` to after it succeeds, and before `insert_active`
2. In `kcdata.rs`: delete the `split_map_id` and `extract_max_hp` function definitions (lines ~113-130)
3. Add `use emukc_model::codex::map::{split_map_id, extract_max_hp}` import to `kcdata.rs`
4. `extract_max_hp` is already `pub` in model crate — no visibility change needed

**Patterns to follow:**
- `next_sortie` in `sortie/mod.rs` already calls `clear_pending_sortie_runtime_state` — match that sequencing

**Test scenarios:**
- Happy path: `start_sortie` followed by `sortie_battle_result` — sortie state transitions correctly
- Error path: simulate DB transaction failure in `start_sortie` — verify pending state is NOT cleared (still recoverable)
- Happy path: `split_map_id("1-5")` called from bootstrap returns same result as model's `split_map_id("1-5")`

**Verification:**
- Existing sortie tests pass (`cargo test --test gameplay_tests map::`)
- `cargo build` succeeds for both `emukc_bootstrap` and `emukc_model` crates

---

### U3. Fix verify.rs dead code and _manifest naming

**Goal:** Remove empty production module; rename misleading parameter

**Requirements:** R4, R5

**Dependencies:** None (independent of U1/U2)

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/mod.rs` (remove or gate `mod verify` declaration)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/sources.rs` (rename `_manifest` → `manifest`)

**Approach:**
1. In `mod.rs`: gate the `mod verify;` declaration with `#[cfg(test)]` — file content is already `#[cfg(test)]` but the module declaration itself is unconditional, creating a dead module in production
2. In `sources.rs` line 39: rename `_manifest: &ApiManifest` to `manifest: &ApiManifest`
3. Update all references to `_manifest` in the function body to `manifest`

**Patterns to follow:**
- Standard Rust convention: underscore prefix signals intentionally unused; a used parameter must not carry it

**Test scenarios:**
- Test expectation: none — both changes are mechanical; `cargo build` and `cargo test` confirm correctness

**Verification:**
- `cargo build -p emukc_bootstrap` succeeds
- `cargo test -p emukc_bootstrap` passes (verify.rs tests still run)
- `cargo clippy -p emukc_bootstrap` produces no new warnings

---

### U4. Add lock serialization to start_sortie and sortie_battle_result

**Goal:** Prevent TOCTOU races on sortie state by wrapping critical sections in `with_profile_lock`

**Requirements:** R1

**Dependencies:** U2 (premature clear ordering in same function)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie/mod.rs`

**Approach:**
1. In `start_sortie`: wrap `clear_pending_sortie_runtime_state` + `insert_active` in `with_profile_lock`. The lock scope is from just before the clear call through just after `insert_active`. DB operations (`tx.commit()`) happen before the lock — the clear must happen after commit succeeds and inside the lock.
2. In `sortie_battle_result`: wrap the active state read-modify-write segment in `with_profile_lock`. Extract the segment that reads `active` from the store, mutates it, and writes back. DB operations (map record updates, etc.) remain outside the lock to keep the critical section minimal.
3. Match the lock acquisition pattern from `next_sortie` and `sortie_battle_impl` which already use `with_profile_lock`.

**Technical design (directional):**
```
// start_sortie — critical section after DB commit (closure-based API, takes Future)
tx.commit().await?;
self.sortie_store().with_profile_lock(profile_id, async {
    self.sortie_store().clear_pending_sortie_runtime_state(profile_id);
    self.sortie_store().insert_active(profile_id, active);
}).await;

// sortie_battle_result — critical section around state mutation (closure-based API, takes Future)
self.sortie_store().with_profile_lock(profile_id, async {
    let mut active = self.sortie_store().get_active(profile_id)...;
    // ... mutate active ...
    self.sortie_store().insert_active(profile_id, active);
}).await;
// ... DB operations follow ...
```

**Patterns to follow:**
- `next_sortie` lock pattern: `let _lock = self.sortie_store().with_profile_lock(profile_id).await;` at the top
- `sortie_battle_impl` lock pattern: same approach
- `tokio::sync::Mutex` allows nesting — safe to hold profile lock across DB operations if needed

**Test scenarios:**
- Happy path: single `start_sortie` → `next_sortie` → `sortie_battle_result` → `goback_port` sequence works
- Edge case: `start_sortie` when pending sortie state exists from prior incomplete sortie — pending state cleared under lock, new sortie inserted atomically
- Error path: `sortie_battle_result` DB commit fails — verify in-memory sortie state unchanged (lock serialized the read but commit failure leaves state as-is)

**Verification:**
- All existing sortie tests pass (`cargo test --test gameplay_tests map::`)
- `cargo test -p emukc_gameplay sortie_battle` passes
- Manual review: confirm lock scope is minimal and no `.await` points exist between sortie state read and write

---

### U5. Add unit tests for map_overlay/merge.rs and map_progress.rs

**Goal:** Test the new public API functions that currently have zero coverage

**Requirements:** R6

**Dependencies:** None (independent of U1-U4)

**Files:**
- Create: `crates/emukc_bootstrap/src/map_overlay/merge_tests.rs` (or add `#[cfg(test)] mod tests` to `merge.rs`)
- Create: `crates/emukc_gameplay/src/game/map_progress_tests.rs` (or add `#[cfg(test)] mod tests` to `map_progress.rs`)

**Approach:**
1. `merge.rs` tests:
   - `build_public_map_catalog_overlay_from_captures` with empty captures → returns empty overlay
   - Single capture with valid map_id → overlay contains the captured map
   - Capture referencing non-existent map_id → produces rejection record
   - Two captures covering different stages → both stages accumulated
   - Conflicting `master_cell_id` for same cell → produces merge rejection
2. `map_progress.rs` tests:
   - `resolve_record_stage_id` with valid stage → returns correct stage_id
   - `resolve_record_stage_id` with invalid/missing stage → returns fallback
   - `active_stage_for_record` with valid record → returns correct variant
   - `select_stage_id_for_rank` with valid rank mapping → returns mapped stage
   - `select_stage_id_for_rank` with missing rank → returns default

**Patterns to follow:**
- Existing `#[cfg(test)] mod tests` pattern in `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs`
- Use `new_mem_db()` for gameplay tests (import from `emukc_db::mem`)

**Test scenarios:**
- Unit: `build_public_map_catalog_overlay_from_captures` — empty input, single capture, multi-capture, unknown map_id
- Unit: capture with Err result → rejection record
- Unit: `resolve_record_stage_id` — valid stage_id, invalid stage_id fallback, missing stage_id fallback
- Unit: `active_stage_for_record` — valid record, edge case with cleared=true
- Unit: `select_stage_id_for_rank` — rank 1 maps to stage, rank with no mapping falls back to default

**Verification:**
- `cargo test -p emukc_bootstrap map_overlay::merge_tests` passes
- `cargo test -p emukc_gameplay map_progress` passes (new tests run alongside existing)

---

### U6. Strengthen gameplay integration test assertions

**Goal:** Replace `is_ok`/`is_err` assertions with concrete state verification

**Requirements:** R7

**Dependencies:** U4 (lock fixes should land first)

**Files:**
- Modify: `tests/gameplay_tests/map/retreat.rs`
- Modify: `tests/gameplay_tests/map/non_boss_pending.rs`

**Approach:**
1. `retreat.rs`: after retreat via `goback_port`, verify:
   - Ship HP unchanged from pre-sortie values
   - Ship fuel/ammo unchanged
   - `defeat_count` in map_record not incremented
   - Sortie store `get_active` returns None (state cleaned up)
   - `pending_battle` and `pending_result` cleared
2. `non_boss_pending.rs`: after non-boss node battle + `goback_port`, directly verify:
   - `pending_battle` is None (cleared by cleanup)
   - `pending_result` is None
   - Instead of relying on second `goback_port` call failure as indirect proof

**Patterns to follow:**
- `tests/gameplay_tests/map/multi_gauge.rs` assertions that verify concrete gauge index and HP values
- Use `codex().map_catalog()` to look up expected values
- Use `gameplay().get_map_records(profile_id)` to read DB state

**Test scenarios:**
- Integration (retreat): start sortie on 1-1, advance to node, call `goback_port` → verify ship state unchanged, defeat_count=0, active sortie cleared
- Integration (non-boss pending): start sortie on 1-1, advance to non-boss node, call `goback_port` → verify pending_battle=None, pending_result=None
- Edge case: retreat without advancing (immediate `goback_port` after `start_sortie`) → sortie state cleared

**Verification:**
- `cargo test --test gameplay_tests map::retreat` passes with concrete assertions
- `cargo test --test gameplay_tests map::non_boss_pending` passes with direct state verification

---

## System-Wide Impact

- **Interaction graph:** Lock changes in U4 affect `start_sortie` → `next_sortie` → `sortie_battle_result` call chain. All callers of these functions (HTTP handlers in `src/bin/net/router/kcsapi/api_req_map/` and `api_req_sortie/`) are indirect beneficiaries — no handler changes needed.
- **Error propagation:** U2's clear-after-commit ordering ensures transaction failure leaves in-memory state intact. Previously, a failed commit would have already destroyed the pending state.
- **State lifecycle risks:** U4 locks prevent concurrent `start_sortie` and `next_sortie` on the same profile from interleaving. Without the lock, two simultaneous `start_sortie` calls could both clear the pending state, then both insert active state — second insert silently overwrites first.
- **Unchanged invariants:** Sortie state schema (`ActiveSortieState` fields) unchanged. `SortieStore` public API unchanged. Lock acquisition is internal to `start_sortie` and `sortie_battle_result` — callers see no API change.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Lock scope in `sortie_battle_result` is too broad, holding profile lock across DB operations | Keep critical section minimal — lock only the in-memory read-modify-write; release before DB calls |
| `extract_max_hp` visibility change breaks other crate consumers | Already `pub` — no change needed, risk eliminated |
| Strengthened test assertions expose pre-existing bugs in retreat flow | If tests fail for reasons unrelated to this diff, file separate issue and scope assertions to only verify the post-refactor invariants |

---

## Sources & References

- **Code review run artifact:** `/tmp/compound-engineering/ce-code-review/20260507-141921-70060c97/`
- Related plan: `docs/plans/2026-05-05-003-refactor-map-topology-routing-separation-plan.md`
- Related plan: `docs/plans/2026-05-05-006-fix-route-jumping-and-stale-stage-state-plan.md`
