---
title: "Airstrike Special Bonus and Carrier Shelling of Land - Plan"
type: feat
date: 2026-10-19
status: in-progress
execution: code
---

# Airstrike Special Bonus and Carrier Shelling of Land - Plan

Issue #200。来源 `kancolle-replay@69097abc`：`kcsim.js:2053-2110`（`airstrike`，`enableAirstrikeSpecialBonus` 默认开）；`kcships.js:1986` `CV.prototype.shellPower`。

**基线**：`feat/smoke-screen`（#199）之上。
**非目标**：`isSummerCV`/`isFrenchBB`/`isEuroWP`（issue 已排除）；基地航空队自己的特殊加成链（`airstrikeLBAS`，#200 未列，记入 `anti-installation.md` Known gaps）。

## Implementation Units

### U1 航空战的目标加成

- `kouku.rs` `special_bonus`：照来源链的顺序首个命中即止，各抽一次（多数 0.4，补给基地 0.5）；都不中时俯冲轰炸机（7、11、57）乘目标的 `divebombWeak`（只列链没先接住的 14 个 id），不抽签。PT 分支从 #197 的 `carrier_strike_roll` 并入。
- 之后乘攻击方的船坞与夏季舰 post-cap（`post_cap_multiplier(.., torpedo = true)`，与来源 airstrike 只取这三项一致）。
- 抽签位置：鱼雷机 ×0.8/×1.5 之后、装甲之前（来源 `if (res)` 内，preMod 之后）。
- 单测：链的首中、补给基地 0.5、`divebombWeak` 不抽签、鱼雷机后再抽；既有「对地鱼雷机只有基础 25」改为含砲台小鬼的 1.3/1.7。
- golden：180 份与 `battle_golden.rs` 未变。

### U2 空母昼战炮击对地

待定：现行空母炮击公式是简化的 `1.5 × 轰炸机机数 + 55`（`damage-formula-reference.md` S2），没有火力项可乘对地倍率。
