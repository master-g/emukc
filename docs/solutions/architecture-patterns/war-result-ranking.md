---
title: "戦果 by the month, and a ranking the client decodes"
date: 2026-10-15
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
tags: [war-result, senka, ranking, profile, monthly, 1-6, quest, encoding]
applies_when:
  - "Adding a way to gain admiral experience or 戦果"
  - "Changing the ranking screen (api_req_ranking/mxltvkpyuklh)"
  - "The month-end limits: 22:00 for 戦果, 00:00 on the 1st for EO gauges"
---

# 戦果 by the month, and a ranking the client decodes

Plan `2026-10-15-005`, #147. The user chose (2026-10-15): experience and 1-6's goal count, the
month starts over on the 1st in Japan, and the ranking lists every admiral on the server.

## The month's 戦果

`basic::war_result`: 7 for every 10000 admiral experience gained since the 戦果 month began
(「獲得戦果 = 提督経験値 × 7 ÷ 10,000」: anastasiatetris.com
`kancolle-teitoku-exp-senka`, kankore-sheep.blog.jp `82941742`; wikiwiki was not read), rounded down over the month's total, plus the month's bonus
`profile.war_result`. The bonus is what quests reward (`add_war_result_impl`) and the 75 of
1-6's goal (`sortie/escort.rs`). Quest rewards were kept there before, never reset; they now
reset with the month like the rest.

Two columns say which month it is: `war_result_month` (the 戦果 month, year × 100 + month, 0
before the first) and `war_result_base_exp` (experience when it began). `roll_war_result` starts a new
month when the stored one is another: base = experience before the gain, bonus = 0. It runs
before every gain, so it must be called wherever experience or bonus is added:

- `sortie_result.rs` `update_sortie_result_stats`, `practice.rs`, `expedition.rs` (experience);
- `add_war_result_impl` (bonus).

A read in a month with no gain yet is 0 and writes nothing. A new way to gain experience that
skips the roll would count its experience into the previous month's base.

The columns are added to an older database by `ALTER TABLE` at start (as
`fleet.launch_fully_supplied`); its month 0 rolls at the next gain, and experience gained this
month before the upgrade is not counted.

## Month end

Plan `2026-10-15-006`. Three limits, from the pages read (quoted in the plan):

- The 戦果 month ends at 22:00 JST on its last day (`war_result_month_of`, the month at UTC+11):
  experience from then counts towards the next (zekamashi.net `senka-500`; sftsyr.hatenablog.jp
  `2018/12/03`; dsco11 `kancolle-deadline.html`). Quest rewards follow the same clock.
- An EO gauge emptied from 22:00 to 23:59 gives its medal or プレゼント箱 but no 戦果, in
  neither month (`eo_war_result_counts`; zekamashi `senka-500`, blog.makapy.com `8725`; one
  chiebukuro answer says it carries over, the rest say it is lost).
- Monthly maps come back at 00:00 JST on the 1st (`map::monthly_reset_due`), not 05:00 as
  before (sakamoto-kouji.com `2019/08/04`; chiebukuro `q10146468100`). Monthly expeditions keep
  05:00. For 1-6's goal reached across midnight see `reconnaissance-and-escort-cells.md`.

`api_war_result` in `api_get_member/basic` (a field of ours, not the game's) is the month's
戦果.

## The ranking

`Ctx::ranking`: every profile, most 戦果 first, the earlier profile first on a tie, ten to a
page; page 0 (no `api_pageno`) is the asker's own and `api_disp_page` says which. The handler
writes the fields under the names the client reads (`main.js` `SetAll`, `__drawRanking`):

| Field | Holds |
| --- | --- |
| `api_mxltvkpyuklh` | place |
| `api_mtjmdcwtvhdr`, `api_itbrdpdbkynm` | name, comment |
| `api_pcumlrymlujh` | headquarters rank, which picks the flag (≤2, ≤6, the rest) |
| `api_itslcqtmrxtf` | 勲章, shown as `raw / (KEY + 1853) − 157`, hidden below 1 |
| `api_wuhnhojjxmke` | 戦果, shown as `raw / KEY / SEED − 91` |
| `api_pbgkfylkbjuy`, `api_xlqcmisdyfiu`, `api_mcouotbbbzpx` | kept, not drawn: 0 |

`KEY` is the client's key for `place % 13`, `SEED` its `PORT_API_SEED[member_id % 10]`
without the last two digits, the member id being the viewer's. The test
`the_client_decodes_what_is_encoded` decodes the way the client does. The placeholder before
this put the member id where 戦果 goes and 1001 where 勲章 goes, so the screen showed noise.

`make headless-check SCENARIO=escort_1_6` ends on the ranking: after the goal, the admiral is
first with 75 (`ranking`, 2026-10-15).

## Known gaps

- Quest 戦果 is said to close at 14:00 on the last day, and to be carried over or lost
  (in February, May, August and November) by sources that disagree; here it follows 22:00.
- 引き継ぎ戦果, added on the 1st: the year's experience / 50000 and last month's EO 戦果 / 35
  (zekamashi `senka-500`).
- From 22:00 to 23:59 1-6's goal still sends `api_get_eo_rate` 75; what the game shows then
  is unknown.
- EO maps' clear bonuses (`api_get_exmap_rate`) are not sent or counted.
- The game updates the ranking twice a day; this one is live.
