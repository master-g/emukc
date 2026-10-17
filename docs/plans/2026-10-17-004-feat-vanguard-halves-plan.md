---
title: "Vanguard Front and Rear Halves - Plan"
type: feat
date: 2026-10-17
status: implemented
execution: code
---

# Vanguard Front and Rear Halves - Plan

Issue #195。来源 `kancolle-replay@69097abc`：`kcships.js:1342` `getFormation`，`kcsim.js:6-7` `VANGUARD1`/`VANGUARD2`，`vanguardUseType: 2` 的 `*Mod` 行（220–225），`shell`（500）、`NBattack`（707、797）、`ASW`（980）、`torpedoPhase`（1852–1875）、`choiceWProtect`（2306）。

**范围**：U1–U3，一个 PR。
**非目标**：`vanguardUseType` 1 与 `*Event` 行；对空（两半都是 1.1）；联合舰队（不用阵形 6）；支援射击（本仓库没有）。

**基线**：`main` `460ea895`。

## Implementation Units

### U1 站位

- `BattleRuntimeShip.vanguard: Option<VanguardPlace { number, fleet_len }>`：仅当舰队阵形为 6 时在 `BattleState::from_context` 设置；序号从 1 起，沉没、退避的船保留序号，`fleet_len` 是舰队长度。
- 前半：`number <= fleet_len / 2`。

### U2 效果

- 攻击方命中（`roll_attack`）：前半 炮击 0.8、雷击 0.7、夜战 0.8、对潜 1；后半 1.2、0.9、1.2、1.1（现有的阵形 6 行）。
- 攻击方威力：前半炮击 ×0.5、后半对潜 ×0.6（昼战），雷击两半都是 1；前半夜战 pre-cap ×0.5。
- 防守方回避：目标舰队为 警戒陣 时，攻击命中再乘目标站位那一行（第 1–7 位，越界为 1）：炮击/夜战/对潜 驱逐舰 `[.95,.95,.8,.8,.69,.64,.64]`、其余 `[.95,.95,.95,.95,.86,.79,.7]`；雷击 驱逐舰 `[.9,.9,.65,.58,.5,.42,.42]`、其余 `[.9,.9,.77,.67,.63,.55,.51]`。不受「阵形克制」影响。
- 来源有、简报未列（用户 2026-10-17 选定纳入）：警戒陣 舰队打驱逐舰，雷击命中 ×1.2、炮击与夜战命中 ×1.1（`shell` 446、`NBattack` 708、`torpedoPhase` 1867），整支舰队都算。
- 抽目标（`choice_with_protect`）：探照灯重抽之后、かばう 之前，抽中的目标若在 警戒陣 前半，从同一候选再抽一次。

### U3 测试与文档

- 单测：5/6/7 艘的前后半；生产路径设了站位；两半的命中行与威力；第 5 位 0.69/0.86、第 4 位雷击 0.58/0.67；前半夜战威力减半；前半重抽一次、后半不抽；DD 加成。
- golden 若动，按流程重冻并在 PR 说明。
- 删 `accuracy.rs` 阵形 6 的 ponytail 注释与模块注释里的「警戒陣 by position」；`battle-hit-and-critical-rolls.md` 同步。
