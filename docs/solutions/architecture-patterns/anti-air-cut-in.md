---
title: "対空カットイン: a table of kinds, rolled once before the anti-air fire"
date: 2026-10-10
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [battle, kouku, anti-air, cut-in, api-air-fire, rng]
applies_when:
  - "Adding a kind of 対空カットイン, or one that names a ship"
  - "A seeded battle changed after a fleet was given high-angle guns or radars"
  - "The client shows no cut-in, or asks for a name plate that does not exist"
---

# 対空カットイン: a table of kinds, rolled once before the anti-air fire

## What

`simulation/aaci.rs` decides whether the friendly fleet fires a 対空カットイン when enemy
aircraft come to strike, and `kouku.rs` `fly_through_anti_air` applies it: every slot loses
the kind's fixed number on top of the one a friendly fleet always takes, and the fixed shot is
multiplied by the kind's modifier inside its floor. The packet gains
`api_stage2.api_air_fire` (`api_idx`, `api_kind`, `api_use_items`), absent when none fired.

## Where the rules come from

`KC3Kai/kancolle-replay` at `69097abc`: `kcsim.js` `AACIDATA` (number, rate, modifier, the
letters of the equipment shown), `orderKnown` (the order kinds are preferred in; a kind's
place in it is its priority), `getAACI`, and `kcships.js` `getAACItype`. The rates are the
ones written in `AACIDATA`; `toggleAACIRework` would swap seven of them and nothing in the
source calls it.

Equipment is classed from master data the way the source does it: 高角砲 is icon 16
(`api_type[3]`), one of 対空 8 or more has its own 高射装置, a 対空機銃 (type 21) of 対空 9 or
more is 集中配備, a radar (types 12, 13, 93) of 対空 2 or more is a 対空電探, type 36 is
高射装置, type 18 三式弾, types 3 and 38 大口径主砲.

## What is in and what is not

In: the kinds formed by equipment and class alone — 1, 2, 3 (秋月型, `api_ctype` 54), 4 and 6
(battleships with 大口径主砲, 三式弾 and 高射装置), 5, 7, 8, 9, 12, 13.

Not in: every kind that names a ship (10, 11 and 14 to 53); the enemy's cut-in
(the client has no `api_air_fire_e` to draw, so it would only be numbers); the source's
exclusion of 摩耶改二 and her like from kind 13, which exists because they have 10 and 11; the
resistance some aircraft have to the fixed number. Adding a kind is a row in `KINDS`, a line
in `kinds_of` at the place the source pushes it, and a letter string.

## The golden rule of this phase

A battle draws for a cut-in only when it can happen:

- nothing is rolled unless an enemy slot that strikes still has aircraft (the source returns
  before `getAACI` when there is no bomber);
- only friendly defenders roll;
- ships are tried in fleet order and each ship's kinds in the source's order, and a kind is
  rolled only if it would replace the one standing. A kind that could not win draws nothing.

So a fleet with no such equipment has the same random stream as before, and no seeded
transcript moved when this was added. Giving a fleet the equipment does move its stream.

## What the client does with it

`api_idx` indexes `deck_f.ships`, so in a combined fleet it goes through the same remap as
every other friendly index (`combined_packet.rs` `split_kouku_stage3`). The client does not
read `api_kind` to draw; it shows the ship and up to three name plates from `api_use_items`,
loaded as `slot/btxt_flat` (`CutinAntiAircraft`). Guns, machine guns, directors, radars and
shells all have a plate, so naming the equipment that forms the kind is safe by construction
(`battle-display-name-plates.md`). The two that lacked one in the cache on 2026-10-10 are the
new 574 and 575, which `BTXT_FLAT_IDS` has not caught up with.

## How it is checked

- `aaci.rs` tests: the table against the source, the kinds each loadout gives, which kind
  stands under fixed rolls, and the untouched stream.
- The `anti_air_cut_in` preset (秋月 with two 10cm連装高角砲+高射装置 and 13号対空電探改, at 2-5,
  whose first battle always meets a carrier) is in the sim→validate gate with
  `PhaseExpectation::AntiAirCutIn`.
- `make headless-check SCENARIO=anti_air_cut_in` plays it in the real client; the cut-in is
  on the screenshots `air1`–`air8`.

## When a test breaks

- The gate says the preset never reached `AntiAirCutIn`: either the route no longer starts at
  2-5 C, or its compositions lost their carriers. Look at the map data before the roll.
- The headless check says no cut-in fired: all three of 秋月's kinds fail about 7 times in a
  hundred. Run it again before reading anything into it.
