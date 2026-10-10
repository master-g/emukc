---
title: "Plane losses that last, and proficiency that grows with the battles flown"
date: 2026-10-10
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [sortie, proficiency, aircraft, onslot, supply, slot-item]
applies_when:
  - "A battle result needs to keep something the battle changed on a ship"
  - "Proficiency growth or loss is tuned, or a new aircraft needs a growth constant"
  - "Aircraft counts or bauxite cost look wrong after a sortie"
  - "A code path sets `aircraft_lv`"
---

# Plane losses and proficiency growth

## What it is

A sortie battle result writes three things about aircraft, in the same transaction as HP and
ship experience (`sortie_result.rs` `update_sortie_result_stats`):

1. **What each slot has left.** The snapshot carries `friendly_onslots`, taken from the ships
   the simulation returned, and it goes into `ship.onslot_*`. Resupply puts the aircraft back,
   every slot, at 5 bauxite each.
2. **Each slot's proficiency experience**, `slot_item.aircraft_exp` (0 to 120), settled by
   `game/proficiency.rs` `settle` from the count before and after.
3. Nothing else: the level, `slot_item.aircraft_lv`, is **not** touched by a battle.

`port_view` then brings every level into line with its experience
(`refresh_proficiency_levels_impl`). So a level never moves between two battles of one sortie:
the client and the next battle both read `aircraft_lv`, and it changes when the fleet is home.

## The rules

When proficiency changes follows the wikiwiki page 艦載機熟練度; how much is a fit to player
statistics (the article and sheets named at the top of `proficiency.rs`), which its author
marks as estimates. Nothing here was read from the game.

- A slot that had no aircraft before the battle is left alone. One that lost all of them
  starts over at 0.
- Otherwise it loses first: `M = floor(c x exp x lost / before)`, `c` 0.5 when more than half
  were lost and 0.3 otherwise, and the loss is between `0.45 M` and `0.5 M`. Scouts skip this.
- Then it grows, once for each roll its kind gets: fighters, bombers, torpedo bombers, seaplane
  fighters and jets in a battle with an air phase; scouts and flying boats in every battle
  (search always succeeds in the simulation); seaplane bombers in both.
- One roll adds `floor(k x (0.5 + 0.044 x r))`, `r` uniform in `0..A-1`, `A` being 12, 10, 8, 6
  for experience under 20, 50, 100 and from 100. `k` is the aircraft's growth constant, 2 to 9,
  from the `GROWTH` table; an aircraft the table lacks takes the median, 6.
- Levels start at `0, 10, 25, 40, 55, 70, 85, 100`. Equipment handed out at a level gets
  `0, 10, 25, 40, 55, 70, 85, 120`, the table the battle reads a level with.

## Why it is shaped this way

- **Experience per battle, level at port.** The source says nothing takes effect until the
  fleet returns. Settling the experience with the battle result needs no running total in the
  sortie session and no write at each of the four places a sortie can end; refreshing the level
  at port keeps the one thing a player or a battle could notice. What differs from the source:
  each battle is computed from the experience the last one left rather than from the value at
  departure, and a sortie cut short by a disconnect keeps what its battles settled.
- **Whole points.** The sheet keeps fractions until port; here each battle's loss is rounded.
- **The table is source, not Codex data.** Only the settlement reads it. Putting it in the
  Codex would mean an asset, a parser, a field and a rebuild for one lookup.
- **`aircraft_lv` stays a column.** Everything that reads a level keeps working; the rule is
  that whatever sets `aircraft_lv` sets `aircraft_exp` with it (`exp_of_level`).

## Not modelled

The air corps and air defence; patrol planes and autogyros, which grow when their ship attacks
a submarine; the jet assault's extra roll; the escort fleet's growth (it needs an enemy
combined fleet); scouts' gradual loss. The battle still reads a level's representative
experience rather than the real one. A night-start battle counts as a search like any other,
which the sources do not settle. Practice settles no proficiency; the aircraft it costs were
already kept, and resupplying them now charges for every slot.

## Known deviation: losses come off the largest slot

`emukc_battle` works out how many aircraft a side loses in a phase and takes them one at a
time from whichever slot has the most left (`kouku.rs` `apply_plane_losses`). The game rolls
each slot's losses on its own, in proportion, so there a small slot is the one that gets wiped.
Here a small slot almost never loses anything: Akagi's 18/18/20/10 came out of one battle at
15/15/16/10. That was invisible while aircraft came back after every battle. Now it decides
which slots cost bauxite, which lose proficiency, and it makes "wiped, back to 0" rare where
the game makes it common. Fixing it belongs to the battle simulation and moves its baselines.

## Slots with gaps

The battle walks `slot_items`, which skips empty slots, and pairs it with `api_onslot` by
position. Equipping and unequipping through the API close any gap when the ship is
recalculated, so the two normally line up. Should a ship reach a sortie with a gap anyway
(`slot_1` empty, `slot_2` filled), `build_sortie_friend_ships` closes the same gap in the
counts on the way in and the battle result puts each count back in the slot its equipment
sits in (`occupied_slots`). No test builds such a ship: nothing public leaves one.

## When something looks wrong

- A database made before this has no `aircraft_exp` column and fails on the first equipment
  read. The project keeps no migrations (`36da3d2d`): recreate it.
- Aircraft that never come back after resupply: check that every slot is in the list in
  `compose/supply.rs`. Only the first was, unnoticed, while nothing ever lost aircraft.
- A seeded gameplay test that fights more than one battle with carriers sees fewer aircraft in
  the later ones, and the settlement draws from the same thread-local generator as everything
  else. Change the seed rather than loosen the assertion.
