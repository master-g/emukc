---
title: "Day Shelling: Both Sides Fire in Every Round, Ship by Ship - Plan"
type: fix
date: 2026-10-10
status: implemented
execution: code
---

# Day Shelling: Both Sides Fire in Every Round, Ship by Ship - Plan

## Problem

单舰队昼战里，炮击第一轮整轮只有一方出手，第二轮只有另一方；第二轮只在场上有战舰时存在。
没有战舰的战斗里，后手一方整个昼战不开炮。联合舰队每轮双方都打，但是一方六艘打完才轮到另一方。
谁先手由双方舰队航速决定，这条规则在任何资料里都找不到。

## Evidence

**我方现状（2026-10-10 读代码，`crates/emukc_battle/src/simulation/`）：**

| 位置 | 现状 |
|---|---|
| `mod.rs` `execute_shelling1` | `enemy_first` 为真时只调敌方的 `simulate_shelling_side`，否则只调我方 |
| `mod.rs` `execute_shelling2` | 调另一方；`has_bb_class_at_start` 为假时整轮不存在 |
| `mod.rs` `execute_combined_shelling`、`execute_enemy_deck_shelling` | 双方各调一次整轮，`merge_hougeki` 首尾相接 |
| `mod.rs` `enemy_shells_first` | 敌方舰队最低航速高于我方则敌方先手 |
| `shelling.rs` `simulate_shelling_side` | 一方按舰队位置依次出手；旗舰特殊攻击在整轮开始前判定 |

**模拟器 30 个种子的统计（2026-10-10，`battle sim --seed 1..30`）：** `leveled_for_mid_boss`（六驱逐打 2-1）
敌方炮击 0 次、我方 180 次；`transport_5_6` 敌方炮击 0 次；`gunnery_cutin` 第一轮只有敌方、第二轮只有我方。

**参考：`KC3Kai/kancolle-replay` 的 `kcsim.js`**（命中判定用的同一份本地副本）：

- `shellPhase`（1543）：先建好双方的出手名单，然后按下标交替——我方第 i 艘、敌方第 i 艘；轮到时再查一次
  还能不能打（没被击沉、空母没被打到不能发舰）；任一方全灭就停。我方永远先手。
- `orderByRangeOld`（3381）：第一轮名单按射程从长到短，同射程内洗牌。（新版 `orderByRange` 在排序比较函数里
  掷随机数，结果依赖排序算法，不照抄。）
- `sim`（3828–3842）：第二轮名单按舰队位置；`doShell2` 是开战时任一方有 `BB` / `BBV`，
  与我方 `has_bb_class_at_start` 一致。
- 名单只收开轮时能炮击的舰（`canShell`）；潜水艇不进名单。
- `shellPhaseC`（1601）：联合舰队的每一轮是同一个交替循环，只是目标池不同。
- 特殊攻击（`canSpecialAttack`，1150）：轮到旗舰出手时判定，不在整轮之前。

**射程数据：** `KcApiShip.api_leng` 我方来自 `api_mst_ship`（`codex/ship.rs:282`），敌方来自
`enemy_ship_extra` 的 `range`。两边都不含装备带来的射程延长，装备的射程在 `ApiMstSlotitem.api_leng`。

**客户端：** 炮击包每次攻击自带 `api_at_eflag`，客户端按数组顺序播放。联合舰队的包已经在一轮里混着双方，
无头检查跑过；改成逐舰交替不改变包的形状。

## Decision

1. 每一轮炮击都是双方逐舰交替，我方先手。删掉 `enemy_shells_first`、`fleet_speed` 和它的测试。
2. 第一轮按射程从长到短，同射程用 `BattleRng` 洗牌；一艘舰的射程取舰本身与所带装备里最长的。
   第二轮按舰队位置。
3. 名单在开轮时建，只收当时能炮击的舰；轮到时再查一次。任一方（本轮的目标池）全灭就结束本轮。
4. 旗舰特殊攻击挪到旗舰自己那一手判定。参加者在本轮里还没出手的照旧跳过。
5. 联合舰队沿用现有的「哪一轮是哪个分队、写进哪个 `hougeki`」不变，只把轮内改成交替：
   同一分队连续的第二轮按位置，其余按射程；敌联合的第三轮（对全体）按位置。

