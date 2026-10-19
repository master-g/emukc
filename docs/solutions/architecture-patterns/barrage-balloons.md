---
title: "Barrage balloons: four regular cells, the source's day terms"
date: 2026-10-19
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: low
tags: [battle, balloon, sortie]
applies_when:
  - "Changing which cells raise barrage balloons or what they do"
  - "Touching api_balloon_cell"
---

# Barrage balloons: four regular cells, the source's day terms

The game decides cell by cell whether balloons rise (wikiwiki 「阻塞気球」: マス毎に浮くかどうかの設定
が個別に決められている). Of the regular maps they rise at 3-5 H, 4-3 N, 4-5 T and 6-4 N, and not at
7-5 Q. The source's `BARRAGE_BALLOON_NODES` (`kancolle-replay` `69097abc`,
`js/simulator-ui/common.js`) lists the first three and its event cells; wikiwiki adds 6-4 N (read
2026-10-19). `setup.rs` keeps the four as `BALLOON_CELLS` by map id and node label, and the battle
sends `api_balloon_cell: 1` there by day; the client then draws the balloons of the ships that
carry one. The night packet sends 0, as the client draws none at night. No regular map's enemy
carries 深海観測阻塞気球 (1640) in the KCNav compositions (checked 2026-10-19), so only the
player's 阻塞気球 (513) fly in practice.

At a balloon cell every ship counts the ships of her fleet afloat that carry a balloon, both decks
of a combined one, at most 3 (`Fleet.getNumBalloons`; `BattleState::raise_balloons`). With `n`
her fleet's and `m` the other's, under the source's defaults:

- Day shelling: ×(1 + n/50) after the cap, with a cut-in's multiplier for a cut-in or double
  attack (`postModCI`), else with the post-cap multipliers after an AP shell's (`postModExtra`).
- Day ASW, the opening's too: ×(1 + n/50) after the cap (`ASW`, `!isNightActual`). Night attacks
  take nothing; the count is cleared when the night battle begins.
- A carrier's airstrike and an air corps' strike: the contact multiplier ×(1 + n/50) × (1 − m/20)
  (`AADefenceBombersAndAirstrike`, `LBASPhase`), `n` the striking side's, `m` the target's.
- The accuracy terms (`balloonSelf*`, `balloonOppo*`) are 1 and 0 by default and are left out.

The count is taken as the battle opens; the source counts the ships afloat at each attack, which
differs only when a balloon ship sinks mid-battle (#198, plan 2026-10-19-011).
