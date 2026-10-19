---
title: "Hit and critical rolls: one draw per attack, shared by every battle phase"
date: 2026-10-09
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [battle, accuracy, evasion, critical, formation, morale, kouku]
applies_when:
  - "A battle phase starts or stops rolling for a hit"
  - "An accuracy or evasion correction is added (proficiency, gun fit, combined fleet)"
  - "A cut-in or special attack is added and needs its own accuracy multiplier"
  - "A seeded battle test or a golden transcript starts failing after a battle change"
---

# Hit and critical rolls

## What it is

Every attack in `emukc_battle` is rolled once in `accuracy.rs` and comes out as
`HitOutcome::{Miss, Hit, Critical}`. The phase passes that outcome to the damage function and
writes `outcome.cl()` (0, 1, 2) where the client expects it.

| Phase | Roll | Where `cl` goes |
|---|---|---|
| Day shelling, special attacks | `roll_attack` `Shelling` | `api_cl_list` |
| Day ASW, opening ASW | `roll_attack` `Asw` | `api_cl_list` |
| Opening torpedo | `roll_attack` `Torpedo` | `api_fcl_list_items` / `api_ecl_list_items` |
| Closing torpedo | `roll_attack` `Torpedo` | `api_fcl` / `api_ecl` |
| Night battle | `roll_attack` `Night` | `api_cl_list` |
| Carrier air phase | `roll_strike` at 95 | `api_fcl_flag` / `api_ecl_flag`, set by a critical |
| Air corps attack | `roll_strike` at `90 + 7 x 命中`, evasion x0.86 (x0.68 on a combined fleet) | nothing |
| Raid on the air base | `roll_strike_on_base` at 95 | nothing |

## The formula

Transcribed from `KC3Kai/kancolle-replay` `kcsim.js` (`hitRate`, `accuracyAndCrit`, `rollHit`,
`formationCountered`, the formation table at the top) and `kcships.js` (`moraleMod`,
`moraleModEv`). The plan `docs/plans/2026-10-09-004` lists each with its line.

