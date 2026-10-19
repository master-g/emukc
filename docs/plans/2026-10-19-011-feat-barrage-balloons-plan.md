---
title: "Barrage Balloons - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Barrage Balloons - Plan

Issue #198。先查常规图有没有气球格：有。wikiwiki「阻塞気球」：常规图 3-5-H、4-3-N、4-5-T、6-4-N 确认会浮起，7-5-Q 不会；来源 `js/simulator-ui/common.js` `BARRAGE_BALLOON_NODES` 列 3-5-H、4-3-N、4-5-T（其余为活动格）。两者取并集。KCNav 常规图敌编成里没有 1640。

来源 `kancolle-replay@69097abc`：`kcships.js:329` `getNumBalloons`（格子有效时，舰队含连合两层在场带 513/1640 的舰数，至多 3）；`kcsim.js:561-563`（炮击 `postModCI`/`postModExtra` ×(1+n/50)）、`:1006`（昼间对潜）、`:2583-2584`（航空战接触 ×(1+n/50)×(1−m/20)）、`:3099-3102`（基地航空队同）、`:3531` `api_balloon_cell`；命中项 `balloonSelf*`/`balloonOppo*` 默认 1/0。`sim-interface.js:984-985`：两侧舰队都取格子标志。

**基线**：`feat/airstrike-special-bonus`（#200 U1）之上。
**范围**：U1–U2，一个 PR。
**非目标**：默认无效的命中项；夜战封包的 `api_balloon_cell`（发 0，客户端夜间不画）；中途击沉后的重新计数（ponytail 记在 `raise_balloons`）。

## Implementation Units

### U1 战斗

- `BattleContext.balloon_cell`；`simulate_day` 开场 `raise_balloons`：两侧各舰记本舰队的气球数，封包 `balloon_cell` 置 1。
- `damage.rs`：`balloon_power` 进昼战炮击（CI 时与 CI 倍率同乘、在 AP 取整前，否则在 AP 之后）与对潜；`balloon_contact` 进航空战与基地航空队的接触倍率。夜战开场清零。
- 单测：计数（在场、上限 3、敌方 0、封包）；炮击两种位置；对潜；接触倍率。

### U2 出击

- `setup.rs` `BALLOON_CELLS`（地图 id + 节点字母），`battle_input` 带入。
- 测试：`balloons_rise_at_3_5_h`（3-5 H 昼战 `api_balloon_cell` 1 且通过协议校验，其他战斗格 0）；`balloons_rise_on_the_four_regular_cells`。
- 文档：新增 `barrage-balloons.md`；`accuracy.rs` 与 `battle-hit-and-critical-rolls.md` 的未建模清单去掉气球。
- golden：180 份与 `battle_golden.rs` 未变（都不在气球格）。
