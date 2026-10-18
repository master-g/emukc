---
title: "Enemy Fires at the Round's Deck - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# Enemy Fires at the Round's Deck - Plan

Issue #219。来源 `kancolle-replay@69097abc`：`kcsimcombined.js` 我方联合对敌单舰队，`shellRange`/`shellOrder`（`:234-266`）调用 `shellPhase(order1, order2, alive1f, ...)`，`alive1f` 是这一轮那支舰队；闭幕雷击（`:298`、`:330`）为 `torpedoPhase(alive1C, ...)`，敌方只打护卫舰队。

**范围**：U1–U2，一个 PR。
**非目标**：开幕对潜与开幕雷击里敌方打谁（源码 `:212`、`:231` 同样只给 `alive1C`，另记）；`hasInstall1f` 出场门槛；火力与命中常数；12 对 12。

**基线**：`main` `7f22d4c4` 之后的合并提交。

## Implementation Units

### U1 敌方的目标

- `simulate_shelling_round`：敌方打 `friendly[friendly_deck]`，下标按 `friendly_deck.start` 换算，回合内终止判断也看这一段。单舰队与敌联合时 `friendly_deck` 是整支舰队，行为不变。
- `simulate_raigeki`：敌方的候选取我方护卫舰队那一段（没有护卫舰队时取整支），下标加回起点。
- 护卫舰队作为单独一段挨打时旗舰在 0 号，受击沉保护；源码每支舰队的旗舰都有（`kcships.js:45`、`:1825`），与夜战一致。

### U2 测试与文档

- 单测：空母機動部隊与水上打撃部隊各 20 个种子，每轮敌方的目标都在该轮那支舰队，闭幕雷击的目标都在护卫舰队且至少出现一次。
- `ShellingRound` 字段注释、`simulate_day_combined` 注释与 `day-shelling-order.md` 写明并注明来源；golden 若动，按流程重冻并说明。