## Scope

**做：** 单舰队、我方联合、敌方联合三条昼战炮击路径的轮内顺序。

**不做（列为已知差异）：**

- 先制对潜、夜战的交替顺序，雷击的同时结算：下一份计划。
- 特殊攻击每场只发动一次、参加者之后照常出手：参考是这样，我方现在是每轮都可判定、参加者跳过，这次不动。
- 对方有陆上单位时潜水艇进入炮击名单（`hasInstall`）。
- 联合舰队目标在主力和护卫之间的 39% 分配（`doShellC`），沿用现有选靶。
- 港口画面上 `api_leng` 不含装备射程：只在排炮击顺序时现算，不改舰船状态的计算。

## Implementation Units

### U1 一艘舰的一手

`shelling.rs`：把 `simulate_shelling_side` 循环体提成「这艘舰打一手，结果追加到同一个 `BattleHougeki`」。
`simulate_shelling_side` 留给现有单元测试用（`#[cfg(test)]`），内部走同一个函数。

### U2 一轮

`shelling.rs` 新增一轮的驱动：收我方、敌方各自的出手范围和排序方式（射程 / 位置），建名单，交替调用 U1，
在旗舰那一手判定特殊攻击。下标直接写成整支舰队里的位置，`mod.rs` 的 `shift_*` 与 `merge_hougeki` 随之删除。
测试：没有战舰时敌方也开炮；交替顺序；射程长的先出手；第二轮按位置；被击沉的舰不出手；一方全灭即停。

### U3 三条路径接上

`mod.rs`：`execute_shelling1/2`、`execute_combined_shelling`、`execute_enemy_deck_shelling` 改调 U2。
更新这些函数和 `simulate_day` 上说明旧方案的注释。

### U4 基线与验证

重新冻结 `crates/emukc_battle/tests/golden/*.txt` 与 `tests/gameplay_tests/battle_golden.rs`；
因顺序变化失败的带种子测试换种子，不放宽断言。重跑 30 个种子的统计。

### U5 沉淀

`docs/solutions/architecture-patterns/` 记一篇炮击顺序；`docs/battle/combined-fleet-reference.md` 里若写了
按航速定先手则改掉；`TODO.md` 那一条勾掉；回写 `PROJECT_MEMORY.md`。

## 实施记录（2026-10-10）

- U1–U3 按计划做完；`merge_hougeki`、`shift_enemy_indices`、`shift_friendly_attackers`、`fleet_speed`、
  `enemy_shells_first` 删除，`execute_shelling1/2` 合成 `execute_shelling`。
- 基线：20 个昼战文本基线重新冻结（夜战的 20 个没变）；`battle_golden.rs` 重新冻结——同射程的两艘舰现在
  洗牌定先后，种子 1 下 F2 先出手并暴击击沉，F1 不再出手，MVP 由 F1 变 F2。
- 换种子：`sortie_battle.rs` 的 `DROP_SEED` 1→3。
- 30 个种子的敌方炮击次数：`leveled_for_mid_boss` 0→122，`transport_5_6` 0→159，`gunnery_cutin` 106（两轮都有双方）。
- 无头检查 `fresh_1_1`、`transport_5_6` 通过；用 `test/headless-fair-battle` 分支上的 `gunnery_cutin`（不开无敌）
  跑了两次也通过，包里 `api_at_eflag` 是交替的。
- `combined-fleet-reference.md` 没有按航速定先手的说法，不用改。`TODO.md` 里那一条在另一个未合并的分支上。

## Stop Conditions

- 交替之后某条联合舰队路径需要改变「哪一轮写进哪个 `hougeki`」：停下来问。
- 客户端对交替后的包报页面错误：停下来查，不绕过。

## Verification

- `cargo fmt --all --check`；`cargo clippy --workspace --all-targets -- -W warnings` 仍是 17 条；
  `cargo test --workspace --exclude emukc_time --no-fail-fast` 全过，无忽略。
- `battle sim` 九个预设种子 1 都能打完；30 个种子里 `leveled_for_mid_boss`、`transport_5_6` 的敌方炮击次数不为 0。
- 无头检查 `fresh_1_1`、`transport_5_6` 通过。
