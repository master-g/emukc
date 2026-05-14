---
title: "fix: Complete map topology refactor audit items and clear stale sortie state on port return"
type: fix
status: completed
date: 2026-05-05
---

# Fix: Complete Map Topology Refactor Audit Items and Clear Stale Sortie State on Port Return

## Summary

Three items: (1) complete U3 from the map topology refactoring plan — simplify WikiWiki parser to output a flat label map instead of full `MapCellDefinition` structs; (2) complete U4 — clean up dead code and comments from the refactor; (3) add stale sortie state cleanup in the port handler so reconnecting clients don't carry ghost sortie state into their next session.

---

## Problem Frame

The map topology refactoring plan (`2026-05-05-003`) has U1 and U2 complete, but U3 (WikiWiki parser simplification) and U4 (cleanup) remain incomplete. The parser still outputs full `Vec<MapCellDefinition>` with `next_cells` even though assemble.rs no longer uses them for topology. Functionally the goals are met, but the code is inconsistent — the parser builds structures that are only partially consumed.

Separately, the `api_port/port` handler does not clear stale `ActiveSortieState` from `SortieStore`. If a client disconnects mid-sortie and reconnects (hitting `api_port/port`), the runtime still holds the old sortie. While `start_sortie` does call `clear_pending_sortie_runtime_state` before inserting a new active sortie, stale `pending_battles` or `pending_results` from the previous session could interfere. A defensive cleanup in the port handler eliminates this class of problem.

---

## Requirements

- R1. WikiWiki parser's `build_nodes()` keeps `Vec<WikiwikiNodeDefinition>` return type but no longer populates `next_cells` (label and cell_no are retained; edge data is empty)
- R2. WikiWiki parser no longer constructs `next_cells` in `WikiwikiNodeDefinition`; `into_map_catalog()` produces `MapCellDefinition` with empty `next_cells`
- R3. `into_map_catalog()` uses the new label map; cells in WikiWiki's `MapVariantDefinition` are empty (or a minimal stub for label extraction compatibility)
- R4. Dead code from BFS heuristic removal is cleaned up; comments updated
- R5. `api_port/port` handler clears any stale sortie state (active sortie + pending results/battles) before building the response
- R6. All existing tests continue to pass; new tests cover the port cleanup path

---

## Scope Boundaries

- **Out of scope:** Changes to runtime route evaluation (`map_route.rs`)
- **Out of scope:** WikiWiki-only fallback — kcdata is required for topology
- **Out of scope:** Changes to the `SortieStore` data structure (new method exposure on existing struct is in scope)
- **Out of scope:** Database schema changes — sortie state is purely in-memory

### Deferred to Follow-Up Work

- Validate kcdata topology against real_map_start_data vertex counts
- Add graph validation test suite for all maps (not just 1-1/1-2/1-3)
- Evaluate `merge_variant_definition`'s `inferred_start` fallback logic for dead code

---

## Context & Research

### Relevant Code and Patterns

- **WikiWiki parser `build_nodes()`**: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs:340-428` — currently returns `Vec<WikiwikiNodeDefinition>` with `next_cells`; BFS heuristic already removed but output type unchanged
- **WikiWiki `into_map_catalog()`**: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs:103-270` — builds `Vec<MapCellDefinition>` from parser output, including `next_cells`
- **`WikiwikiNodeDefinition` struct**: defined in `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` — carries `label`, `cell_no`, `is_boss`, `is_battle`, `next_cells`
- **`SortieStore`**: `crates/emukc_gameplay/src/game/sortie_store.rs` — holds `active_sorties`, `pending_results`, `pending_battles` as `Mutex<HashMap<i64, _>>`
- **Port handler**: `src/bin/net/router/kcsapi/api_port/port.rs` — builds port response, no sortie cleanup
- **Goback port handler**: `src/bin/net/router/kcsapi/api_req_sortie/goback_port.rs` — clears active sortie + pending state via `clear_pending_sortie_runtime_state`
- **`clear_pending_sortie_runtime_state`**: `crates/emukc_gameplay/src/game/sortie.rs:1550-1553` — removes pending result and day battle result
- **`SortieOps` trait**: `crates/emukc_gameplay/src/game/sortie.rs:191` — includes `sortie_goback_port` method
- **`GameOps` trait**: `crates/emukc_gameplay/src/game/mod.rs:69` — composes `SortieOps` among others

