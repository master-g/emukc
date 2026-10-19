---
title: "PT Imps - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# PT Imps - Plan

Issue #197。来源 `kancolle-replay@69097abc`，默认开关（`NERFPTIMPS`、`BREAKPTIMPS` 关，`enableAirstrikeSpecialBonus` 开）：`kcsim.js` `shell` `:527`/`:547-555`/`:608-611`、`NBattack` `:810`/`:830-839`/`:878-881`、`torpedoPhase` `:1888-1894`/`:1925-1927`、`airstrike` `:2058`、`airstrikeLBAS` `:3180-3182`/`:3303`，targeting `:1112-1118`、`:1580-1584`、`:1744-1747`；`kcships.js:33-42`（`isAntiPT`）、`:967-996`（`ptDmgMod`、`ptAccMod`）。

issue 写的 `:2108`、`:3352` 只在 `enableAirstrikeSpecialBonus` 关时走；默认开时走特殊加成链里的 PT 分支（`:2058` 0.8/0.5，`:3303` 0.7/0.4）。链上其它目标归 #200。

**基线**：`fix/air-corps-combined-bonus`（#251）之上。
**范围**：U1–U3，一个 PR。
**非目标**：skip bomber 对驱逐的加成（`:1987`、`:3205`，未建模）；`NERFPTIMPS`/`BREAKPTIMPS`；海防艦命中 −13（与 PT 无关的既有缺口）。

## Implementation Units

### U1 `pt_imp.rs`

- `AntiPt::of`：`ptAccMod`、`ptDmgMod`。来源装备 `type` 与 `api_type[2]` 对本处用到的 1、4（含 95）、7、11、21、39、45、57 一一对应（`kcEQDATA.js` 逐件核对，`MAINGUNSAA` 无成员）。
- `pt_accuracy`、`pt_power`、两个特殊加成抽签；`mark_anti_pt`（903/908 64，同队相邻 DD/DE 32）、`prefer_pt`。

### U2 接入

- `accuracy.rs` `hit_value`：昼夜炮击把 `pt_accuracy_flat` 加进求和；末尾按种类变换（炮击 ×ptAccMod、警戒陣 ×1.2；夜战再 ×0.7；雷击 ×0.7）。
- `damage.rs`：post-cap 之后、暴击之前，炮击 `pt_power × ptDmgMod`，夜战 `× 0.6 × ptDmgMod`，雷击 `pt_power`。
- `kouku.rs`：鱼雷机抽签之后抽 PT 倍率；`air_base.rs`：命中 ×0.95（B-25 ×0.85），命中时抽 PT 倍率。
- `targeting.rs`：昼夜炮击候选收窄到 PT；敌连合按 0.39 抽完后改去只有 PT 的那层。`BattleState::from_context` 取 `&Codex` 以标记反 PT 舰。

### U3 测试与文档

- 单测：变换公式；装备与舰种的倍率；天霧改二与相邻驱逐的标记与选靶；昼炮击、夜战、雷击的命中与威力；航空战两档抽签。
- 文档：新增 `pt-imps.md`；`accuracy.rs`、`battle-hit-and-critical-rolls.md`、`night-attack-types.md` 去掉「PT 未建模」。
- golden：180 份与 `battle_golden.rs` 未变（golden 场景没有 PT）。
