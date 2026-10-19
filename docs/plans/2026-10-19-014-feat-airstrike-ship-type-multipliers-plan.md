---
title: "Airstrike Ship-Type Multipliers - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Airstrike Ship-Type Multipliers - Plan

Issue #263。来源 `kancolle-replay@69097abc`（`hayabusa65Buff`、`enableSkipTorpBonus` 默认开）：`kcsim.js:3243-3276` 按机体的 `planebase`；`kcsim.js:3286-3295` 航空队跳弹 `preMod`；`kcsim.js:1983-1988`、`2042-2051` 舰载机空袭跳弹命中与 `preMod`；`kcEQDATA.js` `isSkipBomber`（459、1625、1626、1634、1635、1636）。

**基线**：`main`（#266 之后）。
**非目标**：喷气阶段；来源对特定敌舰的命中乘数（1557、1586 等 ×1.1）。

**范围补充**：issue 只列了航空队的伤害；同一函数里的按机体命中（`kcsim.js:3183-3206`）实施中一并加入（U4），否则 B-25 只对一半。

## Implementation Units

### U1 航空队按机体数值

- `air_base.rs` `strike_stat` 增 `ship_type`：选好基础数值后、`with_airstrike_improvement` 之前套 224/491/405/406/444/454/484/562 各自的覆盖、乘数或加值（舰种按 codex stype，来源 `type` 与之逐一对应，PT小鬼群是 DD）。潜艇照旧提前返回。

### U2 跳弹 preMod

- `kouku.rs` 新 `is_skip_bomber`、`skip_bombing_pre_cap(on_land, ship_type)`，供两处共用；来源 `AT` = stype 15（7 艘，与来源一致）。
- 航空队在 `strike_power` 乘进 `pre`；舰载机在艦攻 0.8/1.5 之后乘进 `pre`。

### U3 舰载机跳弹命中

- `kouku.rs` `skip_bombing_accuracy`：`execute_airstrike_phase` 给 `AIRSTRIKE_HIT_PERCENT` 加 31/22/18/14 点，DD 是 PT 时不加。

### U4 航空队按机体命中

- `air_base.rs` `strike_accuracy_bonus`：444/484/453/454 按舰种加减点数（只看 type，不看是否陆上）；跳弹对地 −9，对海复用 `skip_bombing_accuracy`（驱逐 13，舰载机 14）。加在 PT 乘数之后。

### 验证

- 单测：405、224 的数值；B-25 与 454 的命中点数；B-25 对 DD/BB/对地；深海双発陸爆 对 DD 的 ×1.9 与命中加点。
- golden 与 gameplay 测试未变（默认场景不带这些机体）。
- 文档：`air-corps.md`、`battle-kouku-stage3.md`。
