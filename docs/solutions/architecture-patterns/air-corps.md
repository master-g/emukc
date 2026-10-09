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
names the equipment still waiting for `api_plane_info.api_base_convert_slot`. The port that
releases a squadron also sends `api_unset_slot` with the whole unequipped list of that
equipment type: the client dropped the item from its own list on deployment
(`main.decoded.js:14684`) and learns of its return nowhere else. Until then a
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

## The raid on the air base

6-5 is the only regular map whose sortie brings one. Its raiders come from KCNav's node `AB`
(`maps/{map}/nodes/AB/enemycomps`), a node no edge enters; a map variant carries them as
`air_raid_fleets`, and a map with none asks nothing more of `next_sortie`, not even a random
number.

The raids against the gauge's last bar are kept apart (`last_bar_air_raid_fleets`): 6-5 sends
its strongest fleet then and never before. Which fleets those are is asked of KCNav, not read
off their ships: the same node queried with `minGaugeLevel=1&maxGaugeLevel=1`, and whatever
answers is taken out of the whole-gauge list. Two groups are all 6-5 has; a map whose raids
change bar by bar would need the query per bar.

When it comes is a rule of ours (`RAIDS_FROM_BOSS_KILLS` and the lines under it in
`sortie/mod.rs`): the boss sunk twice and the gauge unbroken, half the time on reaching a
battle cell, for certain on the boss cell, once a sortie. Upstream publishes only "random,
likelier further in"; KCNav has no raid before the gauge is down two of six and none after it
is broken, which is where the two came from.

The battle itself has a source: `simLBRaid` in `kancolle-replay`'s `kcsim.js`, in
`emukc_battle::simulation::air_raid` — the defenders' fighter power (a land fighter adds its
interception and twice its anti-bomber figure, the best reconnaissance aircraft multiplies the
corps), the share of each raiding slot shot down, and bases of 200 that keep their last point.
What it costs follows wikiwiki (`settle_air_raid_impl`): fuel or bauxite for nine tenths of the
damage, and 1 to 4 aircraft off the first squadrons of a base that took 50 or more, unless it
was ordered to shelter.

The raid goes out inside `api_req_map/next` as `api_destruction_battle`, which the client
reads as a day battle record whose friendly side is the bases. `api_plane_from[0]` and
`api_map_squadron_plane` are null when nobody defends, and the latter is keyed by the base's
place in the area as a string.

## Known gaps

- A raid always costs stores when it does damage; upstream that is random at a rate nobody
  published. Defending does not tire a squadron (no source says it does), 改修 and 熟練度 add
  nothing to the defenders' fighter power, and the high-altitude modifier is left out.

- A tired squadron fights as well as a fresh one. Upstream lowers its accuracy by an
  amount nobody has published. The hit roll itself is in
  `battle-hit-and-critical-rolls.md`; a raid on the base lands 95 times in a hundred.
- 休息 does not halve the bauxite regeneration (no figure upstream).
- What a 航空特別増加食 restores has no source; it brings a squadron below 40 back to 40.
- The server lets a relocating plane be deployed again; only the client stops it.
- Jet assault (`api_air_base_injection`) and 超重爆迎撃 (`api_req_map/air_raid`), which only
  event maps have.
- A friendly combined fleet does not get air corps attacks; 6-4 and 6-5 cannot be sortied
  with one.
- The maintenance level lives on the area's airbase rows, so an area without an air corps
  cannot be raised, and the event area is not listed at level 0.
