---
title: "Who acts when: shelling, opening ASW, night battle and torpedoes"
date: 2026-10-10
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
tags: [battle, shelling, order, range, night, torpedo, asw, combined-fleet]
applies_when:
  - "A day shelling round is added, reordered or given a different set of ships"
  - "A phase is found where one whole side acts before the other"
  - "A seeded battle test or a golden transcript fails after a change to who fires when"
---

# Who acts when

## What was wrong

A single fleet's first round held one side's attacks and its second round the other's, and the
second round exists only when a battleship was present at the start. A fleet of destroyers
therefore never came under shellfire by day: over thirty seeds of six destroyers against 2-1
the enemy shelled 0 times and the fleet 180. Which side had the first round was decided by
fleet speed, a rule no source has. The combined-fleet rounds held both sides, but one whole
side fired before the other.

## What it is now

`shelling::simulate_shelling_round` fights one round. `ShellingRound` says which friendly
ships fire (`friendly_deck`), which enemy ships fire (`enemy_deck`), which enemy ships the
friendly side fires at (`enemy_targets`), and whether the order is by range. The enemy
fires at `friendly_deck`: the whole of a single fleet, and the deck whose round it is of a
combined fleet (`kcsimcombined.js` `69097abc`, `shellRange`/`shellOrder` pass that deck's
`alive1f`; plan `docs/plans/2026-10-18-003`). The two enemy ranges are the same except in the
battleship round against an enemy combined fleet: there the main deck fires and both decks
are fired at (`kcsimcombined.js` `69097abc`, single vs enemy combined, shelling 3: `order2`
from `ships2`, targets `alive2` and `alive2C`; plan `docs/plans/2026-10-18-002`).

- The two orders are built when the round begins, from the ships that can shell then.
- They are walked together: friendly first, then the enemy's ship of the same turn, and so on.
  A ship sunk before its turn loses it; the round ends when one side has nobody left.
- A first round is ordered by range, longest first, ships of one range in random order
  (`BattleRng`, a shuffle followed by a stable sort). A ship's range is its own or that of the
  longest piece it carries: `api_leng` on the ship does not include equipment.
- A second round goes down the line.
- The flagship's special attack is decided when the flagship's turn comes and takes that
  turn only: the ships that join it still fire on their own (below).
- Every index written is a position in the whole fleet (`append_turn` lifts them), so the
  combined paths need no shifting afterwards.

`simulation/mod.rs` only decides which round is which: `execute_shelling` for a single fleet,
and the two combined orchestrators, where a deck's second consecutive round goes down the
line and the enemy combined fleet's third round does too. Against an enemy combined fleet
the rounds are escort deck against the friendly fleet, main deck against it, then the main
deck firing while the friendly fleet fires at both decks.

Source: `KC3Kai/kancolle-replay` `kcsim.js`: `shellPhase`, `shellPhaseC`, `orderByRangeOld`,
and the two shelling blocks of `sim`. The plan `docs/plans/2026-10-10-001` lists the lines.

## The other phases

The same fault was in three more places, each letting the whole friendly side act before the
enemy did (plan `docs/plans/2026-10-10-002`):

- **Opening ASW** (`asw.rs`) uses the shelling loop: the ships of each side that can open with
  ASW, by range (`shelling::firing_order` takes the condition), in turn. Against a friendly
  combined fleet the enemy fires at the escort fleet alone (`shellPhase(..., alive1C, ...)`,
  `kcsimcombined.js` `69097abc` `:207-224`, plan `docs/plans/2026-10-18-004`).
- **Night battle** (`night.rs`) goes down the two lines in turn: the ship at position `i` of
  the friendly fleet, then the enemy's, each checked when its turn comes. `night_turn` is the
  one attack both sides share.
- **Torpedoes** (`torpedo.rs`) are in the water together. The friendly salvo is still resolved
  first, so the random stream keeps its order, but what the enemy fires is settled from a copy
  of the enemy taken when the phase began: a ship the friendly salvo sinks has fired already.
  Neither torpedo is aimed at a land target (`target_candidates`; `torpedoPhase` leaves
  `isInstall` out of both sides' targets, `kancolle-replay` `69097abc`, plan
  `docs/plans/2026-10-18-001`), so a deck of land targets alone counts as empty for the
  closing torpedo's deck draw below. The enemy's opening and closing torpedoes at a friendly
  combined fleet are aimed at the escort fleet alone (`torpedoPhase(alive1C, ...)`, `:228-231`,
  `:298`, `:330`; plans `docs/plans/2026-10-18-003` and `-004`).

## Flagships

Each friendly fleet's flagship, the escort fleet's included, is kept from sinking in every
phase, as in the source (`kcships.js` `69097abc` `:45`, `:1825`). `BattleState::from_context`
marks the first ship of the fleet and of the escort deck (`is_flagship`) and `take_hit`
reads the mark, so it no longer matters which slice a phase fires at: before, a flagship was
the ship at index 0 of the slice, and the enemy's airstrike, flown at the whole combined
fleet, could sink the escort flagship (plan `docs/plans/2026-10-18-005`). A test that builds
its ships by hand marks its flagship itself.

## Whose air state

`AirState` is worked out from the friendly side. An enemy attack reads it reversed
(`AirState::reversed`): the enemy spots for its guns by day, and its night recon counts, when
the sky is the enemy's. Day shelling and the night battle both pass it that way.

## The flagship's special gunnery attacks

`special_attack.rs`, against `kancolle-replay` `69097abc` (`canSpecialAttackUnique`,
`getSpecialAttackShips`, `getSpecialAttackMod`, `updateSupply`) and wikiwiki's 発動条件.

