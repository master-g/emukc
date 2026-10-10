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
  - "An accuracy or evasion correction is added (proficiency, gun fit, 改修, combined fleet)"
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
  capped attack power; against submarines only sonar counts, at twice its 対潜.
- Target: `floor((回避 + sqrt(2 x 運)) x formation)`, bent at 40 and 65, less the points its
  fuel is short of 75%.
- Chance: `max(attacker - target, 10) x target morale`, floored, at most 96.
- One draw in 0..=99: at or under `floor(sqrt(chance) x factor)` is a critical, at or under the
  chance is a hit. The factor is 1.3 for shelling and ASW, 1.5 for torpedo and night, 0 for
  aircraft, whose criticals come from proficiency alone.
- A critical multiplies the attack power by 1.5 and floors it **after the cap and before the
  armour**; a miss sends 0 and deals 0. Scratch damage still belongs to a hit that cannot
  pierce.

A formation that is countered (複縦 against 単横, 梯形 against 単縦, 単横 against 梯形) gives
shelling and ASW no accuracy multiplier; torpedo and night attacks always take theirs.

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
`api_fcl_flag` / `api_ecl_flag`) and to the day shelling of a CV, CVL or CVB. A ship without
skilled aircraft rolls exactly as before, on the same single draw.

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

Each is a correction the source applies on top: proficiency for the night air
attack and ASW flown by aircraft; the first-slot terms of the carrier cut-in; gun fit,
改修, the combined-fleet accuracy terms, 警戒陣 by position (the rear
half's row is used for the whole fleet), star shells and night contact, AP-shell accuracy,
the accuracy multipliers of flagship special attacks, smoke, balloons, PT imps, event bonuses.
A night attack on a submarine still always lands for scratch damage. Abyssal ships fight at
condition 49 with full fuel; an enemy built from the manifest alone has no evasion.

## When a test breaks

- A seeded test that asserts "it was hit" may now land on a miss. Change the seed; do not
  loosen the assertion. `WIN_RANK_SEED` in `crates/emukc_gameplay/tests/practice_battle.rs` and
  the seeds in `simulation/mod.rs` and `simulation/air_base.rs` were moved for this reason.
- `crates/emukc_battle/tests/golden/*.txt` is re-blessed with `EMUKC_BLESS_GOLDEN=1`;
  `tests/gameplay_tests/battle_golden.rs` is re-frozen by copying the new transcript in.
