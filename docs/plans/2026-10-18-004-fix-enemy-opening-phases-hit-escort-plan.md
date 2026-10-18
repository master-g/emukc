---
title: "Enemy Opening Phases Hit the Escort Fleet - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# Enemy Opening Phases Hit the Escort Fleet - Plan

Issue #225。来源 `kancolle-replay@69097abc`：`kcsimcombined.js` 我方联合对敌单舰队，开幕对潜（`:207-224`）为 `shellPhase(order1, order2, alive1C, subsalive1C, alive2, ...)`，开幕雷击（`:228-231`）为 `torpedoPhase(alive1C, subsalive1C, alive2, ...)`，敌方都只打护卫舰队。

**范围**：U1–U2，一个 PR。
**非目标**：两阶段以护卫舰队有无活舰为前提的出场门槛；空战等对整支舰队打的阶段里护卫旗舰的击沉保护（#226）；火力与命中常数。

**基线**：`main` `3ad29c24`。

## Implementation Units

### U1 敌方的目标

- `targeting::escort_deck_start`：我方联合舰队护卫舰队的起点，单舰队为 0；闭幕雷击（#219）改用它。
- `simulate_opening_torpedo`、`simulate_opening_taisen`：敌方在 `friendly[escort_deck_start..]` 里选靶，写入战报的下标加回起点。单舰队行为不变。
- 护卫舰队单独挨打时旗舰在 0 号，这两个阶段也受击沉保护，与 #219 一致（`kcships.js:45`、`:1825`）。

### U2 测试与文档

- 单测：空母機動部隊 20 个种子，敌方开幕雷击与开幕对潜的目标都在护卫舰队，且两者都至少出现一次。
- `simulate_day_combined` 注释与 `day-shelling-order.md` 写明并注明来源；golden 若动，按流程重冻并说明。
