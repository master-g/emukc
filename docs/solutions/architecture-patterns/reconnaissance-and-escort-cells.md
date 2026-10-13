---
title: "Reconnaissance and escort-success cells: rules in code, by event id"
date: 2026-10-13
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [sortie, map, reconnaissance, airsearch, escort-success, landing, 1-6, 6-3, 5-6, rng]
applies_when:
  - "Changing what 6-3's reconnaissance finds or when it succeeds"
  - "Changing what the goal of 1-6 gives or how its gauge empties"
  - "A seeded sortie that passes 6-3 G or H, or ends on 1-6 N, moved"
  - "A second way into a node has another event id than the first"
---

# Reconnaissance and escort-success cells: rules in code, by event id

## What

Three kinds of cell have rules of their own, and the client plays each by its `event_id`
alone (`TaskNextSpot._cellEvent`):

| `event_id` | Cell | Where | Server |
| --- | --- | --- | --- |
| 7 | 航空偵察 | 6-3 G, H | `sortie/reconnaissance.rs` |
| 8 | 船団護衛成功 | 1-6 N | `sortie/escort.rs` |
| 9 | 揚陸地点 | 5-6 E | nothing to give; `landing_tp`, see below |

`next_sortie` settles 7 and 8 in the transaction that settles a resource cell. The rules
are constants in code: the regular maps have one map of each kind. A second one is the
time to move them into `map_cell_effects.json`.

## Reconnaissance (6-3 G, H)

Source: wikiwiki 中部海域, 6-3 「航空偵察について」 and 「航空偵察で獲得できる資源一覧」,
read 2026-10-13.

- **Score:** each slot of 水上偵察機, 水上爆撃機 or 大型飛行艇 (`type3` 10, 11, 41) gives
  索敵 × ⁴√(aircraft left in the slot). A slot with none left gives nothing.
- **Result:** the node's constant is 12 at G and 16 at H. A score of the constant is a
  success; of the constant × *r* ÷ 10, *r* a whole number from 16 to 22, a great success.
  No scoring slot at all: `api_plane_type` 0 and the client says the reconnaissance is
  called off.
- **What it finds:** one of twenty, drawn evenly. 弾薬 50, 60, 80 (great: 100, 150, 200),
  ボーキサイト 30, 40, 50 (80, 100, 150), 開発資材 1 (2, 3, 4).
- **When it is given:** it is shown at the cell and kept in the sortie's state
  (`ActiveSortieState::recon_rewards`). A B or better at the boss gives it
  (`settle_sortie_battle_impl`), at once: the boss is the end of the map.

Ours, not the source's:

- Only 開発資材's seven of twenty follow from it (0.35 on average by a success, 1 by a
  great one, so 2, 2, 3, 3, 3, 3, 4). 弾薬 has seven and ボーキサイト six, each amount as
  often as the next.
- 「以上」, not 「より大きい」: the source says both; its worked figures (12以上で成功確定)
  decide.
- `api_plane_type` is 1 where a flying boat scores and 2 otherwise. The client has no
  picture for a 水上爆撃機.

## Escort success (1-6 N)

Source: wikiwiki 鎮守府海域, 1-6.

- 1-6 has no boss. Arriving at N is what empties its gauge: seven arrivals
  (`api_required_defeat_count`), counted in the map record's `defeat_count`.
- Every arrival gives one of fuel, ammunition, steel and bauxite. It waits for the port
  with what the resource cells gave (`cell_effect::put_by`).
- The seventh gives 燃料 1000, a プレゼント箱 (`api_mst_useitem` 60, at once) and 戦果 75,
  and clears the map. The monthly reset is the map's, as before.

Ours: while the gauge stands the resource is drawn evenly and its amount evenly from 100
to 1000 by the hundred; afterwards from 10 to 100 by the ten. The source has 「100単位」,
「多いものでは1000」 and 「大幅に減少」.

## Landing (5-6 E)

The cell gives nothing. The client's `CellTaskLanding` reads the map's gauge and the
fleet's equipment and no field of the response; the cargo is counted there
(`landing_tp`) and landed at the boss (`land_transport`).

## The wire

- `api_airsearch` goes with every step, `{0, 0}` where there is no reconnaissance: a real
  6-3 start has it so. At a reconnaissance cell `api_itemget` carries the one thing found;
  the client's balloon draws icons 2, 4 and 7 only.
- `api_itemget_eo_comment`: `api_usemst` 4, `api_id` 1 to 4, `api_getcount`. The client
  takes one off the gauge itself, from `defeat_required − defeat_count` as the map list
  had them before the sortie.
- `api_itemget_eo_result` (`api_usemst` 5, the use item's id) and `api_get_eo_rate` only
  with the arrival that empties the gauge.

## The catalog

- 1-6's count came from `DEFAULT_MAP_RECORDS` as 4; the master data has 7.
- A node with two ways in is two cells. The sources that name a cell's kind know the
  first only, so 1-6's 17 (B → N) was a plain cell and 6-3's 12 (G → H) a landing cell,
  its colour being the same 9. `kcnav::apply_cell_events` corrects both from the recorded
  routes.

## Known gaps

- 戦果 is not kept anywhere; 75 is for the client to show.
- 1-6's month-end limits (21:59 for 戦果, 23:59 for the プレゼント箱).
- The quests that count 「ゴール地点到達」.
- Played in the client on 2026-10-13 by `make headless-check SCENARIO=escort_1_6` (the
  arrival that empties the gauge) and `SCENARIO=recon_6_3` (a great success by flying boat
  at G and H), without a page error: `headless-client-check.md`. Not played: an arrival
  that leaves the gauge standing or comes after it, a plain success, a failure, a
  reconnaissance called off, the seaplane's picture. The balloon was read for 開発資材
  only; 弾薬 and ボーキサイト came up in later runs whose screenshots nobody read.
- 6-3 A is a choice of route that the client is never offered: the start answers without
  `api_select_route` (#106).
- Event maps.

## When a test breaks

- A seeded sortie moved: a reconnaissance draws once when its score reaches the constant
  and once more when it finds something; an arrival at N draws twice, and not at all when
  it empties the gauge. Move the seed, not the assertion.
- `tests/gameplay_tests/map/cell_effects.rs` reaches 6-3 G with 水母 1, 軽巡 1, 駆逐 4;
  秋津洲 2 and one 駆逐 never got there. A ship's own equipment counts towards the score:
  千歳 with one 零観 put in her first slot scored a great success where 11.17 was expected.
