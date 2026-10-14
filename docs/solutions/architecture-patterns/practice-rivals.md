---
title: "Practice rivals: drawn from the player's HQ level and practice group"
date: 2026-10-14
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Changing how practice rivals are generated or what they carry"
  - "Changing the practice exp formula, which reads the rival's first two ships"
  - "Touching generate_practice_rivals, draw_practice_rivals or the practice group setting"
tags: [practice, rivals, practice-group, balance]
related_components: [emukc_model, emukc_db]
---

# Practice rivals: drawn from the player's HQ level and practice group

## Context

There are no other players to draw from, so the five practice rivals are made up
(#102, plan `docs/plans/2026-10-14-006-feat-practice-rivals-plan.md`). They used
to be five fixed templates: HQ 119 down to 115, one random level-180 ship each,
and the practice group the player picked was stored but ignored.

## Guidance

`draw_practice_rivals` (`crates/emukc_gameplay/src/game/practice.rs`) is a pure
function of the codex, the player's HQ level P, the practice group and a
`GameRng`. `generate_practice_rivals` reads P from the profile, seeds a
`GameRng` from the thread-local stream (so `rng::seed` in tests still pins the
rivals) and stores the result, replacing the previous batch.

- **HQ level H**, drawn evenly from the clamped range: first group `[P, P+20]`,
  second group `[P−10, P+10]`, "all" picks one of the two per rival; both ends
  clamped to 1–120 before drawing, so low-level players do not get a pile of
  H=1 rivals.
- **Fleet**: 1–6 ships. Each ship first draws its level L evenly from
  `[max(1, H/2), min(H, 99)]` (so no married ships), then a form from the ships
  a player can own whose remodel level is at most L.
- **Ship pool**: the keys of the codex's `ship_extra` (the 865 player ships,
  none abyssal). A form's remodel level is its own `remodel.level`, 1 for a
  first form. Taking the maximum along the remodel path instead gives the same
  value for every ship today, so the path is not walked.
- **One ship per remodel chain**: `Codex::ships_before_and_after` gives the
  whole chain; a candidate whose chain meets a ship already in the fleet is
  dropped and another drawn.
- **Display**: name and comment from `RIVAL_NAMES` / `RIVAL_COMMENTS` in code;
  deck name is the flagship's `api_name` + 「艦隊」. Rank, flag and medals stay
  random; the rest of `RivalDetail` and `RivalShip.star` keep their old fixed
  values. Ships carry their initial equipment at battle time
  (`build_practice_enemy_ships` → `Codex::new_ship`).
- The 補 forms (743/744/745/748/749) are in the pool: `api_mst_ship` lists a
  remodel into each at Lv92. No primary source says whether a player can own
  them; drop their ids from the pool if one says no.

## Why This Matters

Practice exp reads these ships (#109, plan
`docs/plans/2026-10-14-007-feat-practice-exp-plan.md`), so how rivals are drawn
decides what practice pays. `calculate_base_exp`
(`battle/practice/response.rs`) follows wikiwiki 演習:

- raw = the flagship's total exp / 100 + the second ship's / 300 (each
  integer-divided, from `level::ship_level_required_exp`); a lone flagship
  counts a level-1 second ship; later ships do not count;
- base = raw up to 500, then 500 + ⌊√(raw − 500)⌋; no floor and no cap;
- the win rank scales it (`rank_corrected_base_exp`: S 1.2, A/B 1.0, C 0.64,
  D 0.56, E 0.4). `api_get_base_exp` and the ship exp, on top of which the
  MVP ×2 / flagship ×1.5 / training cruiser / `practice_exp_boost` multipliers
  apply, both use the scaled value. The snapshot keeps the unscaled base, so
  the night battle can rescore with its own rank, and admiral exp still reads
  the unscaled base (`calculate_admiral_exp`; the wiki gives no formula).

Checked against the wiki's 基本経験値表: 361 of 362 cells match. The odd one
(flagship 150, second 140: wiki 727, formula 728) sits among matching cells and
is taken as a wiki typo. The wiki's "0–3 extra" is not reproduced.

Practice pays less than it did. The rival's HQ × 9 gave 1035–1071 with the old
templates; two level-99 ships now give 613 (735 on S). Rival ships never
outlevel the rival's HQ, so a new account's rivals give tens to about two
hundred, and a lone level-1 flagship gives 0. Both drops were accepted;
`practice_base_exp_follows_the_player_level` pins the ranges.

## Known gaps

- The name and comment lists are hand-written and display only.
- Rival ranks, medals and `RivalDetail` counts do not follow H.
