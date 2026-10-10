---
title: "Day shelling order: both sides in every round, ship by ship"
date: 2026-10-10
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
tags: [battle, shelling, order, range, combined-fleet]
applies_when:
  - "A day shelling round is added, reordered or given a different set of ships"
  - "A phase is found where one whole side acts before the other"
  - "A seeded battle test or a golden transcript fails after a change to who fires when"
---

# Day shelling order

## What was wrong

A single fleet's first round held one side's attacks and its second round the other's, and the
second round exists only when a battleship was present at the start. A fleet of destroyers
therefore never came under shellfire by day: over thirty seeds of six destroyers against 2-1
the enemy shelled 0 times and the fleet 180. Which side had the first round was decided by
fleet speed, a rule no source has. The combined-fleet rounds held both sides, but one whole
side fired before the other.

## What it is now

`shelling::simulate_shelling_round` fights one round. `ShellingRound` says which friendly
ships fire (`friendly_deck`), which enemy ships fire and are fired at (`enemy_deck`), and
whether the order is by range.

- The two orders are built when the round begins, from the ships that can shell then.
- They are walked together: friendly first, then the enemy's ship of the same turn, and so on.
  A ship sunk before its turn loses it; the round ends when one side has nobody left.
- A first round is ordered by range, longest first, ships of one range in random order
  (`BattleRng`, a shuffle followed by a stable sort). A ship's range is its own or that of the
  longest piece it carries: `api_leng` on the ship does not include equipment.
- A second round goes down the line.
- The flagship's special attack is decided when the flagship's turn comes.
- Every index written is a position in the whole fleet (`append_turn` lifts them), so the
  combined paths need no shifting afterwards.

`simulation/mod.rs` only decides which round is which: `execute_shelling` for a single fleet,
and the two combined orchestrators, where a deck's second consecutive round goes down the
line and the enemy combined fleet's third round does too.

Source: `KC3Kai/kancolle-replay` `kcsim.js`: `shellPhase`, `shellPhaseC`, `orderByRangeOld`,
and the two shelling blocks of `sim`. The plan `docs/plans/2026-10-10-001` lists the lines.

## Not modelled

- A special attack can be rolled in each round and its participants lose the turns they have
  not taken yet; the source rolls it once a battle and lets them fire again.
- Submarines joining the order when the other side has an installation.
- The 39% split of a combined fleet's targets between main and escort.
- Opening ASW and the night battle still let one whole side act first, and torpedoes are
  resolved one side after the other.

## When a test breaks

The order draws random numbers, so every day transcript moved with this change. Re-bless
`crates/emukc_battle/tests/golden/*.txt` with `EMUKC_BLESS_GOLDEN=1`, re-freeze
`tests/gameplay_tests/battle_golden.rs`, and move the seed of a seeded test rather than its
assertion (`DROP_SEED` in `crates/emukc_gameplay/tests/sortie_battle.rs` went from 1 to 3).
