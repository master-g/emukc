---
title: "Gauges Count Sunk Flagships - Plan"
type: fix
date: 2026-10-07
status: implemented
execution: code
---

# Gauges Count Sunk Flagships - Plan

## Problem

击破型血条（1-5、2-5、7-2 等带 `required_defeat_count` 的常规图）在真实游戏里只有击沉 boss 旗舰才减一格。
`apply_sortie_map_result` 原先只看「是不是 boss 点」和「胜负是不是 S / A / B」，旗舰没沉的胜利也减一格，
血条因此比真实游戏好打。用户实测发现，2026-10-07 确认要改。

## Decision

- 击破型血条：boss 点胜利（B 以上，沿用原门槛）且敌旗舰最终耐久 ≤ 0 才计数。旗舰是否击沉取
  `settle_sortie_battle_impl` 的 `final_enemy_nowhps[0]`，即夜战之后的那份，与 `api_dests` 同源。
- 5-6 第一段的占位（真实是输送血条，不要求击沉旗舰）改为显式的「A 胜以上计数」：
  `map_gauge_rules.json` 用 `wins` 代替 `defeats`，变体上是 `gauge_counts_wins`。
- 不动：没有血条的图（boss 点 B 胜以上即通关）；HP 型血条分支（活动图，当前路线图外）。

## Verification

`a_defeat_gauge_moves_only_when_the_boss_flagship_sinks`；三道质量门。
