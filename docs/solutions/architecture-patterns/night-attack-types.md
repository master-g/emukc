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

## Known gaps

- A cut-in or double attack that misses does 0 here; the source gives it scratch damage.
- A star shell's +5 to accuracy, and a searchlight's effects on who is fired at.
- The night Zuiun cut-in and the fleet special attacks are other paths in the source.
- A submarine making her torpedo cut-in from one late-model torpedo and her radar has one
  torpedo to show; whether the client wants the radar named was not looked at.
- Who attacks at night is `can_attack_night_ship`, and it is not the source's `canNB`: a
  submarine never attacks here, so her torpedo cut-in is reached by the unit tests only, and
  a heavily damaged ship that is not a carrier still attacks (without the 18 of moderate
  damage) where the source keeps her out.
- The destroyer cut-ins with a lookout show any lookout and any drum-type equipment.
- Played in the client (`night_cut_in`): types 2, 5 and 7 and the star shell. Not played: 3, 4,
  8 to 14, the submarine's.
