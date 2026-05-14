---
title: "fix: Fix route jumping, stale stage-id, and premature sortie termination"
type: fix
status: completed
date: 2026-05-05
origin: docs/plans/2026-05-05-005-fix-map-system-audit-issues-plan.md
---

# Fix: Route Jumping, Stale Stage-ID, and Premature Sortie Termination

## Summary

Fix two remaining route-jumping gaps in `evaluate_route_destination` where client-provided `selected_cell_id` is accepted without validating against `next_cells` (topological neighbors). Fix stale `stage_id` and `boss_cell_id` in `ActiveSortieState` by deriving stage identity from the database `map_record` at the point of use rather than independently maintaining copies, eliminating dual-ownership that diverges after gauge-clear stage transitions.

---

## Problem Frame

Plan `2026-05-05-005` addressed four correctness bugs from the map-system audit but did not close two root-cause gaps discovered during follow-up testing. The prior plan added `cell_has_routing_outgoing` and fixed the `all_source_unknown` route-acceptance path, but two other paths in `evaluate_route_destination` still accept client-provided cell IDs without topology validation. Separately, `ActiveSortieState` holds independent copies of `stage_id` and `boss_cell_id` — fields that are also authoritatively tracked in the database `map_record`. When `apply_sortie_map_result` writes a new `stage_id` to the DB after a gauge clear, the in-memory store is never updated, causing subsequent requests (`next_sortie`, `sortie_battle`, `sortie_battle_result`) to operate on stale stage data. The downstream symptom is premature sortie termination — `should_finish_sortie` compares the current cell against a stale `boss_cell_id` from the pre-transition stage.

---

## Requirements

- R1. `evaluate_route_destination` must always validate both client-provided and server-chosen route targets against `current.next_cells` before accepting them.
- R2. `sortie_battle_result` must refresh `ActiveSortieState.stage_id` and `boss_cell_id` from the database `map_record` after the transaction commits, before deciding whether the sortie should finish.
- R3. `next_sortie` must refresh `stage_id` and `boss_cell_id` from the database `map_record` before loading the stage for route evaluation, providing defense-in-depth against any path that could leave the store stale.
- R4. All existing tests continue to pass; new regression tests cover each topology-validation gap and each stale-state scenario.
- R5. No external API changes — `SortieStartResponse`, `SortieNextResponse`, and `SortieBattleResultResponse` surface unchanged.
- R6. At codex-build time, wikiwiki routing rule targets that do not exist in kcdata topology must be dropped with a parse warning rather than silently entering the codex.
- R7. At codex-build time, enemy fleet entries for non-existent cells must be dropped with a parse warning.

---

## Scope Boundaries

- **In scope:** The two remaining `next_cells` validation gaps in `evaluate_route_destination`, stale `stage_id`/`boss_cell_id` refresh in `sortie_battle_result` and `next_sortie`.
- **Out of scope:** Changing `start_sortie` stage resolution (correct by construction), adding a sortie-state DB table, validating rule targets at catalog-build time, fixing pre-existing test failures in `first_gauge_clear_switches_map_variant_without_finishing_map` and `start_sortie_returns_post_p_unlock_layout_after_first_gauge_clear`.

### Deferred to Follow-Up Work

