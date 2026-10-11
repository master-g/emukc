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

The kinds that name a ship (plan `2026-10-11-002`) name her by master id or class, and some
of their equipment by master id too, exactly as `getAACItype` does. Three of its switches are
taken at their defaults: 吹雪改二, 改三 and 改三護 fire kinds 49 to 52, 大和改 and 武蔵改 fire
kind 26, and 秋月型's kind 2 wants a 高角砲 with its own 高射装置.

wikiwiki 対空砲火 was read against the table on 2026-10-11: it lists the same 53 kinds and the
same ships, and gives no rates, so the rates have one source.

Equipment is classed from master data the way the source does it: 高角砲 is icon 16
(`api_type[3]`), one of 対空 8 or more has its own 高射装置, a 対空機銃 (type 21) of 対空 9 or
more is 集中配備, a radar (types 12, 13, 93) of 対空 2 or more is a 対空電探, type 36 is
高射装置, type 18 三式弾, types 3 and 38 大口径主砲.

## What is in and what is not

In: all 53 kinds of the source. Adding one is a row in `KINDS`, a line in `kinds_of` at the
place the source pushes it, and a letter string.

Seven kinds (34 to 37, 39 to 41) show equipment the source writes out by id, and kind 32 the
first of three pairs the ship carries; `shown` does the same. So kind 40 shows two 362 and a
307 on a ship that carries a 363: the plates exist, they are just not always hers.

Not in: the enemy's cut-in (the client has no `api_air_fire_e` to draw, so it would only be
numbers); the resistance some aircraft have to the fixed number.

## Where it differs from wikiwiki

The figures are the source's throughout. wikiwiki disagrees on five kinds:

| Kind | Here (fixed, modifier) | wikiwiki |
| --- | --- | --- |
| 27 | 6, 1.55 | 5, 1.55 |
| 45 | 6, 1.55 | 5, 1.55 |
| 49 | 6, 1.5 | 5, 1.5 |
| 51 | 5, 1.4 | 5, 1.35 |
| 52 | 5, 1.4 | 4, 1.4 |

It also lists only 大和型改二 for kind 26, and wants a 機銃 of 対空 3 or more for kinds 12, 23
and 24 and of 4 or more for kind 33, where the source takes any 機銃.

One difference from the source itself, older than the named kinds: the letter `M` takes both
classes of 大口径主砲 (types 3 and 38) where the source takes the first alone. It only decides
which plate kinds 4 and 6 show.

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

- `aaci.rs` tests: the whole table against the source, the kinds each loadout gives (one
  ship for each sort of condition, not one for each kind), the equipment shown, which kind
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
