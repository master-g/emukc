---
title: "Sortie state machine, battle sequencing, and cell-data correctness"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing or modifying sortie start, battle node resolution, or XP/level clamping"
  - "Authoring Codex map cell metadata (boss_cell_no, color_no, event_id, event_kind)"
tags: [sortie, battle, state-machine, map-cell, level-cap, sortieops]
related_components: [emukc_battle, emukc_model]
---

# Sortie state machine, battle sequencing, and cell-data correctness

## Context

A sortie is a stateful progression through a map. The `SortieOps` trait
(`emukc_gameplay`) manages the in-memory `SortieStore` keyed to a profile, the
fleet consumption gating, battle node simulation via the `emukc_battle`
subsystem, and the Codex map cell metadata that the client relies on for
correct UI (battle triggers, boss cell, level caps). Migrated from
`openspec/specs/sortie/spec.md`.

## Guidance

### Day battle simulation

When the player encounters a battle node, a day battle SHALL be simulated via
`SortieOps::sortie_battle`.

- For a normal day battle, enemy fleet composition is resolved from the Codex
  map cell definition (or the fallback enemy builder on the degraded path); the
  simulation produces an optional aerial phase, shelling phases, and a torpedo
  phase; damage is recorded in the `SortieStore` as a pending result; the
  response includes `api_hourai_flag` indicating which phases occurred.
- All damage fields in the response (`api_damage`, `api_fydam`/`eydam`,
  `api_fdam`/`edam`) SHALL contain **effective** damage values
  (post-sinking-protection), and HP tracking SHALL use effective (clamped)
  damage internally, so client HP animation matches server state.
- For `airbattle` cells, only the aerial phase is simulated — no shelling or
  torpedo phases.
- For `ld_airbattle` / `ld_shooting`, the appropriate battle mode runs with its
  specific phase configuration, and midnight battle is disabled.
- Enemy selection uses weighted node compositions to select from available
  enemy fleets in the Codex; fallback enemy fleets are used only when Codex
  enemy data is missing (degraded path).

### Sortie state machine

A sortie SHALL be a stateful progression through a map, managed by an
in-memory `SortieStore` keyed to the profile.

- Sortie start: when a valid fleet on a valid map area/stage starts a sortie,
  the fleet's fuel and ammo are reduced by the map's consumption rate; a new
  `ActiveSortieState` is created with map cell data; the starting cell is
  determined by the map definition; the response includes `cell_data` with
  `api_passed: 0` for ALL cells (none visited yet), plus map area/stage
  identifiers and the initial cell position.
- Sortie start SHALL fail when: the selected fleet is already in a sortie or
  on an expedition; the specified map area/stage does not exist in the Codex;
  any ship in the selected fleet has HP of 0 (sunk); or the specified map has
  `unlocked = false` for the player (error response).

### Map cell data correctness

The Codex map catalog (`map_catalog.json`) SHALL contain correct cell metadata
matching the real KanColle game data for all maps with available API captures:

- `boss_cell_no` SHALL match the real `api_bosscell_no`.
- `color_no` per cell SHALL match real `api_color_no` values.
- `event_id` and `event_kind` SHALL be inferred from `color_no` using the
  standard mapping (color 0=start, 2=resource, 3=maelstrom, 4=battle, 5=boss,
  9+=special).
- A battle node (real `api_color_no` = 4) SHALL have `event_kind = 1` (battle)
  and `event_id = 4`, so the client correctly triggers battle UI.

### Unmarried ship level cap enforcement

The system SHALL prevent unmarried ships from exceeding level 99 through any
XP-granting mechanism. When a ship is not married and has reached level 99, XP
gain SHALL be set to 0 and the level SHALL NOT increase.

- Practice XP and sortie XP SHALL both be blocked at level 99 for unmarried
  ships.
- A married ship at level 99 MAY exceed level 99 up to the married cap.
- An unmarried ship at level 98 gaining enough XP to reach level 100 SHALL be
  clamped to level 99, with excess XP beyond level 99 discarded.

## Why This Matters

Sortie is the central gameplay loop. Cell metadata correctness drives the
client UI; a battle node mislabeled as safe silently breaks routing and
encounters. Effective-damage reporting keeps HP animation in sync with server
state, preventing client/server desync. The level cap preserves the
marriage/leveling progression economy.

## When to Apply

- When implementing or modifying `SortieOps` or the `SortieStore`.
- When authoring or regenerating Codex `map_catalog.json` cell metadata.
- When adding any XP-granting mechanism (sortie, practice, quest rewards).

## Examples

- `api_hourai_flag` response field tells the client which day phases occurred.
- A cell with real `api_color_no = 4` becomes `event_kind = 1, event_id = 4`
  so the client triggers battle UI on arrival.
- An unmarried level-99 ship that would gain 500 XP instead gains 0 and stays
  level 99; the same ship married continues leveling to the married cap.

## Related

- `docs/solutions/architecture-patterns/material.md` — consumption and caps.
- `docs/solutions/architecture-patterns/map-unlock.md` — `unlocked = false`
  gating.
