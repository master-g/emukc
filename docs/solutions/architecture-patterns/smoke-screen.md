---
title: "Smoke screens: wikiwiki's rates, the source's estimated effects"
date: 2026-10-19
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [battle, smoke, accuracy, sortie]
applies_when:
  - "Changing when a fleet lays a smoke screen or what it does"
  - "Touching api_smoke_flag, api_smoke_type or api_limit_state"
---

# Smoke screens: wikiwiki's rates, the source's estimated effects

The player asks for 煙幕 on the formation screen; the client sends `api_smoke_flag=1` with the
day battle (`api_req_sortie/battle` and its siblings, the combined ones and `ec_battle`). The
handlers turn it into a `SortieOrder { formation_id, smoke }`; a bare formation id converts to one
with no smoke, so every older caller still compiles. `sortie_battle_impl` asks the battle for
smoke only when the sortie has not laid one yet (`ActiveSortieState::smoke_used`); a laid screen
sets it, a failed roll does not (wikiwiki: 失敗した場合は以降の戦闘で再び展開指示が可能). `next`
then answers `api_limit_state: 1`, which hides the button (`CellTaskBattle._isEnabledSmoke`; the
client also hides it with no smoke generator on a ship not sent home, and at night-start cells).

## The roll (`smoke.rs`)

Where the source rolls it (`getSmokeType`, `kcsim.js:3635`): after the engagement, before any
phase, once, and only when some friendly ship afloat carries 発煙装置 (500) or 発煙装置改 (501)
and, for a single fleet, four ships are afloat (a combined fleet rolls over both decks with no
such gate). Otherwise nothing is drawn, which keeps every seeded battle without smoke as it was.

The source ships with `smokeChance: [0,0,0]` and every `smokeMod*` at 1, so on its defaults smoke
never happens. The user chose (2026-10-19) wikiwiki 「発煙装置(煙幕)」's 発動率推定式 for the rates
and the source's `SIMCONSTS.smokeEst` for the effects; both are estimates.

- `k = ⌈√(flagship's luck) + 0.3 Σ★(500) + 0.5 Σ★(501)⌉`, `n = #500 + 2 × #501`, over the ships
  afloat; the flagship is the first ship (deck 1's in a combined fleet).
- `Pg0 = clamp(3.2 − 0.2k − n, 0, 1)`, and 0 when luck ≥ 1 and n ≥ 3.
- n = 1: `Pg1 = 1 − Pg0`. n = 2: `Pg2 = (1 − Pg0) × 0.05 × (k + 2)`, `Pg1` the rest.
  n ≥ 3: `Pg3 = 0.04k + 0.15(n − 3)` (the page's coefficient is "0.04–0.045?"; 0.04 taken),
  `Pg2 = min(0.3, 1 − Pg3)`, `Pg1` the rest.
- One draw `r`: under `Pg3` is 3, under `Pg3 + Pg2` 2, under all three 1, else none.

The page also has a 多重率 table (layers given a screen is laid). It overlaps the Pg table, which
is already a full distribution, so it is not used.

## What it does

Every friendly ship carries the level (`BattleRuntimeShip::smoke`), and an attack is smoked when
either ship is in a screen (the source's `ship.fleet.smokeType || target.fleet.smokeType`). The
accuracy is multiplied after everything else and before a PT imp's correction (`hit_value`):

| Attack | Player's | Enemy's |
| --- | --- | --- |
| Shelling, no radar | 0.01 | 0.5 |
| Shelling, radar (12, 13, 93) | 0.35, 0.25, 0.25 | 0.9, 0.83, 0.75 |
| Torpedo | 0.45, 0.42, 0.42 | 0.7, 0.6, 0.5 |
| ASW | 0.25 | 1 |

(by level 1, 2, 3 where they differ). Air attacks are 1 in the source and left alone; night
attacks are not smoked (`NBattack` has no term), but the night battle of the same cell keeps the
level on the ships, so an attack on a submarine that night is, as in the source. The player's
shells at 0.01 fall to the 10 % floor.

A fleet in its screen fires no anti-air but the fixed shot (`getAAShotProp`, `getAAShotFlat`
zero their terms); the two half-chance draws are still made.

`api_smoke_type` is the level in the day packet; the client draws one to three layers of smoke in
front of the fleet (`common.smoke_type`). The night packet sends none.

## Not modelled

The rocket barrage's chance in smoke (`rocketBarrageChance(0)`): no rocket barrage is modelled.
Radar-fire cells: the client offers no button there and the server does not check.

Played in the client: `make headless-check SCENARIO=smoke_screen` (plan 2026-10-19-009).