- Remove `stage_id` and `boss_cell_id` from `ActiveSortieState` entirely (always derive from DB — larger refactor in a separate PR). The `refresh_sortie_stage` helper introduced here is a tactical fix; the deferred item remains the intended end state to eliminate the dual-ownership pattern permanently.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_gameplay/src/game/map_route.rs` — `evaluate_route_destination` (lines 97-198): three paths accept `selected_cell_id`. The `all_source_unknown` path was fixed in plan 005; the indeterminate path (line 113) and executable path (line 147) are unfixed.
- `crates/emukc_gameplay/src/game/sortie.rs` — `ActiveSortieState` (line 73): holds `stage_id` and `boss_cell_id` set once in `start_sortie` (line 310-317) and never refreshed.
- `crates/emukc_gameplay/src/game/sortie.rs` — `sortie_battle_result` (line 518-630): commits DB transaction (which may have changed `stage_id` via `apply_sortie_map_result`), then checks `should_finish_sortie` using possibly-stale stage data.
- `crates/emukc_gameplay/src/game/sortie.rs` — `next_sortie` (line 362-452): reads `active.stage_id` from store without re-verifying against DB.
- `crates/emukc_gameplay/src/game/sortie.rs` — `sortie_battle_impl` (line 881): reads `active.stage_id` from store for stage lookup.
- `crates/emukc_gameplay/src/game/map_progress.rs` — `resolve_record_stage_id` (line 4): derives current stage from `map_record` DB row. Already used in `start_sortie`.
- `crates/emukc_gameplay/src/game/map.rs` — `find_map_record_impl` (line 205): simple DB lookup by `(profile_id, map_id)`.
- `crates/emukc_gameplay/src/game/sortie_result.rs` — `apply_sortie_map_result` (line 345): writes `stage_id` to DB via `assign_stage_id` when gauge clears and a `clear_to_variant_key` transition is triggered.

### Institutional Learnings

- Plan `2026-05-05-004` added `clear_sortie_state_if_any` to the port handler for stale-state cleanup on reconnect. The current plan addresses the complementary problem: stale state *during* an active sortie after gauge transitions.
- Plan `2026-05-05-003` established that `next_cells` is the sole source of graph topology. Runtime enforcement of this invariant prevents wikiwiki data inconsistencies from producing invalid routes.

---

## Key Technical Decisions

- **TD1. Add `refresh_sortie_stage` helper rather than inline the DB lookup at each call site.** A single helper function that reads `map_record`, resolves the current `stage_id`, compares to the store value, and updates the store when divergent keeps the refresh policy in one place and makes the intent explicit.

- **TD2. Refresh in `sortie_battle_result` AFTER the transaction commits.** The `stage_id` may change during `apply_sortie_map_result` which runs inside the DB transaction. The refresh must happen after `tx.commit().await` to see the updated record.

- **TD3. Also refresh in `next_sortie` for defense-in-depth.** Even though the `sortie_battle_result` refresh should keep the store correct, adding a refresh in `next_sortie` — the entry point for continued navigation — provides a safety net if the store ever becomes stale through another path (e.g., port handler cleanup race, future code changes).

- **TD4. Both remaining `selected_cell_id` paths must validate against `next_cells`.** The indeterminate path (line 113) and executable-rules path (line 147) both currently accept rule targets without topology verification. The fix adds `current.next_cells.contains(&selected_cell_id)` to each.

- **TD5. When stage changes and `current_cell_id` no longer exists in the new stage, finish the sortie.** If the cell the player was on does not exist in the refreshed stage, continuing is impossible — treat it as a sortie end. This is a safety guard rather than an expected path (the cell IDs should be stable across stage transitions).

---

## Implementation Units

- U1. **Add `next_cells` guard to the indeterminate path in `evaluate_route_destination`**

**Goal:** Prevent the client from selecting a cell that is not a topological neighbor when route evaluation is indeterminate.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (existing `#[cfg(test)]` module)

**Approach:**
- In `evaluate_route_destination`, at the indeterminate `selected_cell_id` acceptance block (approximately line 113-122), change:
  ```rust
  // Before: accepts any cell in candidate_targets (includes rule targets + next_cells)
  if any_indeterminate && candidate_targets.contains(&selected_cell_id) {
      return Ok(selected_cell_id);
  }
  ```
  to:
  ```rust
  // After: only accepts topological neighbors
  if any_indeterminate && current.next_cells.contains(&selected_cell_id) {
      return Ok(selected_cell_id);
  }
  ```
- Remove the `candidate_targets` variable from this block entirely — it is no longer used for the acceptance gate.

**Patterns to follow:**
- The `all_source_unknown` fix from plan 005 at line 104-106.
- Existing `current.next_cells.contains()` pattern at line 207 in `select_route_from_cells`.

