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
  - "Changing the practice exp formula (#109), which reads the rival's HQ level"
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

Practice exp is still `calculate_base_exp`: the rival's H × 9, clamped to
100–1200. With the templates every practice gave about 1050 whatever the
player's level; now it follows P and the first group pays more (a new HQ 10
account gets 100–270). This was accepted as a balance change and is pinned by
`practice_base_exp_follows_the_player_level`. The formula itself is #109's to
change, now that rivals have realistic levels to read.

## Known gaps

- The name and comment lists are hand-written and display only.
- Rival ranks, medals and `RivalDetail` counts do not follow H.
