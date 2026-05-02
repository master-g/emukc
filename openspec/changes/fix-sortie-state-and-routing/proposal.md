## Why

Two related sortie bugs: (1) Starting a new sortie does not clean up residual state from a previous sortie or practice session, causing corrupted enemy compositions (practice opponents appearing in sortie battles) and skipping ahead to boss nodes. (2) Map 1-3 routing does not follow the directed graph edges defined in map data, suggesting either incorrect codex data or routing logic fallback issues.

## What Changes

- Add state cleanup at the start of `start_sortie` to remove any residual active sortie, pending battles, and pending results for the profile
- Verify and fix practice battle session cleanup to prevent cross-contamination with sortie battles
- Investigate 1-3 map routing data in codex and fix routing to follow directed graph edges

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `sortie`: Sortie start must clean up previous state; practice end must not leak into sortie
- `pathrules-loading`: Verify 1-3 map routing data correctness and fix edge definitions if needed

## Non-goals

- Changing the routing algorithm itself (evaluate_route_destination logic)
- Changing SortieStore architecture
- Map data for maps other than 1-3 (unless investigation reveals broader issues)

## Impact

- `crates/emukc_gameplay/src/game/sortie.rs` — start_sortie cleanup
- `crates/emukc_gameplay/src/game/sortie_store.rs` — may need explicit clear-before-insert
- `crates/emukc_gameplay/src/game/battle/practice.rs` — verify cleanup on practice end
- `src/bin/net/router/kcsapi/api_req_map/start.rs` — start handler
- `src/bin/net/router/kcsapi/api_req_sortie/goback_port.rs` — goback handler
- Codex map data for map 1-3 — may need correction