**Test scenarios:**
- Error path: `any_indeterminate = true`, client sends `selected_cell_id` present in rule targets but NOT in `current.next_cells` → assertion: the function falls through to `select_route_from_cells(current, stage, selected_cell_id)` which rejects non-next_cells values.
- Happy path: `any_indeterminate = true`, client sends `selected_cell_id` present in BOTH rule targets and `next_cells` → assertion: returns the selected cell.
- Edge case: `selected_cell_id = None` (not provided) → existing `select_route_from_cells` behavior unchanged.

**Verification:**
- `cargo test -p emukc_gameplay --lib -- map_route` passes all 19 existing tests plus the 2 new tests.

---

- U2. **Filter executable rule targets by `next_cells` in `evaluate_route_destination`**

**Goal:** Prevent any route selection — both client-provided and server-chosen — from targeting a cell that is not a topological neighbor.

**Requirements:** R1

**Dependencies:** None (independent of U1)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (existing `#[cfg(test)]` module)

**Approach:**
- After computing `candidate_targets` from executable rules (approximately line 146), filter the set to only include targets also present in `current.next_cells`. If the filtered set is empty, return an error — topology does not support any rule target.
  ```rust
  let candidate_targets: BTreeSet<i64> = executable.iter()
      .map(|rule| rule.to_cell_no)
      .filter(|cell_no| current.next_cells.contains(cell_no))
      .collect();
  if candidate_targets.is_empty() {
      return Err(GameplayError::WrongType(format!(
          "cell {} has no executable route",
          current.cell_no,
      )));
  }
  ```
- The `selected_cell_id` check at line 147 naturally validates against the already-filtered `candidate_targets` — no separate `next_cells` guard needed.
- The single-target shortcut at line 156 and the weighted random selection at line 162 both operate on the filtered `candidate_targets`.

**Patterns to follow:**
- The `select_route_from_cells` topology check at line 207.

**Test scenarios:**
- Error path: executable rule targets cell 10, but `current.next_cells` is `[7, 8]` → `candidate_targets` becomes empty → returns `Err("no executable route")`.
- Happy path: executable rule targets cells 7 and 10, `current.next_cells` is `[7, 8]` → `candidate_targets` is `[7]`, client sends `selected_cell_id = 7` → returns `Ok(7)`.
- Happy path: `selected_cell_id = None`, one filtered target → returns the single target.
- Happy path: `selected_cell_id = None`, multiple filtered targets → weighted random from filtered set.
- Edge case: `selected_cell_id = 10` where 10 is a rule target but filtered out → `candidate_targets` doesn't contain 10 → returns `Err("not a valid route")`.

**Verification:**
- `cargo test -p emukc_gameplay --lib -- map_route` passes.

---

- U3. **Add `refresh_sortie_stage` helper and call in `sortie_battle_result`**

**Goal:** After the DB transaction commits (which may have changed `stage_id` via gauge clear), refresh the in-memory `ActiveSortieState` before deciding whether the sortie should finish.

**Requirements:** R2, R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie.rs`
- Test: `crates/emukc_gameplay/src/game/sortie.rs` (existing `#[cfg(test)]` module)

**Approach:**

Add a private helper function:
```rust
/// Refresh ActiveSortieState stage_id and boss_cell_id from the database map_record.
/// Called after gauge-clear transitions may have changed the active stage.
/// If the current cell no longer exists in the refreshed stage, returns None
/// (caller should finish the sortie).
 async fn refresh_sortie_stage(
     db: &DatabaseConnection,
     codex: &Codex,
     profile_id: i64,
     active: &mut ActiveSortieState,
 ) -> Result<bool, GameplayError> {
     let catalog = active_map_catalog(codex);
     let definition = catalog.as_ref()
         .map_definition(active.map_id)
         .ok_or_else(|| GameplayError::EntryNotFound(...))?;
     let record = find_map_record_impl(db, profile_id, active.map_id).await?;
    let new_stage_id = resolve_record_stage_id(&definition, &record)
        .unwrap_or_default();

    if new_stage_id != active.stage_id {
        let new_stage = definition.stage(&new_stage_id).ok_or_else(|| ...)?;
        // If the current cell doesn't exist in the new stage, sortie must end
        if !new_stage.cell(active.current_cell_id).is_some() {
            return Ok(false); // signal sortie end
        }
        active.stage_id = new_stage_id;
        active.boss_cell_id = new_stage.boss_cell_no;
    }
    Ok(true) // stage is current, sortie can continue
}
```

