---
title: "LoS routing gates compared the raw LoS sum, so they always passed"
date: 2026-10-06
category: logic-errors
module: emukc_gameplay
problem_type: logic_error
component: service_object
symptoms:
  - Every 「索敵 N 以上」 branch was taken and every 「索敵 N 未満」 branch never was, on all 14 regular maps with LoS conditions
  - A fleet could not be turned away from a boss route for lack of LoS, whatever it carried
root_cause: logic_error
resolution_type: code_fix
severity: high
---

# LoS routing gates compared the raw LoS sum

## Symptom

183 routing conditions on 14 maps test the fleet's LoS. None of them could fail: 2-5's
boss gate asks for 49, and a real six-ship fleet was being compared at 390.

## Cause

Three layers, each hiding the next.

1. **The wrong quantity.** Thresholds are 判定式(33) scores. The rules arrived with
   `formula: null`, and `los_by_formula(None)` returned the sum of every ship's
   `api_sakuteki[0]`. A score is a few dozen; the sum is a few hundred.
2. **The formula on offer was not formula 33 either.** The "式3" branch multiplied all
   equipment by 0.6 and had no branch-point coefficient and no improvement bonus. No rule
   asked for it, so nobody saw.
3. **Whole-number thresholds against a fractional score.** 「28 未満 / 28 以上」 is stored as
   `Lte 27` / `Gte 28`. A score of 27.5 matches neither. This only appears once layer 1 is
   fixed.

It lasted because the fallback was pinned by a unit test as "backward-compatible":
`los_formula_none_uses_los_total` asserted that `formula: None` compares the raw sum. A
test that fixes a fallback in place says nothing about whether the fallback is right.

## Fix

- `FleetRouteContext` keeps the two halves of the score that do not depend on the map —
  `los_ship_term` (`Σ√own LoS − ⌈0.4 × HQ level⌉ + 2 × (6 − ships)`) and `los_equip_term`
  (`Σ equipment coefficient × (LoS + improvement coefficient × √★)`) — and
  `los_score(coefficient)` combines and floors them.
- `RoutePredicate::LoS` carries `coefficient`. Without one the predicate evaluates to
  `SourceUnknown`; there is no number a bare threshold can honestly be compared with.
- The score is floored before comparison, which makes `Lte N-1` mean 「N 未満」.
- The coefficient tables follow the compass simulator's `src/logic/seek/equip.ts`, which is
  maintained; the Fandom 検証Wiki table most write-ups cite stopped in 2018.

## Verification

- `fleet_los_terms_follow_formula_33` builds a fleet in the database and checks both terms
  against hand-computed values.
- The two fleets of the 2026-09-22 live snapshot were scored by this formula and by the
  compass simulator's own implementation, at HQ level 117:

  | Fleet | Raw sum | Cn=1 | Cn=2 | Cn=3 | Cn=4 | Difference |
  | --- | --- | --- | --- | --- | --- | --- |
  | 第1艦隊 (6 ships) | 390 | 22.92 | 47.27 | 71.62 | 95.97 | 0.00 at every Cn |
  | 第2艦隊 (4 ships) | 147 | −18.79 | −18.79 | −18.79 | −18.79 | 0.00 at every Cn |

  Neither fleet carries equipment with a LoS bonus (装備ボーナス), which this formula does
  not model; the simulator adds it under the square root. A fleet that does would differ by
  that term.

## Prevention

A fallback that a unit test pins as "compatible" needs one real number next to it. Here
the number was a fleet's raw sum beside the threshold it was compared with.
