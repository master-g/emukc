---
title: "Abyssal ship stats: totals in battle, her own on the wire"
date: 2026-10-22
category: architecture-patterns
module: emukc_model
problem_type: architecture_pattern
component: service_object
severity: high
tags: [battle, abyssal, enemy, stats, eparam]
applies_when:
  - "Changing how an abyssal ship is built (Codex::new_enemy_ship)"
  - "Writing a battle rule that needs a ship's own figure (the source's statsBase)"
  - "Changing what api_eParam reports"
  - "Changing where enemy_ship_extra's stats come from (kcwiki or the source)"
related_components: [emukc_battle, emukc_gameplay]
---

# Abyssal ship stats: totals in battle, her own on the wire

`Codex::new_enemy_ship` builds an abyssal ship's `api_karyoku`, `api_raisou`, `api_taiku`,
`api_soukou`, `api_kaihi`, `api_taisen` and `api_sakuteki` as her own figure (the codex's
`enemy_ship_extra`) plus her equipment's, both entries alike. The source does the same:
`createDefaultShip` calls `loadEquips(…, true)`, which adds every equipment stat
(`KC3Kai/kancolle-replay@69097abc`, `kcsim.js:4132-4147`, `kcships.js:536-546`). A player ship's
`[0]` is a total too (`cal_ship_status`), so battle reads both sides the same way. Abyssal
equipment has no luck, and her luck is her own.

## Her own figures

The source keeps `statsBase` (total less equipment and 装備ボーナス, `kcships.js:1033-1038`) for
the rules that ask it. Here they derive it:

| Rule | Source | Here |
| --- | --- | --- |
| ASW power, `canASW` | `statsBase.ASW` | `damage::base_asw`, `targeting::bare_asw` |
| Takes part in torpedo phases | `hasTorpStat` | the hull's `api_raig` with 近代化改修 (`targeting::has_own_torpedo`) |
| Carrier night air attack | `statsBase.FP + statsBase.TP` | the hull's `api_houg`/`api_raig` (`night_air_power`) |
| Friendly 加重対空 | `statsBase.AA / 2` | total less equipment (`weighted_anti_air`); an enemy takes `sqrt` of her total, as the source does |

The codex `start2` entries of abyssal ships carry her own firepower, torpedo, anti-air and armour,
equal to `enemy_ship_extra` for all 889 (2026-10-22), so a hull lookup gives her own figure too.
A new rule that wants her own figure must derive it like these; `api_*[0]` holds her equipment's.

## Where her own figures come from

kcwiki's enemy data has no anti-submarine, evasion, line of sight or luck for most abyssal ships,
so bootstrap takes those four from the source's `kcSHIPDATA.js` (`abyssal_stats.json`, generated
by `scripts/gen_abyssal_stats.py`; #284, #289), estimated figures (`unknownstats`) included, and
the source's wins where both have one. HP, firepower, torpedo, anti-air and armour stay kcwiki's.
Where they differ (2026-10-22) neither is right throughout, judged by wikiwiki (kcwiki's wrong
ones are #293):

| Ship | Stat | kcwiki | Source | wikiwiki |
| --- | --- | --- | --- | --- |
| 集積地棲姫III バカンスmode 2144 / -壊 2145 | 火力, 雷装, 対空, 装甲 | swapped (2145 火力 191) | 168/144/101/191, 198/188/121/221 | as the source |
| 駆逐ラ級 2264 | 装甲 | 158 | 169 | 169 |
| 駆逐ラ級(初期型) 2270 | 対空 | 79 | 78 | 110 with 32 from equipment |
| 駆逐ラ級(初期型)-壊 2273 | 雷装 | 98 | 131 | 116 with 18 from equipment |
| 集積地棲姫IV 2166 | 装甲 | 80 | 88 | 80 |
| 潜水新棲姫flagship 2049 | 装甲 | 33 | 17 | 33 |

The other seven (層間棲艦水鬼長女-壊 2353–2355, 軽巡ウ級 2376–2379) are placeholders in both:
kcwiki has none, the source 1 for each. `new_enemy_ship` gives them 1 HP either way.

Range (182 ships) and speed (15) differ too and stay kcwiki's, unexamined (#294): the source's
range is not to be taken wholesale (2264 `RNG: 1`, where wikiwiki and kcwiki have 中).

## The wire

`api_eParam` is 敵基礎ステータス (`docs/apilist.txt`): her own figures. `response::enemy_params`
reads `enemy_ship_extra` for an abyssal ship and the battle totals for anyone else. A practice
rival is the else: the real server sends her bare stats too (capture of 2026-09-22), this project
sends her totals.
