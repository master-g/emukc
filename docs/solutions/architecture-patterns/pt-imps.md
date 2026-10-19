---
title: "PT imps: the source's corrections on every attack made on one"
date: 2026-10-19
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [battle, pt-imp, accuracy, targeting]
applies_when:
  - "Changing how an attack on a PT小鬼群 lands or what it does"
  - "Adding an attack kind that can be aimed at a PT imp"
---

# PT imps: the source's corrections on every attack made on one

A PT imp is `TargetClass::PtBoat` (`target_class`: PT小鬼群 1637–1640 and Schnellboot小鬼群
2192–2194, the source's `isPT` ships). They sail in 5-6, 7-3 and 7-5. Everything below is
`KC3Kai/kancolle-replay` at `69097abc` under its default switches (`NERFPTIMPS`, `BREAKPTIMPS`
off; `enableAirstrikeSpecialBonus` on), in `pt_imp.rs` (#197).

| Attack | Accuracy, in percent `x` before the target's evasion | Power after the cap and post-cap multipliers, before the critical |
| --- | --- | --- |
| Day shelling and its special attacks (`shell`) | `(0.3x + √x + 15) × 1.2 × ptAccMod`, × 1.2 from a fleet in 警戒陣 | `0.3p + √p + 10`, × `ptDmgMod` |
| Night (`NBattack`) | the same × 0.7 | `(0.3p + √p + 10) × 0.6`, × `ptDmgMod` |
| Torpedo (`torpedoPhase`) | `(0.3x + √x + 15) × 1.2 × 0.7` | `0.3p + √p + 10` |
| Carrier airstrike (`airstrike`) | unchanged | × 0.8 on a roll under 0.4, else × 0.5, rolled after the torpedo bomber's |
| Air corps (`airstrikeLBAS`) | × 0.95, × 0.85 for the B-25 | × 0.7 on a roll under 0.4, else × 0.4, rolled only on a hit |

`ptAccMod` and `ptDmgMod` (`kcships.js:967-996`) are `AntiPt::of`: a base of 1 for a destroyer or
escort, 0.82 for a light cruiser and 0.7 for anyone else (accuracy only), times a factor for each
of small main guns (one, two), secondary guns, machine guns (one, two), lookouts, dive bombers
(one, two; jet bombers count for power only), seaplane bombers or fighters, and AB boats or armed
daihatsu (one; two, or two 特大発 tanks). The source's equipment types map one to one onto
`api_type[2]` for every type these read (checked against `kcEQDATA.js`; its `MAINGUNSAA` has no
members), so the counts read start2.

天霧改二 and 天霧改二丁 (903, 908, `isAntiPTShip`) add 64 to the accuracy sum of their day and
night gun attacks on a PT imp, and a destroyer or escort beside one in the same deck adds 32
(`BattleRuntimeShip::pt_accuracy_flat`, set in `BattleState::from_context`). These ships fire at
the PT imps first by day and at night, and against an enemy combined fleet at the deck that alone
has any, after the deck roll is drawn.

The issue cited `kcsim.js:2108` and `:3352` for the strikes; under the default switches those
lines do not run, and the PT arms of the special-bonus chains (`:2058`, `:3303`) do. The carrier's
chain is `special_bonus` in `kouku.rs` (#200); the air corps' other arms are not modelled
(`anti-installation.md`).

## Not modelled

The skip bombers' bonus against destroyers, which leaves PT imps out (`:1987`, `:3205`): no skip
bomber is modelled. Night cover is drawn from every ship rather than the PT imps alone, which
differs only when the flagship is one.
