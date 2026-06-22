---
title: "Fleet composition, management, remodel equipment assignment, and slot/HP correctness"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing fleet slots, ship assignment, presets, or mission status"
  - "Modifying ship remodel equipment/slot/onslot/HP logic"
  - "Auditing Codex ship slot aircraft capacity data"
tags: [fleet, remodel, equipment-slots, aircraft-capacity, repair-time, fleetops]
related_components: [emukc_model, emukc_db]
---

# Fleet composition, management, remodel equipment assignment, and slot/HP correctness

## Context

Fleets (up to 4 decks of 6 ship positions) are managed by `FleetOps`
(`emukc_gameplay`), with presets via `PresetOps` and resupply via
`ComposeOps::charge_supply`. This contract also covers remodel-time equipment
assignment correctness, aircraft-capacity data audit bounds, existing-data
repair, HP restoration, and the CT repair-time modifier. Migrated from
the retired openspec fleet capability spec (see `docs/migration/openspec-sunset-log.md`).

## Guidance

### Fleet slots

Each profile SHALL have up to 4 fleet slots (decks), each holding up to 6 ship
positions, via `FleetOps`.

- Initial state: on profile initialization, fleet slot 1 is unlocked via
  `unlock_fleet_impl` with no ships assigned (all -1); slots 2–4 do not exist
  yet.
- Fleet slot unlock (`unlock_fleet`) for index 2/3/4 creates a new fleet
  record with empty ship slots (all -1).
- `unlock_fleet` for index 1 (already exists) or any index outside 2–4 SHALL
  fail.

### Ship assignment

Ships SHALL be assigned as an ordered array of 6 ship IDs (-1 for empty) via
`FleetOps::update_fleet_ships`.

- Valid index + 6-element array: positions updated to match, stored in order.
- Invalid fleet index (no fleet record) SHALL fail with an `EntryNotFound`
  error.

### Retrieval

- `get_fleet(profile_id, index)` returns the fleet with its ship assignment and
  name; if the fleet has `InMission` status but `return_time` has passed, the
  status is updated to `Returning`.
- `get_fleets(profile)` returns all fleet records ordered by index ascending.
- `get_fleet_ships(fleet)` returns ship records in the same order as the
  fleet's ship array.

### Naming

`update_deck_name` updates the fleet's name for a valid fleet index.

### Resupply

Fleets consume fuel/ammo during sorties and MUST be resupplied via
`ComposeOps::charge_supply`:

- Fuel and ammo are deducted from the profile's materials; each ship's current
  fuel/ammo are restored toward maximum values.
- Insufficient fuel or ammo for full resupply SHALL fail.

### Presets

Profiles SHALL save/load fleet composition presets via `PresetOps`: saving
stores ship IDs; loading replaces the current composition with the preset's
ships.

### Mission status normalization

A fleet retrieved with `InMission` status whose `return_time` has passed SHALL
have its status automatically updated to `Returning` before being returned.

### Remodel correctly assigns equipment to the slot array

When a ship is remodeled, newly created equipment items SHALL be assigned to
`api_slot` (equipment slot IDs), not `api_onslot` (aircraft capacity). The
`api_onslot` array SHALL retain values from `codex.new_ship()` reflecting the
ship's actual aircraft capacity per slot.

- With N default equipment items, each item's DB ID is written to
  `api_slot[0..N]`; `api_onslot` retains the codex `api_maxeq` values.
- With no default equipment (all `item_id == 0`), `api_slot` remains `[-1; 5]`
  and `api_onslot` retains correct capacity values (may be all 0 for non-CV
  ships).

### Ship slot aircraft capacity correctness

`api_maxeq` / `onslot` values in the Codex SHALL match real KanColle data.
Non-CV/CVL/CVB ship types SHALL NOT have slot capacities exceeding reasonable
bounds:

- CV/CVL/CVB: follow official data (can be 0–40+ per slot).
- BB/BBV: seaplane bomber slots typically 0–4.
- CA/CAV/CL/CLT/DD: aircraft slots typically 0–4 (most 0 or 1).
- SS/SSV: typically 0–1 for any aircraft slot.
- AO: typically 0–4.

Any slot capacity exceeding these bounds SHALL be flagged as a data error. On
load into the Codex: no CA-class ship has any slot > 4 or total > 8; no BB-class
ship has any slot > 4; no DD-class ship has any slot > 1.

### Existing remodel data repair

Ships remodeled before the slot/onslot fix SHALL have corrupted `onslot_*` /
`slot_*` values repaired: `onslot_*` reset to codex `api_maxeq` values;
`slot_*` restored to correct equipment assignment. If all ships are within
expected bounds, the repair makes no changes.

### HP restoration after remodel

After `cal_ship_status` computes the new max HP for a remodeled ship, the
system SHALL set `api_nowhp = api_maxhp`. A partial-HP ship before remodel
ends at full HP after remodel; a full-HP ship before remodel also ends at full.

### CT ship repair time modifier

The repair time calculation SHALL use the verified CT (練習巡洋艦) ship-type
modifier. A CT and a CL of the same level with the same HP deficit SHALL differ
in repair time if the CT modifier differs from CL.

## Why This Matters

The slot/onslot distinction and capacity bounds are a recurring source of
data-corruption bugs (remodel writing equipment IDs into aircraft capacity
fields). Mission-status normalization prevents the client from showing a fleet
as "still on expedition" after it has returned. The CT modifier correctness
keeps repair times accurate for that specific ship type.

## When to Apply

- When modifying fleet assignment, presets, resupply, or mission status.
- When touching the remodel path or ship status calculation.
- When auditing or regenerating Codex `api_maxeq` ship data.

## Examples

- A remodeled ship with 3 default equipment items gets those IDs in
  `api_slot[0..3]` while `api_onslot` keeps codex capacity.
- A returned expedition fleet auto-flips from `InMission` to `Returning` on
  retrieval.
- A CA ship with a slot capacity of 6 is flagged as a data error on Codex load.

## Related

- `docs/solutions/architecture-patterns/material.md` — resupply consumes
  materials via `charge_supply`.
- `docs/solutions/architecture-patterns/quest.md` — `unlock_fleet_impl` reward.
