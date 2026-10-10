---
title: "Plane Losses Slot by Slot - Plan"
type: fix
date: 2026-10-10
status: implemented
execution: code
---

# Plane Losses Slot by Slot - Plan

## Problem

开幕航空战里，模拟先算出一方在一个阶段总共损失几架，再一架一架从机数最多的槽里扣。实机是每个槽各自损失，
小槽最容易被打空。计划 `2026-10-10-005` 让损失入库之后，这个偏差开始决定哪些槽花铝土、哪些槽掉熟练度，
并且让「打空归 0」几乎不发生。无头实测：赤城 18/18/27/10 打完一场是 15/15/16/10（第二次是 15/15/15/10），10 架的槽一架没掉。

同一次实测还说明总量偏重：我方制空确保、赤城只带艦戦，一战掉了 73 架里的 18 架。原因在第 2 阶段：
对空射击被算到了全部飞机头上，而它只该打来攻击的机队。

## Evidence

**我方现状（`simulation/kouku.rs` `simulate_kouku`）：**

- 第 1 阶段：按制空状态在一个区间里抽一个比例，乘全队机数得总损失，交给 `apply_plane_losses`
  （每次挑机数最多的槽减 1）。
- 第 2 阶段：`对方全队対空合计 / 400 × 机数`，同样交给 `apply_plane_losses`。代码注释自己写着这是已知的简化。
- `simulation/air_base.rs` 给航空队的第 2 阶段已经按来源实现了加重対空与舰队防空（`weighted_anti_air`、
  `fleet_anti_air`），只认敌方舰。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`，`kcsim.js`）：**

- 第 1 阶段 `AADefenceFighters`（2330），逐槽，参与的机种是 `isfighter`（艦戦、艦攻、艦爆、水爆、水戦、噴式）：
  - 我方：`floor(机数 × (rmin + r))`，`r = floor((floor(1000 × rplus) + 1) × rand) / 1000`；
    确保 / 優勢 / 均衡 / 劣勢 / 喪失 的 `rmin, rplus` 是 `.025,.0333 / .075,.1 / .125,.1666 / .175,.2333 / .25,.3333`。
  - 敌方：`floor(机数 × (0.35 × a + 0.65 × b) / 10)`，`a`、`b` 各是 `0..rmax` 的整数；`rmax` 按**敌方自己的**制空状态
    确保到喪失取 `2 / 5 / 7 / 9 / 11`。
  - 噴式机的损失乘 0.6。
- 第 2 阶段 `AADefenceBombersAndAirstrike`（2543），逐个攻击机槽（`isBomberS2`：艦攻、艦爆、水爆、噴式爆撃），
  每个槽被对方一艘随机的存活舰射击：
  - 比例击坠，一半概率：`floor(机数 × 加重対空 / 200)`。
  - 固定击坠，一半概率：`floor((加重対空 + 舰队防空) × 系数)`，系数防守方是我方时 0.2、敌方时 0.1875；
    我方的舰队防空先除以 1.3。
  - 最低保证：防守方是我方时再加 1。
  - 连合舰队的防守舰，加重対空与舰队防空乘 0.8（第 1 舰队）或 0.48（第 2 舰队）。
- 加重対空 `weightedAntiAir`（`kcships.js:1619`）：我方是 `素対空 / 2 + Σ 装备対空 × 倍率`，
  敌方是 `sqrt(対空) + Σ 装备対空 × 倍率`；倍率与 `air_base.rs` 已有的一致。

## Decision

1. 第 1、2 阶段都改成逐槽结算，公式照上面转写。`apply_plane_losses` 不再被舰载机航空战调用。
2. `weighted_anti_air` 认我方舰（素対空取 `api_taiku[0]` 减去装备対空）；两个函数从 `air_base.rs` 借给 `kouku.rs` 用。
3. 第 1 阶段报的机数不变（仍是全部参战机）；第 2 阶段报的机数改成来源的口径：只算被射击的攻击机槽。
4. 每槽的抽签数变了，所有带航空战的种子都会移动：`crates/emukc_battle/tests/golden/` 有意重新冻结。

## Scope

**不做：** 対空カットイン、阵形的防空倍率、改修与装备加成的対空、噴式强袭、対空噴進弾幕、熟练机的击坠抵抗；
航空队攻击与基地空袭里敌机的第 1 阶段损失（`air_base.rs`、`air_raid.rs` 仍走 `apply_plane_losses`）。

## Implementation Units

- **U1** `types/domain.rs`：`AirState` 给出逐槽的两张表。`simulation/air_base.rs`：两个防空函数放开给同级模块，
  `weighted_anti_air` 分我方与敌方。
- **U2** `simulation/kouku.rs`：两个阶段逐槽结算。测试：小槽会被打空；喪失时每个槽至少损失四分之一；
  没有攻击机的队不吃对空射击；防守方为我方时每个被射击的槽至少掉 1 架。
- **U3** 重新冻结文本基线；带种子的测试落空时换种子不放宽断言；更新 `plane-losses-and-proficiency-growth.md`
  （删掉 Known deviation）与 `PROJECT_MEMORY.md`。

## 实施记录（2026-10-10）

- U1、U2 按计划做完。`fight_for_the_air` 与 `fly_through_anti_air` 逐槽结算；`weighted_anti_air` 认我方舰。
- 基线：`crates/emukc_battle/tests/golden/` 的 20 份文本基线重新冻结（每个带飞机的槽现在各抽一到四次签，
  之后的随机流全部移动）；`battle_golden.rs` 没变。
- 换种子的测试：`kouku.rs` 的两个 `kouku_fdam_*` 从 42 换成 2。它们要求敌方轰炸命中，而对回避高的驱逐舰命中率只有四成，
  原种子在新的随机流里落空；断言没动。
- `plane_proficiency.rs` 的「战后机数入库」改用带攻击机的 `carrier_cutin` 场景：制空确保下只带艦戦的队一战常常一架不掉，
  原来「一定有损失」的前提来自旧模型。
- 损失量的变化：同一场 2-1，只带艦戦的赤城从一战掉 18 架变成 0–2 架；六艘各带艦爆艦攻的空母一战共掉 7 架。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings`、`cargo test --workspace`。
- 无头检查 `gunnery_cutin` 与 `leveled_for_mid_boss` 通过。`gunnery_cutin` 的数据库里赤城仍是 18/18/27/10：
  制空确保、只带艦戦，每槽至多损失 5.8%，这一战一架没掉（原先写的「各槽都有损失」是按旧模型的损失量预期的）；
  四架艦戦的经验是 6、8、8、7。`leveled_for_mid_boss` 第一次运行卡在世界选择页（未进入游戏），重跑通过。
