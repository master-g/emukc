---
title: "Aircraft Proficiency in Fighter Power - Plan"
type: feat
date: 2026-10-10
status: implemented
execution: code
---

# Aircraft Proficiency in Fighter Power - Plan

## Problem

计划 2026-10-10-004 把熟练度接进了命中与暴击，并把「制空値的熟练度加成」留作另立计划。现在制空値只算
`floor(対空 × sqrt(機数))`，带熟练机的舰队制空偏低，制空状態、随之而来的逐槽损失与弹着观测都跟着偏。

## Evidence

**我方现状：** `simulation/kouku.rs` 的 `calculate_fighter_power` 是舰队制空値的唯一入口，四处调用
（开幕航空战双方、基地空袭的敌方、航空队出击的敌方）。敌舰由 manifest 构造，`api_alv` 为 `None`。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcships.js` `setProficiency`（2501）：`APbonus = sqrt(exp × 0.1)`，`exp` 取等级对应的
  `[0,10,25,40,55,70,85,120]`（不做对潜哨戒机的 0.825 折算，那只在命中/暴击一侧）。再按机种：
  - 艦戦、水戦、局戦、噴式戦闘機（type 56）：加 `[0,0,2,5,9,14,14,22][等级]`。
  - 水爆：加 `[0,0,1,1,1,3,3,6][等级]`。
  - 艦攻、艦爆、噴式戦闘爆撃機（type 57）：只有根号项。
  - 其余：在舰上为 0；从基地起飞（`forLBAS`）保留根号项。
- `kcships.js` `airPower`（1839）：每槽 `floor((対空 + AAImprove) × sqrt(機数) + APbonus)`，加成在
  取整**之内**。

## Decision

1. `calculate_fighter_power` 每槽加上该装备的熟练度加成，取整位置同来源。
2. 加成写成按机种与等级取值的纯函数，带一个「从基地起飞」的开关，留给航空队（下一份计划）用。
3. 没有熟练度的槽，结果与现在逐位相同。

## Scope

**不做：** `AAImprove`（★改修的対空加成，现有制空値本来就没算）；`MECHANICS.aswPlaneAir` 对 489/491 两件
装备的特例；航空队的制空値与熟练度（下一份计划）。

## Implementation Units

- **U1** `simulation/kouku.rs`：`proficiency_fighter_power` 与两张等级表；`calculate_fighter_power` 调用它。
  `accuracy.rs` 的 `PROFICIENCY_EXP` 改为 crate 内可见以复用。测试钉住：対空 10、18 機、等级 7 的艦戦
  42 → 67；经 codex 真实装备的艦戦（等级 0/1/2/7）、水爆、艦攻各一例；陸攻在舰上为 0、从基地为根号项。
- **U2** 基线若有变化则重新冻结并说明；更新 `battle-hit-and-critical-rolls.md` 的「Not modelled」。

## 实施记录（2026-10-10）

- U1 按计划做完。噴式三种的对应：来源的 `JETFIGHTER`=56、`JETBOMBER`=57，分别是我方的 `JetFighter`、
  `JetFighterBomber`；`JetAttacker`(58) 来源没有常量，落进「其余」。
- 基线没有变化：`crates/emukc_battle/tests/golden/` 与 `battle_golden.rs` 都不需要重新冻结。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、`cargo test --workspace`。
- 客户端不重算制空値，只读 `api_disp_seiku`，所以这一项没有新的演出字段；无头检查只用来确认没有回归。