Then in `sortie_battle_result`, after `tx.commit().await?` (approximately line 607) and before the `should_finish_sortie` block (approximately line 613), insert the refresh. Crucially, this must happen **between the commit and the should_finish check** — not folded into the line 618 store-update block:

```rust
tx.commit().await?;

// Refresh stage identity from DB before deciding sortie fate.
// apply_sortie_map_result may have changed stage_id via gauge clear.
let stage_refreshed = refresh_sortie_stage(db, codex, profile_id, &mut active).await?;
if !stage_refreshed {
    // current_cell_id does not exist in the refreshed stage — end the sortie
    store.remove_active(profile_id);
    // Build a minimal finish response (snapshot, session, first_clear, ship_drop,
    // next_map_ids are all still in scope from before the commit)
    return Ok(SortieBattleResultResponse { /* normal finish fields */ });
}
// Re-resolve stage AND current_cell from the refreshed data to avoid
// comparing old-stage cell data against new-stage boss_cell_id.
let stage = definition.stage(&active.stage_id).ok_or_else(...)?;
let current_cell = stage.cell(pending_cell_id).ok_or_else(...)?;

// Persist the refreshed store state so subsequent operations see it
store.insert_active(profile_id, active.clone());

// Now should_finish_sortie uses fresh stage_id, boss_cell_id, and current_cell
let should_finish_sortie = current_cell.cell_no == active.boss_cell_id
    || !cell_has_routing_outgoing(current_cell.cell_no, &stage);
```

**Note:** `sortie_battle_result` takes `active` by value (via `store.get_active` at line 537). Change `let active` to `let mut active` and pass `&mut active` to `refresh_sortie_stage`, then `store.insert_active(profile_id, active)` after mutation. No clone needed — the value is already owned. This matches the mutation pattern at line 618 where `pending_battle_cell_id` is cleared.

**Patterns to follow:**
- `resolve_record_stage_id` usage in `start_sortie` (line 291).
- `find_map_record_impl` usage throughout sortie.rs.
- Store insert pattern at line 414 and 620.

**Test scenarios:**
- Happy path: gauge clears on boss cell → `refresh_sortie_stage` updates `stage_id` to the next stage → `should_finish_sortie` sees the updated `boss_cell_id` → sortie ends (boss reached in refreshed stage).
- Integration: gauge clears on non-boss cell (edge case) → stage refreshes → `should_finish_sortie` uses refreshed stage's `boss_cell_id` → sortie continues if the cell has outgoing routes.
- Edge case: `current_cell_id` does not exist in the refreshed stage → `refresh_sortie_stage` returns false → sortie is terminated gracefully.

**Verification:**
- `cargo test -p emukc_gameplay --lib -- game::sortie` passes.
- New test simulates gauge-clear-induced stage switch and verifies store refresh.

---

- U4. **Call `refresh_sortie_stage` in `next_sortie` for defense-in-depth**

**Goal:** Guarantee that `next_sortie` always operates on the current stage, even if `sortie_battle_result`'s refresh was missed or the store became stale through another path.

**Requirements:** R3

**Dependencies:** U3 (needs the helper function)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie.rs`
- Test: `crates/emukc_gameplay/src/game/sortie.rs` (existing `#[cfg(test)]` module)

**Approach:**
- In `next_sortie`, after obtaining `active` from the store (approximately line 362) and before the `definition.stage(&active.stage_id)` lookup (approximately line 377), insert a call to `refresh_sortie_stage`. The DB connection is already available (`let db = self.db()`).
- If `refresh_sortie_stage` returns `false` (cell does not exist in refreshed stage), return an error — the sortie cannot continue.
- After refreshing, re-insert the updated active into the store so subsequent operations see the current stage.
- The existing code already re-reads `store.get_active` before modifying state later in the function, so the re-insert is safe.

