---
title: "Land-based air corps: owned per area, sent per sortie, attacking before the fleets meet"
date: 2026-10-08
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [airbase, air-corps, lbas, sortie, battle, kouku]
applies_when:
  - "An air corps endpoint, its costs or what a squadron may fly changes"
  - "A battle should or should not open with api_air_base_attack"
  - "Base defence, air raids on the base or jet assault is added"
  - "Squadron condition, its recovery or the relocation wait changes"
  - "A battle golden changes after touching the air phases"
---

# Land-based air corps

## What

An area has up to three air corps (基地航空隊), each with four squadrons of eighteen, nine
or four aircraft. The player deploys equipment to squadrons, gives each corps an order, and
on a map that allows it sends the corps ordered to sortie against cells. A battle on such a
cell opens with the corps' attacks.

On the regular maps only 6-4 (one corps may sortie) and 6-5 (two) have them.

## Where the rules come from

| Rule | Source |
| --- | --- |
| What may be deployed, and how many aircraft fly | the client: `getEquipTypes`, `getKadouCount` (`squadron_capacity`) |
| Deployment cost: `api_cost` × aircraft, bauxite | the client's own check, and a live account |
| Resupply: 3 fuel and 5 bauxite per aircraft replaced | wikiwiki 基地航空隊 |
| Sortie cost: fuel and ammunition by type and strength | wikiwiki 基地航空隊, 出撃コスト (`sortie_cost`) |
| Maintenance level 0 to 3, reported only once raised | a live `mapinfo`, the client's `AIRBASE_MAX_LEVEL` |
| Condition (fatigue): range, cost of a sortie, recovery by order and 整備Lv | wikiwiki 基地航空隊, 疲労 and 整備Lv強化による効果 — the page marks them 推測される値 |
| Relocation: 12 / 10 / 8 / 6 minutes by 整備Lv, then a visit to port | same page; `api_base_convert_slot` from a live `api_port/port` and the client |
| Attack power, the fight for the air, anti-air fire | `KC3Kai/kancolle-replay` `kcsim.js` / `kcships.js` |
| Packet shape of an attack | the client: `AirUnitData` extends `AirWarDataBase` |

The formulas, their lines and what was left out are tabled in
`docs/plans/2026-10-08-006-feat-air-corps-sortie-plan.md`.

## How a sortie carries them

1. `api_req_map/start_air_base` names up to two cells per corps (the same one twice is two
   attacks). It checks the order, the map's `airbase_count` and each cell's `distance`
   against the corps' radius, charges the sortie cost, and keeps the choice on
   `ActiveSortieState::air_strikes`. Once a sortie.
2. Battle setup collects the corps pointed at the node the fleet stands on
   (`striking_air_corps_impl`). A node has one cell number per edge leading into it and the
   client names whichever it drew, so the match is by `node_label`, not by cell number.
3. `emukc_battle` flies them before anything else (`simulation/air_base.rs`): per corps, one
   attack per time it was pointed here, each a fight for the air, anti-air fire and a strike.
   Losses carry to the next attack.
4. Settling the battle writes each corps' last `remaining` counts back to `plane_info`.

## Condition and relocation run on one timestamp

`plane_info.condition` is an inner value from 0 to 46 that never leaves the server; the
wire's `api_cond` is its tier (`cond_tier`: 1 untired, 2 orange, 3 red), which is all the
client draws. `plane_info.since` means "last recovery tick" on a flying squadron and
"relocation began" on a relocating one.

Nothing runs on a clock. `settle_conditions_impl` applies the ticks owed whenever a
condition is about to be read or its rate is about to change (a new order, a new 整備Lv, a
swap between two corps) — call it first in any new airbase operation. A sortie takes its 6
or 8 in `start_air_base`, not on the way home: upstream takes it whatever happened in
between, and a sortie ends in more places than it starts.

A relocation is settled by the port view alone (`settle_relocations_impl`), which also
names the equipment still waiting for `api_plane_info.api_base_convert_slot`. Until then a
relocating row stays in its slot, so **a slot can hold two rows**: the squadron that
replaced it flies, and readers by `squadron_id` must take the `Assigned` one
(`squadrons_of`, `find_assigned_squadron`).

`since` was added to a table that had already shipped: after `create_table`, an
`ALTER TABLE ... ADD COLUMN` whose failure is ignored, and on the one start where it
succeeds the old rows' `condition` (then the wire value) is reset to 40. There is still no
migration table; write one when a change cannot be told from "the add failed".

Moving a squadron between two slots of one corps trades the rows as they are. It used to
delete and redeploy, which refilled and rested the squadron for nothing.

## The golden rule of this phase

A battle without air corps does not enter the phase and draws not one random number more.
`BattleContext::air_corps` is empty for every battle but the ones above, which is what kept
the forty goldens unchanged when the phase was added (they gained the line
`air_base_attack: []` and nothing else). Keep the gate where it is.

## Level of detail

Every strike hits and a target is any enemy afloat, as in the carrier air battle
(`kouku.rs`). The anti-air stage is not simplified the same way: one enemy ship fires on
each squadron, as in the source, because the carrier battle's estimate (the whole fleet's
anti-air over 400) empties a squadron against six ordinary ships. Raising the rest (hit
rolls, contact, per-equipment bonuses) should be done for both air battles together.

A corps of bombers without fighters loses heavily to an enemy with aircraft: that is the
air state, not the anti-air fire.

## Known gaps

- A tired squadron fights as well as a fresh one. Upstream lowers its accuracy by an
  amount nobody has published, and the attacks here have no hit roll to lower.
- 休息 does not halve the bauxite regeneration (no figure upstream).
- What a 航空特別増加食 restores has no source; it brings a squadron below 40 back to 40.
- The server lets a relocating plane be deployed again; only the client stops it.
- Base defence (the 防空 order), air raids on the base (`api_destruction_battle`), jet
  assault (`api_air_base_injection`) and 超重爆迎撃 (`api_req_map/air_raid`).
- A friendly combined fleet does not get air corps attacks; 6-4 and 6-5 cannot be sortied
  with one.
- The maintenance level lives on the area's airbase rows, so an area without an air corps
  cannot be raised, and the event area is not listed at level 0.