---

## Key Technical Decisions

- **TD1. Keep `WikiwikiNodeDefinition` as intermediate struct, extract label map in `into_map_catalog()`:** Rather than changing `build_nodes()` return type (which would ripple through many call sites), keep the struct but stop populating `next_cells` in `build_nodes()`. The graph edges are already not inferred (heuristic removed). The label map extraction already happens in `into_map_catalog()` and `assemble.rs`. This minimizes diff while achieving the same structural simplification.

- **TD2. Port handler calls `sortie_goback_port` or equivalent cleanup silently:** The port handler should clear stale sortie state without returning an error if no active sortie exists. The existing `sortie_goback_port` returns an error when no active sortie is found, so a lighter-weight approach is needed: call `remove_active_sortie` + `clear_pending_sortie_runtime_state` directly, or add a new method that silently no-ops when no active sortie exists. Prefer adding a `clear_sortie_state_if_any` method to `SortieOps` for clarity.

- **TD3. WikiWiki cells field becomes empty in output `MapVariantDefinition`:** Since `assemble.rs` already extracts labels from `wikiwiki_variant.cells` (lines 59-68), the cells must still carry `node_label` and `cell_no` for label extraction. But `next_cells` should be empty. This is the minimal change — cells exist as label carriers only, no edge data.

---

## Open Questions

### Resolved During Planning

- **Port cleanup approach:** A new `clear_sortie_state_if_any` method is cleaner than calling `sortie_goback_port` (which errors on no active sortie). The method removes active sortie + pending results/battles silently.
- **WikiWiki cells in output:** Must keep cells as label carriers (assemble.rs reads `wikiwiki_variant.cells` for label extraction). Cannot fully remove cells, but can empty `next_cells`.

### Deferred to Implementation

- **Whether `WikiwikiNodeDefinition.next_cells` field can be removed entirely:** The field may still be used in `into_map_catalog()` for start target inference. Need to check during implementation.

---

## Implementation Units

- U1. **Simplify WikiWiki parser: stop constructing next_cells, extract label map**

**Goal:** Remove next_cells construction from `build_nodes()`. Ensure WikiWiki output has no edge inference.

**Requirements:** R1, R2, R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` — `build_nodes()` function
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` — `into_map_catalog()`, `WikiwikiNodeDefinition` struct
- Test: `crates/emukc_bootstrap/src/parser/wikiwiki_map/` (inline tests)

**Approach:**
- In `build_nodes()`: the BFS heuristic is already removed. Now also stop populating `next_cells` from the `graph` map. The `graph` map is still built from route_rules for BFS ordering, but `next_cells` in `WikiwikiNodeDefinition` should always be empty.
- In `into_map_catalog()`: the `node.next_cells.clone()` at line 228 should produce empty vecs. The `inferred_root_targets` logic (lines 148-186) uses routing rules, not next_cells, for start target inference — verify this is unaffected.
- The `cells` field in WikiWiki's output `MapVariantDefinition` will have cells with `node_label` and `cell_no` but empty `next_cells`.

**Test scenarios:**
- Happy path: WikiWiki parser produces nodes with labels and cell numbers but empty next_cells
- Regression: Routing rules still reference correct cell numbers matching their labels
- Regression: `inferred_root_targets` and start target logic still works with empty next_cells
- Existing: All 67 WikiWiki parser tests pass with updated assertions

**Verification:**
- `cargo test -p emukc_bootstrap --lib -- wikiwiki_map` passes
- No `next_cells` in WikiWiki parser output except for the start cell (which gets its targets from routing rules)

---

- U2. **Add stale sortie state cleanup to port handler**

**Goal:** Clear any stale `ActiveSortieState` when the client hits `api_port/port`, preventing ghost state from affecting subsequent sorties.

**Requirements:** R5, R6