- Attacker: `(base + 2 sqrt(level) + 1.5 sqrt(運) + equipment 命中 + phase extra) x morale x formation x cut-in`.
  Bases are 90 shelling, 85 torpedo, 80 ASW, 69 night. The torpedo's extra is a fifth of its
  capped attack power; against submarines only sonar counts, at twice its 対潜. Improved (★)
  equipment adds its accuracy for the attack (the torpedo's rounded down) to the flat terms
  (`equipment-improvement-bonus.md`).
- Gun fit (`gun_fit.rs`, #193): `getFit` from `kcships.js` at `69097abc`, rules per ship type and
  class over the guns' ids, for both sides. Day shelling adds it after `hitRate`'s multipliers
  and before a cut-in's (`accMod2`, `Aim::day_cut_in`); a night attack adds it after every
  multiplier, like the searchlight's 7. A gun the pinned source does not list has no fit until
  the pin moves.
- AP shell (`armour_piercing`, #196): by day an AP shell with a main gun multiplies `accMod2`
  by 1.1 to 1.3 (more with a radar or a secondary gun) against a CA, CAV, battleship or
  standard carrier, not a light carrier; normal shelling and special attacks alike.
- Target: `floor((回避 + sqrt(2 x 運)) x formation)`, bent at 40 and 65, less the points its
  fuel is short of 75%. Her improved engines (against shelling) and sonars (against torpedoes)
  add their evasion after the curve, and the sum is rounded down.
- Chance: `max(attacker - target, 10) x target morale`, floored, at most 96. The attacker's
  and target's terms are rounded to four places before they are floored (`floor_percent`, #244),
  so `100 x 1.15` counts 115, not 114.
- One draw in 0..=99: at or under `floor(sqrt(chance) x factor)` is a critical, at or under the
  chance is a hit. The factor is 1.3 for shelling and ASW, 1.5 for torpedo and night, 0 for
  aircraft, whose criticals come from proficiency alone.
- A critical multiplies the attack power by 1.5 and floors it **after the cap and before the
  armour**; a miss sends 0 and deals 0. Scratch damage still belongs to a hit that cannot
  pierce.

A formation that is countered (複縦 against 単横, 梯形 against 単縦, 単横 against 梯形) gives
shelling and ASW no accuracy multiplier; torpedo and night attacks always take theirs.

## 警戒陣

A fleet in 警戒陣 (formation 6) fights in two halves (#195, plan `2026-10-17-004`, `kcships.js`
`getFormation`, `kcsim.js` `VANGUARD1`/`VANGUARD2`, `vanguardUseType` 2). Each ship carries
her place (`BattleRuntimeShip::vanguard`, set by `BattleState::from_context`): her number from
1 and the fleet's length; one sunk or sent home keeps hers. The front half is the first
`floor(n / 2)`: 3 of 6 or 7, 2 of 5.

- Attacker's accuracy by her own half: front shelling 0.8, torpedo 0.7, night 0.8, ASW 1;
  rear 1.2, 0.9, 1.2, 1.1. Neither half dodges by its formation.
- Attacker's power: the front half shells at 0.5 and attacks at night at half power before the
  cap; the rear half attacks submarines by day at 0.6; torpedoes are 1 in both.
- Target's place: when the target's fleet is in 警戒陣 the attacker's accuracy is multiplied
  by her place's figure (`vanguard_dodge`), countered or not. Shelling, night and ASW: a
  destroyer `.95 .95 .8 .8 .69 .64 .64`, any other ship `.95 .95 .95 .95 .86 .79 .7`;
  torpedoes: `.9 .9 .65 .58 .5 .42 .42` and `.9 .9 .77 .67 .63 .55 .51`.
- A fleet in 警戒陣 aims its torpedoes at a destroyer at 1.2 and its shelling and night
  attacks at 1.1, whichever half fires.
- `choice_with_protect` draws a target in the front half once more, after the searchlight
  rerolls and before かばう; the second draw stands.

The event-map rows (`*Event`) and the flat-evasion variant (`vanguardUseType` 1) are not used.

## Aircraft proficiency

`plane_proficiency` sums a ship's 熟練度 the way `kcships.js` `updateProficiencyBonus` does and
hands the roll three numbers. The level the server stores (`api_alv`, 0-7) stands for the
experience `0, 10, 25, 40, 55, 70, 85, 120`, and is worth `0, 1, 2, 3, 4, 5, 7, 10` towards a
critical.

- Accuracy: `sqrt(average experience / 10)` plus `1, 2, 3, 4, 6, 9` from an average of 25, 40,
  55, 70, 80, 100. It is added **after** the ceiling of 96, so skilled aircraft can pass it.
- Critical rate: the level's worth times 0.8 for the first piece of equipment on the ship, 0.6
  for any other, added to the threshold after the square root. An aerial strike has factor 0,
  so this is its whole critical rate: 8 in a hundred for one fully skilled first slot.
- Critical damage: `1 + floor(sqrt(experience) + worth) / 100` for the first piece, `/ 200` for
  the others. It multiplies the 1.5 first and the power second, as the source does: 1.2 for one
  fully skilled first slot, which makes 179 of a power of 100, not 180.

It counts torpedo, dive and seaplane bombers, jet fighter-bombers, flying boats and land
attackers, plus a patrol plane or autogyro that can bomb, one level lower. The sum is taken
per ship, not per slot, and goes to the carrier air phase (which then sets the target's
`api_fcl_flag` / `api_ecl_flag`), to the day shelling of a CV, CVL or CVB, to a carrier's night
air attack, and to the day attack on a submarine by the ships whose aircraft make it (not the
opening one); the last two are in `carrier-night-air-attack.md`. A ship without skilled
aircraft rolls exactly as before, on the same single draw.

A carrier's cut-in does not take the summed critical rate: the source cancels it and gives
`13 x average experience / 120` instead (`Aim::as_carrier_cut_in`). Its further terms for the
first slot's aircraft type and experience, and the cut-in's own critical damage, are not here.

## Aircraft: a miss has no marker

The client reads stage 3's `api_fcl_flag` / `api_ecl_flag` as the critical flag
(`AirWarStage3Model.getHitType` returns `flag + 1`). There is nothing that says "miss": the
client draws the run from `rai_flag` / `bak_flag` and the miss from the zero in `api_fdam`.
So the flags are set whenever a strike is flown at a target, whether or not it lands.

## Proficiency in fighter power

Proficiency also raises 制空値 (`simulation/kouku.rs` `proficiency_fighter_power`, the source's
`APbonus`). Each slot counts `floor(対空 x sqrt(count) + bonus)`, the bonus inside the floor:
`sqrt(exp x 0.1)` plus a step by level — `0,0,2,5,9,14,14,22` for fighters (carrier, seaplane,
land, jet), `0,0,1,1,1,3,3,6` for seaplane bombers, nothing more for torpedo, dive and jet
bombers. Any other aircraft gets nothing aboard a ship and the root alone from a land base. The
experience is the plain figure for the level; the 0.825 of a patrol plane belongs to the hit
roll only. How proficiency grows and falls is in `plane-losses-and-proficiency-growth.md`.

## Why it is shaped this way

- **One function for all phases.** The phases differ only in base, formation column and
  critical factor, and they must not drift apart.
- **The outcome is an argument of the damage function, not a wrapper around it**, because the
  critical multiplier sits between the cap and the armour, inside every `calculate_*_damage`.
- **One draw per attack.** Any change to how many draws an attack takes moves the whole random
  stream of the battle, and every seeded transcript with it.

## Not modelled

Each is a correction the source applies on top: the first-slot terms of the carrier cut-in;
the combined-fleet accuracy terms, star shells and night contact, smoke, balloons,
event bonuses. PT imps are in `pt-imps.md`.
A night attack on a submarine in a combined fleet after a day battle scratches where the source
does real damage (see `carrier-night-air-attack.md`, "A submarine at night"). Abyssal ships fight at
condition 49 with full fuel; an enemy built from the manifest alone has no evasion.

## When a test breaks

- A seeded test that asserts "it was hit" may now land on a miss. Change the seed; do not
  loosen the assertion. `WIN_RANK_SEED` in `crates/emukc_gameplay/tests/practice_battle.rs` and
  the seeds in `simulation/mod.rs` and `simulation/air_base.rs` were moved for this reason.
- `crates/emukc_battle/tests/golden/*.txt` is re-blessed with `EMUKC_BLESS_GOLDEN=1`;
  `tests/gameplay_tests/battle_golden.rs` is re-frozen by copying the new transcript in.
