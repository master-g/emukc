---
title: "Night attack types: a generated table, a list per ship, one roll each in order"
date: 2026-10-12
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [battle, night, cut-in, double-attack, star-shell, api-sp-list, rng]
applies_when:
  - "Changing what a ship does at night from her equipment, or how often"
  - "A seeded night battle changed after a ship was given a second gun, a torpedo or a lookout"
  - "The source's NBATTACKDATA changed, or a new cut-in appeared in it"
---

# Night attack types: a generated table, a list per ship, one roll each in order

## What

At night a surface ship's equipment gives her a short list of attack types
(`night.rs` `night_attack_types`). She rolls for each in order, the first that comes up is her
attack, and if none does she makes a plain one (`resolve_night_attack`). There is no falling
back from a failed cut-in to a double attack: the double attack is itself the one general type
of a ship with two guns, rolled at 99%.

The list is a destroyer's four cut-ins, each present or not on its own (main gun, torpedo and
surface radar; lookout, torpedo and surface radar; torpedo squadron lookout and two torpedoes;
that lookout, a torpedo and a drum), followed by the one general type the first of these makes:
three main guns, two main guns and a secondary, two torpedoes, a main gun and a torpedo, two
guns between main and secondary. A carrier flying a night air group has her own cut-ins
instead (`carrier-night-air-attack.md`), and an attack on a submarine is always plain.

## Where the rules come from

`KC3Kai/kancolle-replay` at `69097abc`: `kcsim.js` `NBATTACKDATA` and `NBattack`,
`kcships.js` `NBtypes` and `NBchance`, `nightPhase` for the star shells.

The numbers of every type (damage, accuracy, what the chance is divided by, hits, whether it
carries a torpedo, and the two-hit attack a destroyer of level 80 may make instead) are
`simulation/night_attack_table.rs`, written by `scripts/gen_night_attack_table.py`. Do not
edit it; run the script, and `the_night_attack_table_follows_the_source` compares it with the
rows the script prints. `NightAttackType` reads its four properties from it. What `NBattack`
writes as literals stays in `night.rs`, with the function named beside it: the submarine's
torpedo cut-in with late-model torpedoes (1.75 at chance / 1.05 with her radar, 1.6 at
chance / 1.1 with two) and the D-type guns (267, 366) on the destroyer cut-ins made with a radar.

wikiwiki 夜戦 was read against it on 2026-10-12. It agrees on multipliers, hits, equipment,
every term of the chance and the order of the destroyer cut-ins, and differs in three places,
where the source is followed: the coefficient of the torpedo squadron lookout cut-in (126
there, 125 here), the two-hit main gun, torpedo and radar cut-in (it has 115 only; the source
rolls for the second hit first and then at 130), and the double attack (about 95% or more,
unverified, there; 99% here).

## The chance

`night_cut_in_chance` is one formula for every cut-in, in percent: with luck under 50,
`floor(luck + 15 + sqrt(level) * 0.75)` (her current luck, `api_lucky[0]`), otherwise `65 + floor(sqrt(luck - 50)) +
floor(sqrt(level) * 0.8)`; 15 for the flagship; 8 for the torpedo squadron lookout on a
destroyer, light cruiser or torpedo cruiser and 5 for the plain lookout on anyone, both if she
has both; 18 at moderate damage; 4 and 7 for her own fleet's star shell and searchlight, minus
10 and 5 for the enemy's. The attack type divides it (`chance_modifier`, 1.15 for 115).

