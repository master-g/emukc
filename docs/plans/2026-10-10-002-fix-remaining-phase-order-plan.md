---
title: "Opening ASW, Night Battle and Torpedoes: Who Acts When - Plan"
type: fix
date: 2026-10-10
status: implemented
execution: code
---

# Opening ASW, Night Battle and Torpedoes: Who Acts When - Plan

## Problem

昼战炮击改成双方逐舰交替之后（计划 `2026-10-10-001`），还有三处是「我方整队做完，敌方再做」：
先制对潜、夜战、雷击。先做的一方能在对方出手前把对方打沉或打到不能出手，三处都偏向我方。

## Evidence

**我方现状（2026-10-10 读代码，`crates/emukc_battle/src/simulation/`）：**

| 阶段 | 位置 | 现状 |
|---|---|---|
| 先制对潜 | `asw.rs` `simulate_opening_taisen` | 我方按位置打完，敌方再按位置打 |
| 夜战 | `night.rs` `simulate_night_hougeki` | 同上；两个循环是同一段代码的两份 |
| 开幕 / 闭幕雷击 | `torpedo.rs` `simulate_opening_torpedo`、`simulate_raigeki` | 我方的鱼雷先结算，敌舰被击沉或打到不能雷击就不再发射 |

**参考：`KC3Kai/kancolle-replay` 的 `kcsim.js`：**

- 先制对潜（`sim` 3789–3804）：双方各把能先制对潜的舰按射程排好，走和炮击同一个交替循环 `shellPhase`。
- 夜战（`sim` 3884–3891，`nightPhase` 1693）：名单是整支舰队按位置，第 i 手是我方第 i 艘、敌方第 i 艘；
  轮到时查能不能夜战，任一方全灭即停。
- 雷击（`torpedoPhase` 1776）：先把双方能发射的舰和各自的目标收齐，再逐发结算。能不能发射、发射时的
  损伤状态都按阶段开始时算；被先结算的鱼雷击沉的舰，自己的鱼雷照样结算。

**客户端：** 对潜和夜战的包每次攻击自带 `api_at_eflag`，按数组顺序播放；雷击包按舰位存目标和伤害，
双方同时演出，没有先后。

## Decision

1. 先制对潜：双方各自的名单按射程排（同射程洗牌，和炮击第一轮同一个函数），逐舰交替，我方先手。
2. 夜战：按舰队位置逐舰交替，我方先手；任一方全灭即停。两个循环合成一个。
3. 雷击：敌方的发射资格、目标选择和攻击力按阶段开始时的敌舰状态算；伤害照旧依次落到活着的舰上。
   我方先结算这一点不变，所以随机数的消耗顺序不变，只是敌方发射的鱼雷变多。

## Scope

**做：** 上面三处，单舰队和联合舰队共用同一份代码，一起改。

**不做：** 夜战的照明弹、探照灯、夜间触接；夜战特殊攻击的时机；联合舰队夜战的分队顺序；
雷击目标在联合舰队主力与护卫间的 35% 分配。

## Implementation Units

### U1 先制对潜

`shelling.rs` 的排序函数接受「谁能出手」的条件后给 `asw.rs` 用；`simulate_opening_taisen` 改成交替循环。
测试：双方都能先制对潜时 `api_at_eflag` 交替。

### U2 夜战

`night.rs`：把一艘舰的一手提成函数，`simulate_night_hougeki` 按位置交替调用。
测试：`api_at_eflag` 交替；被击沉的舰不出手。

### U3 雷击

`torpedo.rs`：两个函数在我方结算前留一份敌舰的快照，敌方循环用快照判断资格、选目标、算攻击力。
测试：被我方鱼雷击沉的敌舰仍然发射。

### U4 基线、验证、沉淀

重新冻结文本基线与 `battle_golden.rs`，换种子不放宽断言；`day-shelling-order.md` 改成覆盖所有阶段的顺序；
回写 `PROJECT_MEMORY.md`。

## 实施记录（2026-10-10）

- U1–U3 按计划做完。`shelling_order` 改名 `firing_order` 并接受出手条件；夜战两份循环合成 `night_turn`；
  雷击两处各加一份开始时的敌舰快照。
- 基线：18 个文本基线重新冻结（夜战全部、带雷击或先制对潜的昼战）；`battle_golden.rs` 没变
  （那场战斗没有雷击和夜战）。没有需要换种子的测试。
- 新增三个测试：对潜交替、夜战交替、被击沉的敌舰仍发射鱼雷。
- 无头检查：`fresh_1_1`、`transport_5_6` 通过；不开无敌的 `gunnery_cutin` 三次通过但都在昼战结束；
  另用六艘驱逐（`leveled_for_mid_boss`）不开无敌打 2-1 三次，三次都进了夜战，全部通过，
  夜战包的 `api_at_eflag` 是交替的，回港血量与包一致。

## Stop Conditions

- 客户端对交替后的夜战包或对潜包报页面错误：停下来查。
- 联合舰队夜战的现有测试表明分队顺序依赖「我方先整队打完」：停下来问。

## Verification

- fmt；clippy 17；全量测试全过无忽略。
- `battle sim` 九个预设种子 1 都能打完。
- 无头检查 `fresh_1_1`、`transport_5_6`、不开无敌的 `gunnery_cutin`（含夜战）通过。