**Patterns to follow:**
- Same store-read-then-insert pattern used at line 396-414 for inserting after route evaluation.

**Test scenarios:**
- Happy path: store has stale `stage_id` from previous gauge clear → `next_sortie` refreshes from DB → stage lookup succeeds with the correct stage.
- Edge case: store has correct `stage_id` → refresh is a no-op (no DB write, no store update) — fast path.
- Error path: store has stale `stage_id` and the current cell doesn't exist in the refreshed stage → `refresh_sortie_stage` returns false → sortie ends with an appropriate error.

**Verification:**
- Existing `next_sortie` tests continue to pass.
- New test verifies that when the DB record's `stage_id` differs from the store, `next_sortie` uses the DB value.

---

- U5. **Update existing tests for all changed modules**

**Goal:** All existing test call sites in `sortie.rs` and `map_route.rs` remain consistent with the changed function signatures and module structure.

**Requirements:** R4

**Dependencies:** U1, U2, U3, U4 (these change both modules, requiring test updates)

**Files:**
- Test: `crates/emukc_gameplay/src/game/sortie.rs` (existing `#[cfg(test)]` module)
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (existing `#[cfg(test)]` module)

**Approach:**
- All test call sites to `route_predicate_matches` in the `sortie.rs` test module already pass `&empty_stage()` — this was done in plan 005. Verify no additional call sites need updating after the U3/U4 changes take effect.
- If the `refresh_sortie_stage` helper introduces a new async setup requirement, add a test helper to create the necessary DB + codex context.

**Test scenarios:**
- Regression: all existing sortie tests pass after the stage-refresh changes.
- Regression: the `empty_stage()` pattern in test calls to `route_predicate_matches` still compiles and passes.

**Verification:**
- `cargo test -p emukc_gameplay --lib` passes (excluding the 2 pre-existing failures).
- `cargo test -p emukc_model --lib -- codex::map::merge` passes.

---

- U6. **Validate routing rule targets against kcdata topology at build time**

**Goal:** Prevent wikiwiki routing rules from entering the codex when their target cell_nos are not present in kcdata's topology (silent data corruption at ingest time).

**Requirements:** None (data quality — prevents future runtime surprises)

**Dependencies:** None (independent of all other units)

**Files:**
- Modify: `crates/emukc_model/src/codex/map/merge.rs`
- Test: `crates/emukc_model/src/codex/map/merge.rs` (existing `#[cfg(test)]` module)

