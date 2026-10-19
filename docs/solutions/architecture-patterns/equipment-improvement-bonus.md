---
title: "Equipment improvement (★): firepower, accuracy and evasion"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Modifying day shelling, carrier shelling, night, torpedo or ASW basic power"
  - "Adding or adjusting what equipment stars (★) add to attack power"
  - "Modifying the accuracy or evasion of day shelling, night, torpedo or ASW attacks"
  - "Modifying a carrier's or an air corps' airstrike power"
tags: [equipment, improvement, star-bonus, shelling, torpedo, night-battle, asw, damage-formula, accuracy, evasion]
related_components: [emukc_model]
---

# Equipment improvement (★): firepower, accuracy and evasion

## Context

What each star adds to attack power, accuracy and evasion follows `KC3Kai/kancolle-replay` at
`69097abc`:
`Equip.prototype.setImprovement` (`js/kcships.js:2387-2491`) over the per-type table
`EQTDATA[type].improve` (`js/kcEQDATA.js`). The earlier contract, with no cited source, gave
every gun, torpedo and bomber √★ by day and at night, torpedoes ★ × 1.2 and ASW nothing (#206).

Before #194 no hit roll read the stars at all.

The code is `Improvement`, `ship_improvement`, `improvement_coefficients` and `improvement` in
`crates/emukc_battle/src/damage.rs`. `improvement_coefficients` is the one per-type table, for
every key; the hit rolls read it in `accuracy.rs` (`improvement_accuracy`, `dodge`).

## Guidance

### Firepower

Every equipped item at ★ > 0 contributes to four firepower keys, summed over the ship. The
source keys them `Pshell` (day), `Pnb` (night), `Ptorp` (torpedo) and `Pasw` (ASW). They are
built in this order, a later step overwriting an earlier one:

1. **Aircraft, linear in ★:**
   - Dive, torpedo and jet bombers (client types 7, 8, 57) that are not fighter-bombers:
     day and ASW 0.2★. The fighter-bombers are the items the source marks `isFighterBomber`
     (60, 154, 219, 447, 487, generated into `FIGHTER_BOMBERS`); of them only 487 has ASW 0.2★.
   - Autogyro (25): ASW 0.3★ at 対潜 ≥ 11, else 0.2★. ASW patrol plane (26): the same at 8.
2. **The table, coefficient × √★, by client type `api_type[2]`:**

   | Client types | Day | Night | Torpedo | ASW |
   |---|---|---|---|---|
   | 1, 2, 4, 18, 19, 24, 29, 34, 35, 36, 37, 39, 42, 46, 52, 54 | 1 | 1 | — | — |
   | 3, 38 (large main guns) | 1.5 | 1 | — | — |
   | 5 (torpedo) | — | 1 | 1.2 | — |
   | 21 (AA gun) | 1 | — | 1.2 | — |
   | 22 (midget sub) | — | 1 | — | — |
   | 14, 15, 40 (sonars, depth charges) | 0.75 | — | — | 1 |

   The source types a few items apart from the client: 128, 281 and 465 as `MAINGUNXL` (the
   same weights as client type 3), 142 and 460 as `RADARXL` and 151 as `CARRIERSCOUT2` (no
   firepower keys either way), and 467 (below). No item carries its own `improve` override.
3. **After the table:**
   - A day weight of 1 on an item of 火力 ≥ 13 becomes 1.5√★; a day weight of 1.5 on one
     of 火力 < 13 becomes √★.
   - Secondary guns (client type 4) by id: 12, 234, 247, 463 → day and night 0.3★; any
     other but 11, 77, 130, 134, 135 → 0.2★; those five keep √★. The source's 0.3 list
     also names 467, but types it `SECGUNL`, so the rule never reaches it: it keeps √★.
   - Submarine torpedoes (32): torpedo and night 0.2★.
   - 226, 227, 488: day 0.

Where each key goes, as a flat term in basic power before the pre-cap modifiers:

- Day: day shelling (`day_gun_power`, the source's `Ship.shellPower`) and carrier shelling
  (`calculate_shelling_damage`, `CV.shellPower`), where it sits inside the source's
  `1.5 × (…)` and so counts 1.5 times.
- Night: `night_gun_power` (`NBPower`). A carrier's night air attack takes none of it; her
  aircraft's stars count there on their own.
- Torpedo: `torpedo_attack_power` (`kcsim.js:1851`). Torpedo accuracy takes a fifth of that
  power, so it moves with it.
- ASW: inside the bracket of `calculate_asw_damage`, before the synergy multiplier
  (`ASWPower`).

### Airstrikes

An aircraft's stars also add to her own figure (爆装, 雷装) in the carrier battle's strike and
an air corps' strike (`airstrikePowerImprove`, `kcships.js:2387-2428`; added at `kcsim.js:2019`
and `3277`), as `airstrike_improvement` in `damage.rs` (#264):

- Dive, torpedo and jet bombers (7, 8, 57) that are not fighter-bombers, and seaplane bombers
  (11): 0.2★. The fighter-bombers' stars go to their anti-air instead.
- Land attackers and large land attackers (47, 53): 0.7√★.

It goes in only when the figure is above zero and the target is no submarine: a torpedo bomber
against land, 東海 against a ship, and any strike on a submarine take none. In the carrier
strike it comes before the slot's 装備ボーナス. The fighter-bombers are the generated
`FIGHTER_BOMBERS` (`scripts/gen_installation_types.py`, the source's `isFighterBomber`); the
source types every aircraft as the client does.

### Accuracy and evasion

Six more keys, summed over the ship the same way, each `coefficient × √★` by client type:

| Client types | Day acc. (`ACCshell`) | Night acc. (`ACCnb`) | Torpedo acc. (`ACCtorp`) | ASW acc. (`ACCasw`) | Day ev. (`EVshell`) | Torpedo ev. (`EVtorp`) |
|---|---|---|---|---|---|---|
| 1, 2, 3, 4, 18, 19, 24, 29, 34, 35, 36, 37, 38, 39, 42, 46, 52, 54, and the radars 12, 13, 93 | 1 | 1.3 | — | — | — | — |
| 5 (torpedo) | — | 1.3 | 2 | — | — | — |
| 21 (AA gun) | — | — | 2 | — | — | — |
| 22 (midget sub) | — | 1.3 | — | — | — | — |
| 14, 40 (sonars) | — | — | — | 1.3 | — | 1.5 |
| 15 (depth charges) | — | — | — | 1.3 | — | — |
| 17 (engines) | — | — | — | — | 1.5 | — |

The first row is every type with a day firepower weight of 1 or 1.5 but the AA gun, plus the
radars. A radar of 命中 ≥ 3 takes 1.7√★ by day and 1.6√★ at night instead
(`setImprovement`). None of the items the source types apart from the client changes these.

Where they go (`kcsim.js`):

- Accuracy, into the flat terms of `hit_value` (`hitRate`), before morale, formation and cut-in
  multiply them: day shelling `ACCshell` (`shell`), night `ACCnb` (`NBattack`, a carrier's
  night air attack included), torpedo `floor(ACCtorp)` of the ship's sum (`torpedoPhase`),
  ASW `ACCasw` beside the sonars' term (`ASW`).
- Evasion, the target's, after the evasion curve and before rounding down (`accuracyAndCrit`'s
  `evFlat`): `EVshell` against day shelling, `EVtorp` against torpedoes. None at night or
  against ASW.

Equipment at ★0 adds nothing; abyssal equipment is always ★0.

## Why This Matters

The weights differ by attack and by item, and several are linear in ★ rather than √★: a
submarine torpedo ★10 adds 2 rather than 12, a sonar ★4 now adds 2 to ASW, and most secondary
guns gain far less than √★. The keys go in before the multipliers and compound with them.

## When to Apply

- When changing any of the four basic power formulas above, or the accuracy or evasion of
  those attacks.
- When a client equipment type is added, or the kancolle-replay pin moves: repeat the
  check below.

## Examples

- Torpedo ★10: torpedo 1.2√10 ≈ 3.79, night √10 ≈ 3.16.
- Submarine torpedo ★10: torpedo 2, night 2.
- Large main gun of 火力 15 at ★4: day 3, night 2. At 火力 12: day 2.
- Secondary gun 12 at ★10: day and night 3. 467 at ★9: 3.
- Sonar ★4: day 1.5, ASW 2.
- Non-fighter dive bomber ★5: day (carrier shelling) 1, ASW 1.
- AA gun ★4: day 2, torpedo 2.4, night 0; torpedo accuracy 4.
- Main gun ★10: day accuracy √10, night 1.3√10. Torpedo ★4: torpedo accuracy 4, night 2.6.
- Sonar ★9: ASW accuracy 3.9, torpedo evasion 4.5. Engine ★6: day evasion 1.5√6 ≈ 3.67.
- Radar ★4: day accuracy 3.4 and night 3.2 at 命中 ≥ 3; 2 and 2.6 below.
- 彗星 ★10 in a carrier strike: 爆装 8 + 2. 一式陸攻 ★4 in an air corps' strike: 雷装 10 + 1.4.

## Checking against the source

The table and special cases were compared once with the source for every item below id 1500
in `kcEQDATA.js` at the pin (584, all in the current start2), ★1 to ★10: run the source's own
`setImprovement` in Node over each `EQDATA` entry, and compare its four keys with
`improvement` on the codex's item. No differences: the four firepower keys (#206), and the
six accuracy and evasion keys (#194).

## Related

- `docs/solutions/architecture-patterns/battle-damage-foundation.md` — the damage formula
  these terms feed into.
- `docs/battle/damage-formula-reference.md` §3.5 — the wiki's table, which this replaces.
- `docs/solutions/architecture-patterns/battle-hit-and-critical-rolls.md` — the hit roll the
  accuracy and evasion keys feed into.
