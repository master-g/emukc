---
title: "Sinking Protection on Current HP, and the Enemy's Own Air State - Plan"
type: fix
date: 2026-10-10
status: implemented
execution: code
---

# Sinking Protection on Current HP, and the Enemy's Own Air State - Plan

## Problem

不开无敌的无头检查里，旗舰赤城一场 2-1 从 69 掉到 1。查下来是两个问题：

1. 击沉保护把致命伤害换成比例伤害时，基数取的是进入战斗时的血量，再用「当前血量减 1」封顶。
   受保护的舰只要先挨过一下，第二次致命伤害必定剩 1。
2. 弹着观测射击和夜间触接的制空条件，对敌方用的是我方的制空状态：我方制空确保时敌方也能連撃，
   敌方拿到制空时反而不能。

## Evidence

**复现（2026-10-10，`battle sim --scenario gunnery_cutin --seed 38`）：** 我方制空确保；リ級elite 对赤城連撃，
第一发 64（69→5），第二发致命，结果 4（5→1）。

**我方现状：**

- `types/runtime.rs` `apply_damage`：`h = entry_hp`，`h/2 + rand(0..h)×3/10`，再 `min(current_hp − 1)`。
  `types/mod.rs` 有一个测试 `protection_uses_entry_hp_not_current_hp` 把这一点钉住了。
- `simulation/shelling.rs` 的一轮把同一个 `air_state` 给双方；`resolve_day_attack`（`day_cutin.rs:414`）
  要求 `Supremacy | Superiority`。`simulation/night.rs` 的 `night_turn` 同样把同一个状态给双方，
  `damage.rs` `night_recon_bonus` 按它给夜侦加成。`AirState` 是从我方视角算的（`from_power(friendly, enemy)`）。

**参考（`KC3Kai/kancolle-replay` `kcsim.js`）：**

- `takeDamage`（2121）：受保护的舰血量为 1 时伤害为 0；伤害不小于**当前血量**时换成
  `floor(当前血量×0.5 + 0.3×floor(rand×当前血量))`。这个值恒小于当前血量，不需要封顶。
- 谁受保护没有变：旗舰始终受保护，其他舰以进入战斗时是否大破为准，我方现在的判断是对的。
- 敌方舰队的制空状态是我方的相反数（`fleet.AS`）。

## Decision

1. 比例伤害的基数改成当前血量，公式 `(5×当前血量 + 3×rand(0..当前血量)) / 10` 取整；血量为 1 时为 0。
   去掉封顶。谁受保护的判断不动。
2. `AirState` 加一个取对方视角的方法（确保↔喪失、優勢↔劣勢、均衡不变）；昼战炮击和夜战里敌方出手时用它。

## Scope

**不做：** 敌方能否弹着观测的其他条件（索敌、水侦存活）；应急修理要员；演习里的伤害规则。

## Implementation Units

- **U1** `apply_damage` 与它的文档注释；改写钉住旧行为的测试，加一个「先挨一下再挨致命一击不会剩 1」的测试。
- **U2** `AirState::reversed`；`simulate_shelling_round` 与 `simulate_night_hougeki` 给敌方传反转后的状态；
  测试：我方制空确保时敌方不出弹着观测。
- **U3** 重新冻结基线，换种子不放宽断言；`battle-hit-and-critical-rolls.md` 旁记一笔或在 `day-shelling-order.md`
  之外另写一条；回写 `PROJECT_MEMORY.md`。

## 实施记录（2026-10-10）

- U1、U2 按计划做完。钉住旧行为的测试 `protection_uses_entry_hp_not_current_hp` 改写成按当前血量断言，
  另加「第二次致命伤害不会剩 1」和「敌方只在自己的制空状态下弹着观测」两个测试。
- 基线：13 个文本基线重新冻结；`battle_golden.rs` 没变。没有需要换种子的测试。
- `gunnery_cutin` 种子 38：赤城 69→69，敌方不再連撃；30 个种子里敌方的切入次数从有到 0（我方每场都是制空确保）。
- 文档：`night-battle-sinking-protection.md`、`docs/battle/damage-formula-reference.md` 里的旧公式改掉；
  制空状态的视角记在 `day-shelling-order.md`。
- 无头检查 `gunnery_cutin`、`leveled_for_mid_boss` 通过。

## Verification

- fmt；clippy 17；全量测试全过无忽略。
- `battle sim --scenario gunnery_cutin --seed 38` 里赤城不再剩 1，敌方不再連撃。
- 无头检查 `gunnery_cutin`、`leveled_for_mid_boss` 通过。
