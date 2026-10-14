---
title: "Sortie Quest Composition at Battle Time - Plan"
type: fix
date: 2026-10-14
status: draft
execution: code
---

# Sortie Quest Composition at Battle Time - Plan

Issue #114（ready-for-human）。239 条「编成 + 出击」任务现在不看出击那一刻的编成：任何舰队都能把出击计数刷满，
领奖前换上要求的舰就算完成。issue 的 brief 留了三个设计问题，这份计划给出决定，定下后可以交给 agent 实施。
依赖计划 2026-10-14-001 的 U2/U3 先落地（都改 `SortieBattleCompleted` 与 `apply_sortie_event`）。

## Evidence

- **数据形状**（码典 `quest.json`，含 `Composition` 的 337 条）：
  - `And[Composition, Sortie…]` 233 条：一个编成，后面一到多个出击条件（如 206、219、Bm 系列）。
  - `OneOf[Composition, Sortie, Composition, Sortie…]` 5 条（237、903、997、1014、1019）：编成与出击成对交替，
    任一对完成即可；237 只有一对。
  - `Sequential[Exercise, Composition, Sortie]` 1 条（317）：先演习，再以该编成出击。
  - 纯 `And[Composition]` 96 条；`And[Composition, Consumption, Scrap]` 2 条：编成是「领奖时持有」的状态条件。
- **现在的机制**：`apply_sortie_event`（`thirdparty/quest/matcher.rs`）只比舰队号、海图、boss、评价。编成由
  `validate_composition_quests`（`game/quest/update.rs:350`）在取任务列表、接取时按**当前舰队**判：满足且计数都
  完成就置 `Completed`，不满足且已 `Completed` 就打回 `Empty`。`Composition::is_satisfied` 硬编码为 `false`
  （`thirdparty/quest/progress.rs:55`），所以含编成的 And 任务**只能**靠 `validate_composition_quests` 到达
  `Completed`。
- **演习已经是对的**：`ExerciseBattleCompleted` 带 `fleet_ships: Vec<FleetShipSnapshot>`（`game/practice.rs:259`），
  matcher 用 `validate_composition_snapshot(fleet_id, ships, condition, codex)`（`thirdparty/quest/composition.rs:64`）
  判演习条件里的 `groups`。
- **实机的判定时机**：KC3Kai 在 battleresult 处理里，对带编成要求的任务调
  `KC3QuestManager.isPrerequisiteFulfilled`，用 `KC3SortieManager.fleetSent` 那支舰队判
  （`src/library/modules/Kcsapi.js` 约 3506 行、`QuestManager.js` 约 685 行，commit `6b0534d2`）。也就是**每场战斗
  结算时、对出击的那支舰队**。领奖时不再检查编成。

## Decision

1. **判定时机：每场战斗结算时，对出击舰队当时的舰**。`SortieBattleCompleted`（outcome 与 event）加
   `fleet_ships: Vec<FleetShipSnapshot>`，结算时从出击舰队的舰船构造（位置从 1 起，与演习快照一致）。
   联合舰队按主队（第 1 舰队）判，编成条件的 `fleet_id` 为 1 或 0 时都对主队。
2. **配对规则：每个 `Sortie` 条件受它前面最近的那个 `Composition` 约束**；前面没有 `Composition` 的 `Sortie` 不受约束。
   这一条覆盖上面全部 239 条：And 里一个编成管后面所有出击；OneOf 里每个编成管紧随其后的出击；317 里编成管出击，
   不管在它之前的演习。配对在运行时按 **master 条件列表**的下标算，不改码典形状，旧进度行不用迁移。
   写成 `emukc_model` 里的纯函数（如 `paired_composition(conditions, sortie_idx) -> Option<&Composition>`），
   matcher 一侧与进度计算一侧共用。
3. **配对的 `Composition` 随它的 `Sortie` 算满足**：进度计算（`Kc3rdQuestRequirement::calculate_progress` 与
   `progress_after_event` 用到的判断）里，一个被配对的 `Composition` 在它所配的**全部** `Sortie` 都满足时算满足。
   未配对的 `Composition`（纯编成任务、编成+消耗+解体）照旧由 `validate_composition_quests` 判。
   `validate_composition_quests` 跳过所有 `Composition` 都已配对的任务：既不置 `Completed`，也不打回。
4. **出击事件的匹配**：`update_quest_progress_for_action` 对 `Sortie` 条件，先取配对的 `Composition`，有则用
   `validate_composition_snapshot(event.fleet_id, &event.fleet_ships, comp, codex)` 判，不满足就不推进。
   计划 001 引入的 `SortieStarted`（只给 Bw1 首项，Bw1 没有编成）不需要快照。

## Implementation Units

- **U1 快照**：`SortieBattleCompleted` 加 `fleet_ships`，结算处构造；`build_sortie_battle_outcome_*` 单元测试补字段。
- **U2 配对与匹配**：配对函数及其单元测试（And 单编成多出击、OneOf 成对、317 的 Sequential、无编成）；
  `update_quest_progress_for_action` 在推进 `Sortie` 前检查配对编成。
- **U3 进度与列表校验**：进度计算里配对编成的满足规则；`validate_composition_quests` 跳过全配对任务。
  取列表时的 `recalculate_quest_progress` 也走 `calculate_progress`，改那一处即可覆盖两条路径。
- **U4 测试** `tests/gameplay_tests/quest/`：206 用错舰队出击不推进，事后换对舰队也不完成；用对舰队出击完成，事后
  换掉舰队仍是 `Completed` 且能领；997 用第二种编成完成对应的出击；101（纯编成）行为不变；317 演习后以编成出击
  完成。`composition.rs` 现有的 `test_composition_exact_match_requirement` 只测纯编成，预期不受影响；若有用例按
  「当前舰队」断言混合任务，改预期并在实施记录里说明。
- **U5 文档**：`quest.md` 的 progress tracking 改写编成一条（判定时机、配对规则、来源），删掉「Composition
  validation … after fleet changes」里对混合任务的描述。

U1 → U2 → U3 → U4。

## Risks

- **改变的是已有存档里进行中的任务**：升级前用错舰队刷到一半的计数不会回退，只是之后的战斗要用对的舰队。
  已 `Completed` 的混合任务升级后不再被 `validate_composition_quests` 打回。都是放宽或保持，不会让已完成的变成
  未完成。
- **`OneOf` 的进度**：配对之后 OneOf 的满足判断是「任一对的编成与出击都满足」，要确认现有 OneOf 的进度计算按对
  计数而不是按单个条件计数；不是的话 U3 一并修正。
- **舰船中途变化**：大破撤退（退避）的舰不在快照里，可能使后续战斗的编成不满足。KC3Kai 用的是出击时的舰队对象，
  实机细节没有一手来源；按「战斗时舰队里的舰」做，列为已知不确定。
- **与计划 001 的冲突面**：两者都改 `SortieBattleCompleted` 与 `apply_sortie_event`，必须在 001 合并后再开工。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test --workspace --no-fail-fast`，报告 ignored 数；`battle_golden.rs` 不应变化（只加了 outcome 字段）。
- 可选：`make headless-check SCENARIO=fresh_1_1` 确认出击流程不受影响。