**Dependencies:** None (independent of U1)

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie.rs` — add `clear_sortie_state_if_any` method
- Modify: `crates/emukc_gameplay/src/game/sortie_store.rs` — expose `remove_active_sortie` if needed
- Modify: `src/bin/net/router/kcsapi/api_port/port.rs` — call cleanup before building response
- Test: `crates/emukc_gameplay/src/game/sortie.rs` (inline tests)
- Test: `src/bin/net/router/kcsapi/api_port/port.rs` (inline tests)

**Approach:**
- Add `clear_sortie_state_if_any` to `SortieOps` trait with blanket impl. Method signature: `async fn clear_sortie_state_if_any(&self, profile_id: i64)`. Implementation: call `self.sortie_store().remove_active_sortie(profile_id)` and `clear_pending_sortie_runtime_state()`, both silently no-op if nothing to clean.
- In `api_port/port.rs` handler: call `state.clear_sortie_state_if_any(pid)` before building the port response. The call is fire-and-forget — its result does not affect the port response.
- This ensures that if a client reconnects (hits port after disconnect during sortie), the stale in-memory state is cleaned up before any new sortie can be started.

**Test scenarios:**
- Happy path: Port handler called with stale active sortie → sortie state cleared, port response normal
- Edge case: Port handler called with no active sortie → no error, port response normal (no-op)
- Integration: Start sortie → port handler called → active sortie removed → next sortie starts cleanly

**Verification:**
- `cargo test -p emukc_gameplay` passes
- `cargo test -p emukc --lib` or relevant port test passes
- Manual: start sortie, navigate to port without goback_port, verify state is cleaned

---

- U3. **Clean up dead code and update comments**

**Goal:** Remove dead code from the BFS heuristic removal, update comments in assemble.rs and merge.rs.

**Requirements:** R4

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` — remove dead code around graph construction if `next_cells` is no longer populated
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` — update comments to reflect new data flow
- Modify: `crates/emukc_model/src/codex/map/merge.rs` — update comments

**Approach:**
- After U1, evaluate whether the `graph: BTreeMap<String, BTreeSet<String>>` in `build_nodes()` is still needed (it's used for BFS ordering). If only ordering is needed, simplify to just track labels in order without the edge set.
- Update comments in `assemble.rs` that reference the old `merge_missing_from(wikiwiki)` flow.
- Verify no clippy warnings on changed code.

**Test scenarios:**
- `cargo clippy --workspace` produces no new warnings
- `cargo test -p emukc_bootstrap` passes
- `cargo test -p emukc_model` passes

**Verification:**
- Clean clippy + test run

---

## System-Wide Impact

- **Interaction graph:** Port handler now triggers sortie cleanup. This is a one-way side effect — the port response itself is unchanged. No downstream handlers are affected.
- **Error propagation:** `clear_sortie_state_if_any` silently no-ops on error (no active sortie). No error propagation to port response.
- **State lifecycle risks:** None. The cleanup removes stale state that would otherwise persist until the next `goback_port` or `start_sortie` call.
- **API surface parity:** `api_port/port` response is unchanged. `SortieOps` gains a new method. No other API surfaces affected.
- **Integration coverage:** Port → sortie cleanup → start new sortie flow should be tested as a sequence.
- **Unchanged invariants:** `merge_variant_definition` for public overlays/STAT unchanged. Runtime route evaluation unchanged. `goback_port` handler unchanged (still errors on no active sortie — that's its contract).

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| WikiWiki `inferred_root_targets` depends on `next_cells` | Verify during U1 implementation — the logic uses routing rules, not next_cells |
| Port cleanup race with concurrent sortie operations | Sortie state is per-profile, single-threaded access via Mutex — no race concern |
| Port cleanup adds latency to port response | Cleanup is O(1) HashMap operations — negligible |
| Existing tests depend on WikiWiki producing non-empty `next_cells` | Update test assertions in U1 — parser no longer produces next_cells |

---

## Sources & References

- **Origin plan:** `docs/plans/2026-05-05-003-refactor-map-topology-routing-separation-plan.md`
- WikiWiki parser: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`
- Merge logic: `crates/emukc_model/src/codex/map/merge.rs`
- Assembly: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- Sortie store: `crates/emukc_gameplay/src/game/sortie_store.rs`
- Port handler: `src/bin/net/router/kcsapi/api_port/port.rs`
- Sortie ops: `crates/emukc_gameplay/src/game/sortie.rs`
