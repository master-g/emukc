---
title: "Air Corps Proficiency - Plan"
type: feat
date: 2026-10-10
status: implemented
execution: code
---

# Air Corps Proficiency - Plan

## Problem

航空队的中隊不带熟练度进战斗：`AirSquadronInput` 只有装备与機数，出击与防空的制空値不含熟练度加成，
攻击抽签传的是 `PlaneProficiency::NONE`，所以航空队永远不出暴击。计划 004 与 007 都把它留到这里。

## Evidence

**我方现状：**

- `airbase/mod.rs` 的 `striking_air_corps_impl` 与 `raided_air_corps_impl` 构造中隊时手里就有装备行，
  `aircraft_lv` 没有带出去。
- `simulation/air_base.rs` `fighter_power`（出击）与 `simulation/air_raid.rs` `defence_power`（防空）
  都是 `floor(基础値 × sqrt(機数))`。
- 航空队攻击的 `api_ecl_flag` 写死为 0；客户端按 `_friend ? api_fcl_flag : api_ecl_flag` 读暴击
  （`main.decoded.js` 123104），与开幕航空战同一处。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `LandBase` 构造用 `new Equip(id, level, prof, true)`，即 `setProficiency(rank, forLBAS=true)`：
  艦戦/水戦/局戦/噴式戦闘機与水爆的阶梯同舰上，其余机种保留 `sqrt(exp × 0.1)`。
- `LandBase.airPower`（2241）与 `airPowerDefend`（2258）：每中隊
  `floor(基础値 × sqrt(機数) + APbonus)`。
- `kcsim.js` `airstrikeLBAS`（3135）：按**该中隊自己**的等级算，不做按舰平均。`MECHANICS.LBASBuff`
  默认开（380 行），所以陸攻也算；噴式强襲阶段不算。
  - 对潜哨戒机与旋翼机：经验 × 0.825，等级减 1。
  - 命中 `sqrt(exp × 0.1)` 加等级 1–7 对应的 `0,1,2,3,4,6,9`；等级 0 为 0。
  - 暴击率 `critval × 0.8`，暴击伤害 `1 + floor(sqrt(exp) + critval) / 100`，`critval` 同舰上的
    `1,2,3,4,5,7,10`。
- `kcsim.js` 3071 / 3944：中隊在出击中被打到 0 機时标记 `emptied`，战斗结束把熟练度清零。

## Decision

1. `AirSquadronInput` 增加 `alv`，由装备行的 `aircraft_lv` 填入。
2. 出击与防空的制空値每中隊加 `proficiency_fighter_power(.., land_base = true)`。
3. 攻击的抽签与暴击伤害用该中隊自己的熟练度；出暴击时置 `api_ecl_flag`。仍是一次攻击一次抽签。

## Scope

**不做：** 航空队熟练度的任何变化——成长、按损失比例的下降、打空清零。来源模拟器只有打空清零
（`emptied`），成长不建模；只做清零会让熟练度在航空队里只降不升，比不做更偏离真实游戏，所以三者留到
有成长的出处时一起做。陸上偵察機对制空値的倍率
（`landscoutmod`，现有出击制空値本来就没有）；`AAImprove`；噴式强襲。

## Implementation Units

- **U1** `emukc_battle`：`AirSquadronInput::alv`；`accuracy.rs` `squadron_proficiency`；`air_base.rs` 的
  制空値、抽签、暴击伤害与 `api_ecl_flag`；`air_raid.rs` 的防空制空値。测试钉住满熟练度陸攻的三个值
  （命中 `sqrt(12)+9`、暴击率 8、暴击伤害 1.2）、哨戒机的折算、雷電防空 76 → 101、200 个种子里无熟练度
  不出暴击而满熟练度出。
- **U2** `emukc_gameplay`：两处构造带上 `aircraft_lv`。
- **U3** 更新 `air-corps.md`、`battle-hit-and-critical-rolls.md`、
  `plane-losses-and-proficiency-growth.md`；基线若有变化则重新冻结并说明。

## 实施记录（2026-10-10）

- U1–U3 按计划做完，基线没有变化。
- 初版照来源加了「出击后機数为 0 的中隊熟练度清零」，随后撤掉：航空队没有成长，清零只会单向往下掉。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、`cargo test --workspace`。
- 无头检查 `air_corps_6_4`：确认带航空队的出击在真实客户端里能演完。
