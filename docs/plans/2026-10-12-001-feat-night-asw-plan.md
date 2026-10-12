---
title: "Night Attacks on Submarines - Plan"
type: feat
date: 2026-10-12
status: implemented
execution: code
---

# Night Attacks on Submarines - Plan

规格在 GitHub issue #52（问题、用户故事、范围都在那里）。这份计划是实施之后补写的：实现直接照 issue 的
Implementation Decisions 做，这里只记 issue 里没有的东西——对来源核出的证据、规格里两处需要解释的地方、
实施单元与结果。

## Problem

夜战打潜艇是「必中、擦伤」，不抽命中，也不分昼战之后的夜战与开幕夜战。昼战之后的夜战比实机容易（每发必中），
开幕夜战的潜艇格比实机难（永远击沉不了）。计划 `2026-10-11-004` 把它列为独立的一份计划。

## Evidence

**我方现状（`crates/emukc_battle/src/simulation/night.rs` `night_turn`）：** 目标是潜艇时 `HitOutcome::Hit`，
伤害 `calculate_scratch_damage`；攻击种别固定 `Normal`，不算夜间航空攻击。夜战的输入不知道自己是不是开幕夜战。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`，`js/kcsim.js`）：**

- `ASW`（975–1037）的命中：`hitRate(ship, 80, sonarAcc, accMod)`，`sonarAcc` 是每个声纳対潜的两倍，`accMod` 是士气，
  陣形不被克时再乘 `ASWacc`（没有这一列的陣形用 `shellacc`）；回避用对方陣形的 `ASWev`；暴击系数 1.3；
  `usePlaneProf = planeasw && !isOASW && 非正规空母、非補給艦`。
  这几项与我方 `roll_attack` 的 `AttackKind::Asw`（非先制）逐项相同，夜战可以直接复用。
- `ASW` 的威力：`premod = isNightDmg ? 0 : ASWmod * ENGAGEMENT * damageMod()`。威力为 0 时 `damageCommon`
  落到擦伤，而且只在命中时才进 `damageCommon`，所以打空不抽擦伤。
- 夜战循环（1716、1729）对它的调用：`ASW(ship, target, (!NBonly && !ship.isescort), APIhou, false, true)`。
  威力置 0 的条件是「不是开幕夜战，且不是护卫舰队」。
- `isNightActual` 只关掉阻塞気球的补正。
- 报文：`api_sp_list` 0、`api_n_mother_list` 0。打空时来源的 `api_cl_list` 仍写 1（`(res>1)?2:1`）；我方昼战对潜写 0，
  夜战跟昼战一致。

## Decision

1. **命中复用昼战对潜的抽签**：`Aim::new(AttackKind::Asw, ..)`，不带 `in_the_opening`，熟练度因此照
   `asw_proficiency` 生效。
2. **夜战的输入加 `night_start`**，表示这场是不是开幕夜战。为假：打空伤害 0，命中是擦伤。为真：
   `calculate_asw_damage`，与昼战炮击阶段打潜艇同一个调用。
3. **`night_start` 是格子的属性，与舰队编成无关。** `execute_sp_midnight` 一律置真，连合舰队也是；
   出击与演习的昼战后夜战一律置假。见下一节。
4. **报文的形状不变**，没有潜艇的夜战抽签与结果逐位不变。

## 规格里两处需要解释的地方

- **「连合舰队保持标记为假」。** 读作连合舰队**昼战之后**的夜战不在这次范围：来源里那是真伤害（`!isescort`），
  我方仍是擦伤，记入已知差异。连合舰队进开幕夜战格在来源里因 `NBonly` 就是真伤害，置假反而偏离来源，
  所以不照字面做。用户已确认这个读法，issue 下有留言。
- **「展示装备取爆雷，与现在相同」。** 现状不是爆雷：夜战的普通攻击取 `is_day_surface_display_type` 的第一件，
  带双魚雷加爆雷的驱逐舰实测发 `-1`，昼战对潜的现有测试也是 `-1`。按「与现在相同」处理，不改展示逻辑；
  客户端按目标是潜艇来选爆雷或舰载机的演出（`day_attack.rs` 的注释）。

## Scope

**不做**（issue 的 Out of Scope）：连合舰队昼战后夜战的对潜、夜間瑞雲夜戦カットイン对潜艇的例外、
非空母舰种「素対潜」与「含装备的対潜」之差、烟幕与阻塞気球。

**沿用而不修的既有差异**（昼战对潜本来就有）：对潜伤害没有弹药补正；命中没有改修项、警戒陣按位置的回避、
`bonusSpecialAcc`；打空时也抽一次装甲乱数。

## Implementation Units

- **U1** `types/runtime.rs`、`types/domain.rs`、`simulation/mod.rs`：`NightBattleInput` 与 `NightBattleParams` 加
  `night_start`；`NightBattleParams::engagement` 开始被读。所有构造处补上字段（出击、演习、基线测试为假）。
- **U2** `execution.rs` `execute_sp_midnight`：置真。
  测试：单舰队与连合舰队在开幕夜战里都能把 30 HP 的潜艇击沉。
- **U3** `simulation/night.rs` `night_turn`：Decision 1、2。
  测试（`simulate_night_hougeki` 一层，每条跑 300 个种子）：昼战后的夜战会打空，命中时潜艇至少剩 26/30；
  开幕夜战会打空也能击沉；带声纳打空得更少。猎潜的驱逐舰带双魚雷，断言只打一发、种别 0、母舰标记 0。
  旧的 `night_shelling_against_submarines_is_scratch_damage` 断言旧规则，由这三条取代。
- **U4** 文档：`carrier-night-air-attack.md` 加「A submarine at night」一节并改已知缺口；
  `battle-hit-and-critical-rolls.md` 的 Not modelled 改掉被推翻的那句；`PROJECT_MEMORY.md`。

## 实施记录（2026-10-12）

- U1–U4 做完，分支 `feat/night-asw`。没有带种子的测试移位，没有换种子。
- 两类基线都没有变化：`crates/emukc_battle/tests/golden/*.txt` 与 `tests/gameplay_tests/battle_golden.rs`
  的夜战里没有潜艇。
- 两路审查（规范、规格）没有阻断性问题。审查后改的：猎潜舰换成带切入装备的，覆盖「不出切入」；
  补连合舰队开幕夜战的用例；这份计划。没有改的：三处相似的 DD/SS 测试夹具（收拢要动无关文件）；
  `sortie/mod.rs` 里同名异义的 `night_start()`（返回下标，可另行改名）。
- 没有专门用例的：夜战对潜的熟练度、敌方打我方潜艇。两者走同一条路径，敌方回合的陣形元组是对调的。

## Risks

- **昼战之后的夜战变难打中。** 命中从必中降到对潜的命中率，潜艇在夜战里被削的血变少，这是规格要的。
- **开幕夜战的潜艇格变得会沉。** 开幕夜战格的敌编成里若有潜艇，胜败判定会随之变化。合并后查了码典的
  `map_catalog.json`：开幕夜战格只在 5-3、5-4、5-5、6-5，共 16 个格子条目，敌编成里含潜艇的权重是 0。
  所以敌方潜艇在开幕夜战被击沉这条路径在常规图上触发不到，也没有图可以拿来配无头检查；
  会遇到的是反过来的情形，我方带潜艇进这些格子，敌方按对潜公式打她。
- **`night_start` 被别的夜战规则复用时**，注意它不含「是不是连合舰队」，要那一层得另看 `combined*`。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings` 通过；
  `cargo test --workspace --no-fail-fast` 1334 过、0 败、0 ignored。
- 无头检查加了场景 `night_asw`（六艘潜艇打 2-1，敌方在夜战打我方潜艇）：`make headless-check SCENARIO=night_asw`
  跑了三次都过，客户端演了打空（cl 0、伤害 0）与命中擦伤，没有页面错误。这也是「敌方打我方潜艇」唯一的端到端证据。
  我方在夜战打敌方潜艇、开幕夜战格，没有用客户端跑过。
