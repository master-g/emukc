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
| Carrier air phase | `roll_strike` at 95 | nothing: see below |
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
  aircraft.
- A critical multiplies the attack power by 1.5 and floors it **after the cap and before the
  armour**; a miss sends 0 and deals 0. Scratch damage still belongs to a hit that cannot
  pierce.

A formation that is countered (複縦 against 単横, 梯形 against 単縦, 単横 against 梯形) gives
shelling and ASW no accuracy multiplier; torpedo and night attacks always take theirs.

## Aircraft: a miss has no marker

The client reads stage 3's `api_fcl_flag` / `api_ecl_flag` as the critical flag
(`AirWarStage3Model.getHitType` returns `flag + 1`). There is nothing that says "miss": the
client draws the run from `rai_flag` / `bak_flag` and the miss from the zero in `api_fdam`.
So the flags are set whenever a strike is flown at a target, whether or not it lands.

## Why it is shaped this way

- **One function for all phases.** The phases differ only in base, formation column and
  critical factor, and they must not drift apart.
- **The outcome is an argument of the damage function, not a wrapper around it**, because the
  critical multiplier sits between the cap and the armour, inside every `calculate_*_damage`.
- **One draw per attack.** Any change to how many draws an attack takes moves the whole random
  stream of the battle, and every seeded transcript with it.

## Not modelled

Each is a correction the source applies on top: aircraft proficiency (so no aerial strike is
ever a critical), gun fit, 改修, the combined-fleet accuracy terms, 警戒陣 by position (the rear
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