Her own fleet's star shell also adds 5 to the base accuracy of 69, before a night scout in
contact multiplies it (`Aim::base`, the source's `NBattack`). It counts for every night attack on
a surface ship, the special attacks' rows among them, and not for an attack on a submarine,
which rolls as by day.

A searchlight draws fire and costs its ship her evasion (`kcsim.js` `nightPhase`,
`choiceWProtect`, `NBattack`):

- As the night begins each fleet takes the most any ship afloat has: 1 for a searchlight, 2 for a
  large one (`fleet_searchlight`, `searchlight_rerolls`). A night target of that fleet without a
  searchlight is drawn again up to that many times, stopping on one that has one
  (`select_night_target`; the special attacks' rows through `select_surface_target`). A
  submarine is drawn once. Each redraw is one more roll, before かばう.
- A ship carrying a searchlight keeps a fifth of her formation's night evasion multiplier and of
  her flat night evasion (`night_evasion`): 5 for a heavy or aviation cruiser, 10 for a destroyer
  with a radar and a lookout, added after the evasion curve. The flat evasion is counted without
  a searchlight too. Whether hers is the lit one does not matter.
- A fleet with one lit adds 7 to the accuracy of each of its night attacks on a surface ship
  (`Aim::lit_by_searchlight`; `NBattack`: `if (searchlights[0]) acc += .07`), after every
  multiplier. The source adds it after the gun fit and before the PT boat's correction, neither
  of which is modelled (`accuracy.rs`). The special attacks' rows and the night 瑞雲 cut-in take
  it too; the enemy's searchlight does nothing to it.

## The night 瑞雲 cut-in

A friendly light or torpedo cruiser, aviation cruiser or battleship or seaplane tender with two
main guns and 試製 夜間瑞雲(攻撃装備) (490) in a slot with aircraft left may make it in her turn
instead of her attack (`zuiun_night_attack`, the source's `canSpecialAttack` and
`NBATTACKDATA` 2001 to 2004). The enemy never does.

- She must attack at night with more than half her hull, and a surface ship must be afloat; a
  fleet of PT boats alone is no bar here.
- Her types, in order: two 瑞雲 and a surface radar (1.36), two 瑞雲 (1.32), one and a radar
  (1.28), one (1.24). One roll each until one comes up, against the night cut-in chance less 14
  for her own fleet's star shell and 17 for its searchlight, 3 more for 熟練見張員, 8 less for a
  destroyer or light or torpedo cruiser with 水雷戦隊 熟練見張員, divided by 1.35. Accuracy 1.07.
- It is two hits of her own on surface ships, chosen as night targets are (searchlight
  redraws, かばう), in one row: `api_sp_list` 200, her place in `api_at_list`, `api_si_list` the
  瑞雲 first (the client draws its first slot's aircraft without checking), a second 瑞雲 and the
  radar where the type has them, then main guns to three pieces: `CutinZuiunNightCanvas` reads
  the name plate of three without a check, and a page error stopped the battle when two came
  (found by the headless `night_zuiun`). Which pieces the game itself shows past the 瑞雲 has
  not been seen. The multiplier goes in before the cap; a miss scratches.
- Once it is made, her fleet counts as having fired a star shell for the rest of the night,
  the cut-in's own hits included (the source's `star1 = true`): +4 to the chance and +5 to the
  accuracy of every later friendly attack, 10 off the enemy's chance. That is the source's
  behaviour; nothing read says whether the game does it. `api_flare_pos` is not changed.
- The flagship tries the fleet special attacks first; the 瑞雲 cut-in only if none comes up.
- Played in the client (`night_zuiun`, 2026-10-15): all three shapes it can show, without a
  page error. That proves the packet plays, not that the game lights her fleet afterwards.

## The order of the rolls

Seeded battles depend on it, and the tests in `attack_type_tests` pin it with an RNG that
hands out a given list of rolls and panics on one more.

1. As the night begins, `roll_star_shells`: in each fleet, friendly first, every ship afloat
   with a star shell and more than 4 HP rolls 70% until one fires. A fleet without one rolls
   nothing. Who fired goes out as `api_flare_pos`.
2. Per attack, after the target is chosen, for each of her types: a destroyer of level 80 or
   more first rolls whether the cut-in is the two-hit one (and is then rolled for with that
   row's chance), then the type is rolled for. Against an installation a destroyer cut-in
   without a main gun and a torpedo cut-in are skipped without a roll, except that a ship
   with two guns double attacks in the torpedo cut-in's place.

A ship whose list is empty rolls nothing, so a battle of such ships is what it was before.

## What the client is told

`api_sp_list` carries the type and `api_df_list` / `api_damage` one entry per hit. The mixed
cut-in (2) is two hits shown as one sum and takes two pieces of equipment, the main gun and
then the torpedo: `CutinSpSR` draws the first in the middle and the second above and below.
Both get a name plate, as every gun and torpedo has one (`battle-display-name-plates.md`).
`api_flare_pos` is a position in the fleet, from 6 for a combined fleet's escort.
The torpedo cut-in (3) takes two as well, and `CutinSpRR` reads both without a check: a
submarine who makes it with one late-model torpedo and her radar shows the radar second.

## Who attacks

`targeting.rs` `can_attack_night_ship` is the source's `canNB`: any ship with more than a
quarter of her hull, a submarine included (`Ship.canNB`; `SS` has no other), and a carrier as
`carrier-night-air-attack.md` has it. A ship at a quarter or less makes no attack, whichever
fleet she is in, so the 18 of moderate damage is never read for her. A submarine cannot attack
a submarine (`canASWNight`), so she takes a surface ship or nothing; the client plays her plain
attack as a torpedo whatever equipment is named (`_getNormalAttackType`). `retreated` has no
counterpart here.

## The Kongou class's night assault

`api_sp_list` 104, in `night.rs` beside the hougeki it is part of (`kongou_night_assault`).
In the friendly flagship's turn, if the fleet can make it and it comes up, one row carries her
hit and then her partner's, each on a surface ship of her own choosing, in place of the
flagship's own attack; the partner still takes her turn. The enemy never makes it.

- **Who:** 金剛改二丙, 比叡改二丙, 榛名改二乙/丙 or 霧島改二丙 as flagship with one of her
  partners second (`KONGOU_ASSAULT_SHIPS`; wikiwiki's table and the source's
  `canSpecialAttackUnique` agree), neither heavily damaged, five surface ships afloat, and
  単縦陣, 梯形陣 or 第二 / 第四警戒航行序列. A combined fleet's night is its second fleet's, so
  the flagship is that fleet's.
- **How often:** wikiwiki's `floor(3.5(√Lv + √Lv) + 1.1(√luck + √luck) + A + B - 33)` percent,
  A and B for a surface radar of 8 line of sight or more and a large searchlight on the
  flagship, by which ship she is (榛名改二乙 15 and 丙 20 for the radar: the English wiki's
  table has the two the other way round). The source has no rate for it. Nothing is rolled unless the
  fleet can make it and a surface ship is left to attack.
- **How hard:** 2.6 for the flagship and 2.7 for her partner before the cap, 1.06 more since
  the update of 2026-04-23 (2.756 and 2.862; the source rounds them to 2.76 / 2.86), 1.25 at
  T advantage and 0.8 at T disadvantage, and her own 35.6cm guns (改四 / 改三丙: 1.11 for
  one, 1.15 for more; 改二 / 改三(ダズル迷彩仕様): 1.05, 1.08). 1.4 to accuracy. The source's
  second gun id for the lesser pair, 529, is a destroyer's gun and read as 329.
- **The rate is the one from before 2026-04-23.** That update raised it "a little" and nobody
  has published a formula since (plan `2026-10-13-001`).
- **The client** (`_kongo_cutin`, `PhaseKongoAttack`) takes the two attackers from the fleet's
  first two places, not from the row, reads no equipment, and plays up to three targets. Its
  cut-in speaks one of the flagship's voices 900 and 990 to 993 by the pair; the cache list
  names them (`voice.rs` `SPECIAL`).
- **How many:** three a sortie, counted apart from the special gunnery attacks
  (`SpecialAttacksMade::kongou_assaults`, kept by the sortie and read off each night's
  `api_sp_list`), and none in an exercise. A night gives the flagship one turn, so it is one a
  battle at most.
- **What it costs:** the two pay 1.1 times the battle's ammunition (`kongouSpecialBuff5`;
  1.2 before 2026-04-23, which is what wikiwiki's page still says). None is made
  against PT boats alone.

## Attack power and night contact

A ship attacks at night with firepower plus torpedo plus her equipment's improvement, with
none of the day's 5 (`night_gun_power`). As the night begins each fleet puts its night scouts
up (`roll_night_contact`, the source's `getNightEquipsScout`): each comes into contact at
`floor(√Lv × √LoS) / 25`, and the one with the most accuracy of those that do is the
fleet's. A fleet that lost the air by day flies none, nor does one that met the enemy on even
terms, unless the battle opens at night. The scout in contact goes into `api_touch_plane` and
gives every night attack of its fleet, by its own accuracy of 1, 2 or 3: 5, 7 or 9 attack
power before the cap, 1.1, 1.15 or 1.2 on the base accuracy of 69, and a critical factor of
1.57, 1.64 or 1.7 in place of 1.5 (`NightContact`). The client shows the scout on the other
fleet's radar; the headless scenario `night_contact` plays ours. In a sortie the enemy's
stays -1: no abyssal equipment has the night scout's icon (`api_type[3]` 50). An exercise's
opponent can carry one: `practice_night_contact` plays a rival's 九八式夜偵 on our radar, with
触接中! under it (three runs on 2026-10-15, contact in two, without a page error).

## A hit that misses

A plain attack that misses does nothing and is written `0` in `api_cl_list`. A hit of a
double attack, a cut-in or a special attack that misses still scratches and is written `1`
(`cut_in_hit`; the source's `shell` and `NBattack`), by day and at night.

## Known gaps

- The flagship's special gunnery attacks of the day are rolled for at night too
  (`special_gunnery_at_night`), in the same one row as the night assault
  (`special_attack_row`); `day-shelling-order.md` has their rules.
- Smoke (`smokeType`) is not modelled at all, by day or at night, so neither is its effect on
  the attack on a submarine.
- A night attack on an installation leaves the torpedo out; its equipment multipliers are in
  `anti-installation.md`, with what of them is still missing.
- The destroyer cut-ins with a lookout show any lookout and any drum-type equipment.
- Played in the client (`night_cut_in`): types 2, 3, 5 and 7 and the star shell; the
  submarine's torpedo cut-in with her radar and her plain attack (`night_asw`); the Kongou
  class's night assault (`kongou_night_assault`); 101 at night, once and with one hit
  (`nagato_broadside`); 4 and 7 to 14 (`night_destroyer_cut_in`, 2026-10-15), without a page
  error. The client reads three pieces for each (`getSlotitem(0..2)`, -1 past the row's end):
  4's `CutinSpSSF` reads their `mstID` unchecked, so it must get three, and does (two main guns
  and a secondary are what make it); the destroyer cut-ins' `CutinSpSRD`, `RDJ`, `Suirai` and
  `Yusou` (11 to 14 use the same) check for a missing piece and would draw it blank. All of them
  get three here. The client lays the pieces out its own way: 8 sends torpedo, lookout, radar
  and shows torpedo, radar, lookout.