**Approach:**
- In `merge_routing_overlay`, after remapping each rule's `from_cell_no` and `to_cell_no` through `cell_no_map`, validate that the remapped `to_cell_no` corresponds to a cell that exists in `definition.cells`. If not, skip the rule and append a parse_warning.
- Also validate `from_cell_no` exists (though this is the key in `routing_rules` and inherently valid if we're iterating).
- Record skipped rules as `parse_warnings` entries: `"routing_rule target cell {to_cell_no} not in topology — dropped"`.
- Apply the same validation in the `remap_variant_to_definition_identity` codepath (used by STAT overlay and public overlay merges via `merge_variant_definition`).

**Patterns to follow:**
- Existing parse_warnings pattern in `merge_variant_definition` (lines 93-108).
- `definition.cells` iteration for existence check.

**Test scenarios:**
- Happy path: kcdata variant has cells [1, 2, 3], wikiwiki rule targets cell 2 → rule inserted.
- Error path: kcdata variant has cells [1, 2, 3], wikiwiki rule targets cell 5 (not in topology) → rule skipped, parse_warning appended.
- Edge case: all rules target non-existent cells → routing_rules remains empty, parse_warnings populated.
- Integration: STAT overlay and public overlay merges also validated (via remap_variant_to_definition_identity path).

**Verification:**
- `cargo test -p emukc_model --lib -- codex::map::merge` passes including the 3 new tests.

---

- U7. **Validate enemy fleet cell_nos against kcdata topology at build time**

**Goal:** Same as U6, for enemy fleet entries. Prevent enemy compositions from being attached to non-existent cells.

**Requirements:** None (data quality)

**Dependencies:** None (can be done in the same edit as U6)

**Files:**
- Modify: `crates/emukc_model/src/codex/map/merge.rs`
- Test: `crates/emukc_model/src/codex/map/merge.rs` (existing `#[cfg(test)]` module)

**Approach:**
- In `merge_routing_overlay`, after remapping enemy fleet `cell_no`, validate that the mapped cell exists in `definition.cells`. If not, skip the entry and append a parse_warning.
- Same for the `remap_variant_to_definition_identity` path.

**Test scenarios:**
- Happy path: mapped cell_no exists → enemy fleet inserted.
- Error path: mapped cell_no not in cells → enemy fleet skipped, warning appended.

**Verification:**
- `cargo test -p emukc_model --lib -- codex::map::merge` passes.

---

## System-Wide Impact

- **Interaction graph:** `evaluate_route_destination` is called from `start_sortie` (via cell 0), `next_sortie`, and test modules. The signature does not change — only internal validation tightens. `merge_routing_overlay` gains validation logic at codex-build time (never called at runtime). `refresh_sortie_stage` is a new private helper called from `sortie_battle_result` and `next_sortie`.
- **Error propagation:** Rejected `selected_cell_id` values now return `WrongType` errors with descriptive messages. These surface to the client as API errors — the client should not send non-topological cell IDs in normal play. Build-time validation produces `parse_warnings` in the codex; these are diagnostic-only and do not block catalog building.
- **State lifecycle risks:** The store is now mutated during `sortie_battle_result` (after DB commit) and during `next_sortie` (before route evaluation). Both follow the existing store mutation pattern (read, clone, mutate, re-insert). No new concurrency concerns — the store Mutex serializes access per-profile.
- **API surface parity:** No response type changes. `SortieStartResponse`, `SortieNextResponse`, and `SortieBattleResultResponse` are unchanged.
- **Integration coverage:** The gauge-clear → stage-switch → refresh chain should be tested as a sequence: start sortie, apply a boss-result snapshot that clears the gauge, verify store has updated stage_id.
- **Unchanged invariants:** `next_cells` remains the sole source of graph topology. Database `map_record` is the authoritative source of stage identity. `ActiveSortieState` fields other than `stage_id`/`boss_cell_id` are unchanged. Codex-build process still completes even if some wikiwiki rules are rejected — parse_warnings are informational.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `refresh_sortie_stage` requires `profile_id` in `sortie_battle_result` | The function has `profile_id` as a parameter — pass it through |
| The `active` variable in `sortie_battle_result` is moved from the store (owned), not borrowed | Clone `active`, mutate the clone, then re-insert via `store.insert_active` — consistent with the existing store pattern at line 618 |
| Stricter topology validation rejects legitimate rule targets that wikiwiki references but kcdata hasn't encoded as edges | This is the desired behavior — the plan architecture says next_cells is the authority. If this causes unexpected errors in production, the fix is at the data layer (adding missing edges to kcdata), not at the validation layer |
| `refresh_sortie_stage` adds a DB query to `next_sortie` | The DB is already queried for fleet ships in `next_sortie`. The `map_record` lookup is a single-row fetch by indexed composite key — negligible overhead |

---

## Sources & References

- **Origin plan:** `docs/plans/2026-05-05-005-fix-map-system-audit-issues-plan.md`
- **Prior sortie-state plan:** `docs/plans/2026-05-05-004-fix-map-refactor-audit-and-sortie-state-plan.md`
- **Topology refactor plan:** `docs/plans/2026-05-05-003-refactor-map-topology-routing-separation-plan.md`
- Route evaluation: `crates/emukc_gameplay/src/game/map_route.rs`
- Sortie lifecycle: `crates/emukc_gameplay/src/game/sortie.rs`
- Sortie store: `crates/emukc_gameplay/src/game/sortie_store.rs`
- Map progress: `crates/emukc_gameplay/src/game/map_progress.rs`
- Merge logic: `crates/emukc_model/src/codex/map/merge.rs`
