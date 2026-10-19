---
title: "Smoke Screens - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Smoke Screens - Plan

Issue #199。来源 `kancolle-replay@69097abc`：`kcsim.js:3405` `getSmokeType`（`:3635`、`kcsimcombined.js:47/742/1158` 调用：交战形态之后、各阶段之前，单舰队要 4 艘在场）；命中 `:536`（炮击，按雷达分行）、`:994`（对潜）、`:1882`（雷击）、`:1992`（航空，全 1）；`:2391`、`:2410`（被烟幕的舰队对空只剩固定击坠）；`:2590`（对空弹幕，未建模）；`:3532` `api_smoke_type`。

**前提与用户决定（2026-10-19）**：来源默认 `smokeChance: [0,0,0]`、各 `smokeMod*` 为 1，照默认永不生效。用户选：发动率用 wikiwiki「発煙装置(煙幕)」発動率推定式（Pg0–Pg3；n≥3 系数「0.04~0.045?」取 0.04），命中倍率用来源的 `SIMCONSTS.smokeEst`。wiki 的多重率表（Pm）与 Pg 表重叠，Pg 已是完整分布，不叠用。

客户端（`main.decoded.js`）：阵形画面的煙幕按钮 → 昼战请求 `api_smoke_flag=1`；`next` 的 `api_limit_state == 1` 隐藏按钮（`CellTaskBattle._isEnabledSmoke`，另要求有未退避的煙幕装备舰、非开幕夜战格）；`api_smoke_type` 1–3 画一到三层烟（`common.smoke_type`）。

**基线**：`feat/pt-imps`（#197）之上。
**范围**：U1–U3，一个 PR。
**非目标**：Pm 表；航空命中（来源为 1）；对空弹幕；雷达射击格（客户端不给按钮，服务器不另查）；演习（客户端不发）。

## Implementation Units

### U1 战斗

- `smoke.rs`：`roll`（闸门：有在场舰带 500/501，单舰队 4 艘在场；否则不抽）、`rates`、`accuracy`（`smokeEst` 表）。
- `BattleContext.use_smoke`；`simulate_day` 在 `from_context` 之后、任何阶段之前抽签，`BattleState::lay_smoke` 把等级记到己方各舰（`BattleRuntimeShip::smoke`）与封包 `smoke_type`。
- `hit_value`：攻防任一方在烟中即乘倍率，在 PT 修正之前；夜战不乘，但同格夜战的对潜沿用（来源行为）。
- `kouku.rs` `fly_through_anti_air`：烟中的防守舰比例与固定以外的击坠为 0，两次半数抽签照抽。
- transcript：有烟时多一行 `smoke screen: N`（无烟时不变，冻结的 transcript 不动）。

### U2 出击

- `SortieOrder { formation_id, smoke }`，`From<i64>`；十个昼战入口改收 `impl Into<SortieOrder>`，67 处既有调用（含冻结的 `battle_golden.rs`）不改。
- 十个 handler 读 `api_smoke_flag`。`ActiveSortieState::smoke_used`：发动成功才置位；`next` 回 `api_limit_state`。
- 预设 `smoke_screen`（1-1，四艘驱逐，旗舰带 501+500，n=3 必发动），`PhaseExpectation::SmokeScreen`；sim→validate 门与 `battle sim` 按预设下令放烟。

### U3 测试与文档

- 单测：发动率表；闸门不抽签；各攻击倍率；烟在 PT 修正之前；烟中对空只剩固定击坠；昼战封包带等级。
- 集成：`a_smoke_screen_is_laid_once_a_sortie`（第一战发动、`limit_state` 1、第二战不再发动；不下令则不发动）。
- 无头：`make headless-check SCENARIO=smoke_screen`。
- 文档：新增 `smoke-screen.md`；`night-attack-types.md`、`carrier-night-air-attack.md`、`battle-hit-and-critical-rolls.md`、`accuracy.rs`、`headless-client-check.md`、CLAUDE.md 场景表。
- golden：180 份与 `battle_golden.rs` 未变（都不下令放烟）。
