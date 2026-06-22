---
title: "Map unlock progression: prerequisite chains, per-player state, and sortie gating"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing or modifying map prerequisite chains or unlock cascades"
  - "Wiring api_get_member/mapinfo filtering or api_req_map/start gating"
  - "Migrating existing accounts to the unlock-state model"
tags: [map, unlock, progression, prerequisites, mapinfo, sortie-gating]
related_components: [emukc_model, emukc_db]
---

# Map unlock progression: prerequisite chains, per-player state, and sortie gating

## Context

Maps become available to players through a prerequisite chain tracked per
player. The Codex (`MapCatalog`) defines prerequisites; an `unlocked` boolean
on the `map_record` entity tracks per-player state; sorties are gated on it;
and clearing a map cascades unlocks to dependents. Migrated from
the retired openspec map-unlock capability spec (see `docs/migration/openspec-sunset-log.md`).

## Guidance

### Map prerequisites defined in codex

The Codex (`MapCatalog`) SHALL define per-map prerequisite data mapping each
regular map ID to the map that must be cleared first. Map 1-1 SHALL have no
prerequisite (always available).

- Same-area sequential unlock: map N-M's prerequisite is N-(M-1) (e.g., 1-2
  requires 1-1, 1-3 requires 1-2).
- Cross-area unlock: clearing area N's final map (N-4) makes area (N+1)'s first
  map ((N+1)-1) available (e.g., 1-4 → 2-1, 2-4 → 3-1).
- First map always available: map 1-1 has no prerequisite and is always
  unlocked.

### Per-player unlock state tracked in database

The system SHALL track map unlock state per player via an `unlocked` boolean
on the `map_record` entity. Only unlocked maps SHALL appear in
`api_get_member/mapinfo` responses.

- New account initialization: only map 1-1 has `unlocked = true`; all others
  are `false`.
- Unlocked maps are returned in `mapinfo`; locked maps (`unlocked = false`)
  SHALL NOT appear.

### Sortie gated by unlock status

The system SHALL reject sortie requests to locked maps via
`api_req_map/start`.

- Sortie to an unlocked map (`unlocked = true`) begins normally.
- Sortie to a locked map (`unlocked = false`) SHALL return an error response
  (`api_result = -1`).

### Unlock cascade on map clear

The system SHALL automatically unlock dependent maps when a prerequisite map is
cleared (first clear, boss defeated), returning newly unlocked map IDs via
`api_next_map_ids` in the battle result response.

- Clearing 1-1 unlocks 1-2; `api_next_map_ids` contains `[12]`.
- Clearing 1-4 (area boss) unlocks 2-1; `api_next_map_ids` contains `[21]`.
- Clearing a map whose dependents are already unlocked: `api_next_map_ids` is
  absent or empty.
- When clearing a map unlocks more than one dependent (rare edge case), all
  newly unlocked map IDs appear in `api_next_map_ids`.

### Existing account migration preserves access

The system SHALL migrate existing accounts by setting `unlocked = true` for all
maps already cleared or whose prerequisites are satisfied.

- An account with maps 1-1 through 3-4 cleared: all maps in areas 1–3 are set
  to `unlocked = true`.
- An account that cleared 1-1 but not 1-2: 1-1 and 1-2 are unlocked (1-2's
  prerequisite satisfied), but 1-3 and later remain locked.

## Why This Matters

Unlock progression is the PvE gating that prevents skipping ahead. Per-player
`unlocked` state is what `mapinfo` and sortie-start both check; getting it
wrong either hides accessible maps from the client or lets players sortie to
locked maps. The cascade + `api_next_map_ids` is what makes clearing a boss
feel responsive in the client.

## When to Apply

- When modifying map prerequisite definitions or the unlock cascade.
- When wiring `mapinfo` filtering or `api_req_map/start` gating.
- When writing an account migration that must preserve existing access.

## Examples

- New account: only 1-1 unlocked.
- Clearing 1-4 returns `api_next_map_ids: [21]` so 2-1 unlocks.
- Migration sets `unlocked = true` for cleared maps and their satisfied
  dependents.

## Related

- `docs/solutions/architecture-patterns/sortie.md` — sortie start also checks
  sunk-ship and invalid-map conditions.
- `docs/solutions/architecture-patterns/map-data-authority.md` — how the
  underlying map cell metadata is assembled.