- **Who** (`check_*`): Nelson or Rodney with the third and fifth ship, neither a carrier nor
  a submarine (100); 長門改二 (101) or 陸奥改二 (102) with a battleship second; Colorado or
  Maryland with battleships second and third (103); two of Richelieu改, Richelieu Deux and
  Jean Bart改 (105); Warspite改 and Valiant改 (106). The flagship more than half her hull, the
  ships named beside her more than a quarter, the two of a Nelson Touch in any state.
- **When** (`try_special_attack`): six ships afloat that are not submarines, 梯形陣 (4) for
  101, 102, 103 and 106, 複縦陣 (2) for 100 and 105, 第二警戒航行序列 (12) for all; not in an
  exercise, not by 第2艦隊's flagship, not against PT boats alone (`is_pt_only`). It is rolled
  for in each turn of the flagship's, by day and at night, until the sortie has made one:
  `SpecialAttacksMade::gunnery`, kept in `ActiveSortieState`, handed to every battle and
  taken in from the packet afterwards (`BattlePacket::special_attacks`).
- **What** (`ResolvedSpecialAttack::hits`): the flagship twice and her partner once for a
  pair, a hit each for three ships, on surface ships only. The multiplier is after the cap by
  day and before it at night. Accuracy 1.05 for a Nelson Touch, 1.4 for the others, 1.15 more
  for an AP shell and for a surface radar.
- **One row**: the client reads all three hits from one row of the flagship's
  (`PhaseHougeki._nelson_touch` and the others, `d_indexes[0..3]`), by day and at night.
- **What a battle costs** (`battle_supply_cost`, plan `2026-10-13-003`): a fifth of the
  fuel and of the ammunition, never less than 1; half as much ammunition again, rounded up,
  for a night after the day; a tenth of each where the battle opens at night. Against a
  combined fleet the night costs no more, and takes a special attack's cost away again.
  Some cells cost otherwise (`cell_supply_cost`, plan `2026-10-13-004`), in percent of fuel
  and ammunition: PT boats alone 4 and 8, an air raid 6 and 4 (4 and 8 on 6-4 and 6-5),
  radar fire 4 and 0, submarines alone 8 and 0 unless at a boss or at 4-1 D and 4-3 C.
- **The ammunition modifier** (`ammo_modifier`, `resolve_damage`): below half her
  ammunition a ship's damage, not her power, is multiplied by the percentage left over 50;
  what comes to nothing is a scratch. Shelling, torpedoes, night attacks, attacks on
  submarines and carrier air strikes all take it.
- **Ammunition** (`special_attack_ammo`): the flagship and second ship of 101, 102 and 105
  and the three ships of 103 pay half as much again for the battle, the two of the night
  assault a tenth more. 106 is made only while both of its ships have ammunition left
  (`check_qe_attack`). Evidence in plan `2026-10-12-010`.
- **Unsettled:** whether 106 costs ammunition (the source charges nothing, wikiwiki says
  nothing) and whether the other attacks also need ammunition left (KC3Kai asks it of every
  one of them, on the strength of the same one measurement). The factor is on the whole
  of the battle's ammunition and rounded down, as the source has it; wikiwiki rounds the
  Nagato class's up against a single fleet (長門改二 by day: 68 there, 67 here). 105
  and 106 take the Nagato class's rate, having none of their own on record.
- **How often** (`trigger_rate`, plan `2026-10-13-002`): the formulas fitted to TsunDB's
  samples in 2024-08. 100: `floor(1.1√Lv1 + 1.4√luck1 + √Lv3 + √Lv5) + 25` percent. 103: the
  same over her second and third ships, and 5 more with an SG レーダー(後期型) on one of the
  three. 101, 102, 105, 106: `floor(√Lv1 + √Lv2 + 1.5(√luck1 + √luck2)) + 25`.
- Played in the client (`nagato_broadside`): 101 by day and at night.

## Submarines and the two decks of an enemy combined fleet

Plan `docs/plans/2026-10-17-005`, against `kancolle-replay` `69097abc`.

- **A submarine shells** (`can_shell_day_ship`, `SS.canShell`) when she carries WG42 or a
  特型内火艇, and then only at a land target (`target_candidates`, `shellPhaseTarget`). The
  source also keeps her out of the order when the other side has no land target; here she is
  in it and finds nothing to fire at, which differs only in the random draws (the order's
  shuffle, and her deck roll against an enemy combined fleet).
- **Which deck** (`deck_candidates`): when the ships fired at hold both decks of an enemy
  combined fleet (the third round of `simulate_day_enemy_combined`), a shell goes for the
  escort deck first on a roll under 0.39 and the main deck first otherwise, and for the other
  deck when the first has nothing to fire at (`doShellC`). A closing torpedo goes for the main
  deck on a roll under 0.35 and the escort deck otherwise, with no roll when one deck has
  nothing (`torpedoPhase` with `combinedAll`). The candidates are found within the deck, so a
  ship that would rather attack a submarine looks for one there. The opening torpedo and a
  special attack's hits draw from both decks.

## Not modelled

- Star shells, searchlights and night contact; the order of the decks in a combined night
  battle.

## When a test breaks

The order draws random numbers, so every day transcript moved with this change. Re-bless
`crates/emukc_battle/tests/golden/*.txt` with `EMUKC_BLESS_GOLDEN=1`, re-freeze
`tests/gameplay_tests/battle_golden.rs` (the second change left it as it was), and move the seed of a seeded test rather than its
assertion (`DROP_SEED` in `crates/emukc_gameplay/tests/sortie_battle.rs` went from 1 to 3).
