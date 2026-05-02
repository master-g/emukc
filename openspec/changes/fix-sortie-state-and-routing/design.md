## Context

The `SortieStore` holds three maps keyed by `profile_id`: active_sorties, pending_results, pending_battles. Practice and sortie sessions share the same store. When a new sortie starts (`start_sortie`), it inserts into `active_sorties` without first clearing any existing entry. If the previous sortie or practice session did not cleanly exit (e.g., client crash, missing goback_port call), stale state persists.

For routing, `evaluate_route_destination` uses `routing_rules` from codex map data. When rules are absent, it falls back to `select_route_from_cells` which reads `current.next_cells`. On 1-3, the fallback may select `next_cells[0]` deterministically instead of following the correct directed graph.

## Goals / Non-Goals

**Goals:**
- Guarantee clean state at sortie start
- Prevent practice→sortie state cross-contamination
- Fix 1-3 routing to follow directed graph edges

**Non-Goals:**
- SortieStore architecture redesign
- Routing algorithm overhaul
- Fixing maps other than 1-3

## Decisions

### D1: Defensive cleanup at sortie start

**Decision**: In `start_sortie` gameplay function, call `remove_active_sortie`, `take_pending_result`, and `take_pending_battle` for the profile before creating the new active sortie state.

**Rationale**: Defense-in-depth. Even if goback_port should have cleaned up, the start handler guarantees no stale state survives. Matches real server behavior where each sortie is independent.

### D2: Practice cleanup audit

**Decision**: Audit practice battle result handler to ensure it clears `pending_battle` and `pending_result` from the SortieStore after processing. If missing, add cleanup calls.

**Rationale**: The reported bug of practice opponents appearing in sortie enemies strongly suggests practice sessions leave residual data in the shared store.

### D3: 1-3 routing investigation

**Decision**: First inspect codex map data for 1-3 to verify routing_rules and next_cells are correct. If data is correct, the bug is in the routing logic fallback. If data is wrong, fix the data source.

**Rationale**: Cannot determine fix without knowing whether the issue is data or code.

## Risks / Trade-offs

- **[Risk] Over-cleanup**: Clearing state at start might lose data if a battle result wasn't claimed → Acceptable: unclaimed results from a previous sortie are already invalid
- **[Risk] 1-3 data vs code**: If the issue is in codex data generation (bootstrap), the fix may need to extend to the decoder/bootstrap pipeline → Limit scope: fix data manually if needed, address pipeline separately
