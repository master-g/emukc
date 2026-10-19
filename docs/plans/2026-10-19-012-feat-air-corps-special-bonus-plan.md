---
title: "Air Corps Special Bonus - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Air Corps Special Bonus - Plan

Issue #259。来源 `kancolle-replay@69097abc`：`kcsim.js:3300-3329`（`airstrikeLBAS`，`enableAirstrikeSpecialBonus` 与 `enableSummerHarbourLBASBonus` 默认开）；`kcSHIPDATA.js` 的 `LBWeak`、`divebombWeak`；`kcEQDATA.js` 的 `isdivebomber`。

**基线**：`main`（#261 之后）。
**非目标**：`enablePlaneBonus`/`bonusSpecialP`（活动专用）；关掉特殊加成时的旧分支（`kcsim.js:3347-3355`）；B-25 等按装备 id 的对舰种倍率（`kcsim.js:3243-3292`，另议）。

**前提（已核对）**：

- 来源的 `isdivebomber` 在基地航空队能出击的机种里只缺艦攻（8）：艦爆、水爆、オートジャイロ、対潜哨戒機、陸攻（47、53）、噴式戦闘爆撃機都是。来源没有 58 号机种。
- 链里常规图出现的目标（`map_catalog.json` 敌编成）：6-4 的 1620、1653、1656、1665–1672，6-5 的 1586，7-5 的 1665、1666。兜底的 `LBWeak`/`divebombWeak` 只落在 1684–1692，常规图不出现。

## Implementation Units

### U1 基地航空队的目标加成

- `air_base.rs` 新 `special_bonus`：照来源链的顺序首个命中即止，命中才抽一次（0.35/0.4/0.5 各档照抄）；PT 档从 `pt_imp::air_corps_strike_roll` 并入，后者删除。都不中时不抽签：cap 前乘 `LBWeak`（只列链没先接住的 1687–1692），非艦攻再在 cap 后乘 `divebombWeak`（复用 `kouku::dive_bomb_weak`）。
- `strike_power` 多收一个 cap 前倍率，乘在既有 `pre × scouting` 之后（1.0 时浮点结果不变）。
- 抽签位置不变：命中判定之后、潜艇的 cap 前随机数与装甲之前（来源 `if (res)` 内；链上的目标都不是潜艇，两者不会同时抽）。
- 单测：一个有抽签的档（6-4 的 1668，2.0/1.5）、PT 档照旧、`LBWeak` 进 cap 前、陸攻吃 `divebombWeak` 而艦攻不吃。
- 文档：`anti-installation.md`、`air-corps.md` 的 Known gaps。
- golden：180 份与 `battle_golden.rs` 未变（现有场景没有航空队打链上的目标）。
