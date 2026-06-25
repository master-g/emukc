---
title: "fix: Batch Craft Quest Progress — Diagnostic and Repair of Development-Quest Event Matching"
status: completed
type: fix
date: 2026-06-22
origin: openspec/changes/fix-batch-craft-quest-progress (translated during openspec sunset, see docs/plans/2026-06-22-001)
---

# fix: Batch Craft Quest Progress

## Summary

批量开发 (`api_multiple_flag=1`) 时，开发相关任务进度不推进。代码层面 `create_slotitem` 已对每个成功 item 调用 `update_quest_progress_for_action`，但用户报告任务计数不前进。本计划定位事件匹配、任务条件解析、或计数逻辑的具体断裂点并修复。

This is a diagnostic-first fix: add trace logging at three points in the quest-event chain, verify the quest manifest data, then repair the break point once diagnosed. It does not restructure the quest system.

## Resolution (2026-06-25): archived — bug does not reproduce

The reported batch-craft bug **no longer reproduces on current code**. Root cause was fixed
earlier by commit `22b61c9` ("fix: quest progress counting"). End-to-end factory→quest
integration tests (commit `234422f`, `tests/gameplay_tests/quest/factory_progress.rs`) confirm:
a batch of 3 successful crafts advances quest 607 from 3→0, a single craft advances 605 from
1→0, and a batch containing a failure counts only the successes (607: 3→1).

Disposition of the diagnostic-first units:
- **U1 (trace logs)** and **U2 (manifest verification)** are moot — there is no live break point
  to diagnose; tests above prove the chain works and the 605/607 `SlotItemConstruction` conditions
  match correctly.
- **U3.1 (repair)** was effectively completed by `22b61c9`.
- **U3.2 (`DevelopmentAttempt` failure event)** stays unbuilt — no quest counts failed attempts
  (YAGNI, per the plan's own conditional guard).
- **U4 (integration tests)** landed via `234422f`.

The original 2026-06-22 reconciliation (written before this verification) is preserved below.

## Reconciliation (2026-06-22)

**Status: 0/11 done — plan is fully fresh (0% complete, no drift).**

Verified against current code:

| Unit | Tasks | Done | Evidence |
| --- | --- | --- | --- |
| U1 diagnostic logs | 1.1, 1.2, 1.3 | 0/3 | 0 `tracing::` calls in `factory.rs`, `quest/update.rs`, `quest/matcher.rs` |
| U2 verify manifest | 2.1, 2.2, 2.3 | 0/3 | `quest.json` exists (1.39 MB) but no documented validation of the triggered quest's condition type/period/activation has been performed |
| U3 fix progress | 3.1, 3.2 | 0/2 | `DevelopmentAttempt` event type absent from codebase; repair (3.1) depends on undiagnosed U1/U2 |
| U4 tests | 4.1, 4.2, 4.3 | 0/3 | existing `tests/gameplay_tests/quest/event_matching.rs` covers matcher-level unit tests only, not factory→quest integration tests |

**Notable:** the event-matching logic in `Kc3rdQuestCondition::matches_event` (`matcher.rs:73`) correctly maps `Factory(SlotItemConstruction)` ↔ `SlotItemConstructed`. The four candidate break points remain unconfirmed — the diagnostic-first approach (U1+U2 before U3) is still the right execution order.

## Problem Frame

Users report that batch crafting (`api_multiple_flag=1`, which sets `upper=3` and consumes 3× materials) does not advance development-type quest progress. A code-level read shows `create_slotitem` already calls `update_quest_progress_for_action` for each successfully crafted item, yet the counter does not move.

The current code flow:

```
createitem handler (src/bin/net/router/kcsapi/api_req_kousyou/createitem.rs)
  ├── api_multiple_flag=1 时 upper=3
  ├── 随机生成 crafted_mst_ids (成功=id, 失败=-1)
  └── state.create_slotitem(pid, &crafted_mst_ids, &costs)

create_slotitem (crates/emukc_gameplay/src/game/factory.rs:129-140)
  ├── 扣除材料
  ├── 创建装备
  └── for id in mst_id.iter():
       └── if *id > 0:  ← 仅成功 item
            update_quest_progress_for_action(SlotItemConstructed { item_mst_id })

update_quest_progress_for_action (crates/emukc_gameplay/src/game/quest/update.rs)
  ├── 查询所有 activated quest
  └── 对每个 quest: condition.matches_event(event) → apply(event)
```

Four candidate break points (each must be confirmed or ruled out):

1. **事件类型不匹配.** `Kc3rdQuestCondition::Factory(SlotItemConstruction(count))` matches `QuestActionEvent::SlotItemConstructed`, but the quest manifest's condition may use a different type.
2. **失败开发不计数.** Some daily quests count development *attempts* (including failures); the current code only emits a success event.
3. **批量开发消费了一次材料.** With `api_multiple_flag=1` materials are ×3, but quest events fire only per successful item. If a quest expects "number of crafts" rather than "number of successes," a batch of 3 emits 0–3 events instead of 3.
4. **quest manifest condition 类型差异.** `Factory` vs `Factory2`, or the condition struct's initial `count` value is wrong.

## Requirements

- R1. The exact break point in the create_slotitem → update_quest_progress_for_action → matches_event chain is identified via trace logging, not by guesswork.
- R2. The specific quest ID the user triggers is verified against its Codex requirements definition — the root cause may be quest-manifest data, not code.
- R3. A single successful craft advances development-quest progress by exactly 1.
- R4. A batch of 3 successful crafts advances development-quest progress by exactly 3 (one event per item, not one per batch).
- R5. Failed crafts are handled correctly: either counted (if the quest expects attempts) or skipped (if it expects successes), with no silent mis-counting.
- R6. If a quest requires counting failed development attempts, a corresponding event type exists and is emitted by the createitem handler.

## Non-goals

- 不修改 quest 奖励系统.
- 不修改开发成功率计算.
- 不重构 quest 整体架构.
- 不修改 quest manifest 数据格式.

## Key Technical Decisions

- **KTD1. Diagnostic-first, data-before-code (Decision 1 + Decision 2).** Before changing any matching or counting logic, add `trace` logs at three choke points and verify the quest manifest data. The audit found four plausible break points; only one is the real cause. Changing code blindly risks "fixing" a non-cause and leaving the real bug latent. The three log sites are:
  - `create_slotitem` (`crates/emukc_gameplay/src/game/factory.rs`) quest-event loop — log each event's `mst_id`.
  - `update_quest_progress_for_action` (`crates/emukc_gameplay/src/game/quest/update.rs`) — log event type, matched quest list, per-quest counter delta.
  - `Kc3rdQuestCondition::matches_event` (`crates/emukc_model/src/thirdparty/quest/matcher.rs`) — log the match result.
  These three together isolate whether events are emitted, whether they reach the matcher, and whether the matcher accepts them.
- **KTD2. Verify quest manifest data before editing code (Decision 2).** Confirm the triggered quest ID, check its Codex `requirements` definition, confirm its condition type is `Kc3rdQuestConditionFactory::SlotItemConstruction`, and confirm its period (daily/weekly/once) and activated status. If the condition type in the manifest does not match what the matcher expects, the fix is a data correction, not a code change — this must be ruled out first.
- **KTD3. Repair is conditional on diagnosis (task 3.1).** Task 3.1 is deliberately open-ended: the repair depends on which of the four break points the diagnosis confirms. It may require (a) extending the event enum, (b) correcting matcher logic, or (c) correcting quest-manifest data. The plan does not prescribe which, because the diagnosis has not been run yet — doing so would be guessing.
- **KTD4. Failed-craft event type is conditional (task 3.2).** `DevelopmentAttempt` is only added if the diagnosis shows a quest that counts attempts (including failures). Adding it speculatively would add a dead event type. Task 3.2's "如果某些 quest 需要计算失败开发" guard enforces this conditionality.

## Risks & Dependencies

- **[Risk] 问题可能在 quest manifest 数据而非代码.** Mitigation: tasks 2.1–2.3 verify the data before any code change (KTD2). If the manifest condition type is wrong, the fix is a data correction committed separately.
- **[Risk] 不同 quest 可能需要不同事件类型.** A fix for one quest may not cover all development quests (e.g., `Factory` vs `Factory2` / type 11). Mitigation: the diagnosis (tasks 1.x, 2.x) records the exact condition types encountered; the repair (3.1) handles all confirmed types, not just the first one found.
- **[Dependency] Codex data.** Tasks 2.1–2.3 require a bootstrapped `.data/codex` to read quest definitions. Tests (tasks 4.x) are codex-dependent and skip cleanly if absent.
- **[Dependency] Diagnostic logs must land before repair.** Tasks 1.x must ship before 3.1, otherwise the repair has no diagnosis to act on.

## Behavioral notes

The `quest-factory-events` capability delta from the source openspec change requires:

- 批量开发 (`api_multiple_flag=1`) 必须正确推进开发类任务进度.
- 每个成功开发的 item 必须触发独立的 quest progress update.
- 开发类任务的 condition 必须正确匹配 `SlotItemConstructed` 事件.
- 如有需要，开发失败也应有对应事件类型.

The migrated capability contract for the broader quest event/matcher architecture lives at `docs/solutions/architecture-patterns/quest.md` (migrated from `openspec/specs/quest/spec.md` during the openspec sunset — see `docs/migration/openspec-sunset-log.md`). This plan's changes must remain consistent with that contract's event-matching invariants.

## Implementation Units

### U1. 添加诊断日志 (Add diagnostic logging)

- **Goal:** Isolate the exact break point in the quest-event chain via trace logs at three choke points (KTD1).
- **Requirements:** R1.
- **Dependencies:** none.
- **Files:**
  - `crates/emukc_gameplay/src/game/factory.rs` — `create_slotitem` quest-event loop.
  - `crates/emukc_gameplay/src/game/quest/update.rs` — `update_quest_progress_for_action`.
  - `crates/emukc_model/src/thirdparty/quest/matcher.rs` — `Kc3rdQuestCondition::matches_event`.
- **Tasks:**
  - [ ] 1.1 在 `create_slotitem` (`crates/emukc_gameplay/src/game/factory.rs`) 的 quest event 循环中添加 trace 日志，记录每个 event 的 mst_id
  - [ ] 1.2 在 `update_quest_progress_for_action` (`crates/emukc_gameplay/src/game/quest/update.rs`) 添加 trace 日志，记录 event 类型、匹配的 quest 列表、每个 quest 的 counter 变化
  - [ ] 1.3 在 `Kc3rdQuestCondition::matches_event` (`crates/emukc_model/src/thirdparty/quest/matcher.rs`) 添加 trace 日志，记录匹配结果
- **Verification:** running a batch craft with `RUST_LOG=trace` produces logs at all three sites; the logs reveal whether events are emitted, whether they reach the matcher, and whether the matcher accepts them.

### U2. 验证 quest manifest 数据 (Verify quest manifest data)

- **Goal:** Rule out (or confirm) a data-level root cause before editing code (KTD2).
- **Requirements:** R2.
- **Dependencies:** U1 recommended (the logs identify which quest ID to inspect), but not blocking.
- **Files:** `.data/codex/quest.json` (read-only inspection); quest definition types in `crates/emukc_model`.
- **Tasks:**
  - [ ] 2.1 确认用户触发的具体 quest ID，检查该 quest 在 Codex 中的 requirements 定义
  - [ ] 2.2 确认 quest condition 类型是否为 `Kc3rdQuestConditionFactory::SlotItemConstruction`
  - [ ] 2.3 确认 quest 的 period (daily/weekly/once) 和 activated status
- **Verification:** the triggered quest's condition type, period, and activation status are documented; either it matches the matcher's expectation (data ruled out) or it does not (data is the root cause → separate data-fix commit).

### U3. 修复 quest 进度更新 (Repair quest progress update)

- **Goal:** Fix the confirmed break point so development-quest progress advances correctly for single and batch crafts.
- **Requirements:** R3, R4, R5, R6.
- **Dependencies:** U1 and U2 must complete first — the repair acts on the diagnosis (KTD3).
- **Files:** the file(s) identified by the diagnosis — `crates/emukc_model/src/thirdparty/quest/matcher.rs`, `crates/emukc_gameplay/src/game/factory.rs`, `crates/emukc_gameplay/src/game/quest/update.rs`, and/or `src/bin/net/router/kcsapi/api_req_kousyou/createitem.rs`; quest manifest data if the root cause is data.
- **Tasks:**
  - [ ] 3.1 根据诊断结果修复断裂点（可能需要扩展 event 类型、修正 matcher 逻辑、或修正 quest manifest 数据）
  - [ ] 3.2 如果某些 quest 需要计算失败开发，添加 `DevelopmentAttempt` 事件类型并在 createitem handler 中发送
- **Verification:** single craft advances progress by 1 (R3); batch of 3 successful crafts advances by 3 (R4); failed crafts are handled per the quest's expectation (R5); `cargo test -p emukc_gameplay` and `cargo test --test gameplay_tests` green.

### U4. 测试 (Tests)

- **Goal:** Pin the fix against regression for single, batch, and failure-containing crafts.
- **Requirements:** R3, R4, R5.
- **Dependencies:** U3 must land first.
- **Files:** `tests/gameplay_tests/` — new test(s) for factory/quest progress.
- **Tasks:**
  - [ ] 4.1 添加测试：单次开发推进 quest 进度
  - [ ] 4.2 添加测试：批量开发（3 次）正确推进 quest 进度
  - [ ] 4.3 添加测试：批量开发（含失败）正确处理 quest 进度
- **Verification:** the three tests pass; they are codex-dependent and skip cleanly if `.data/codex` is absent.

## Acceptance / Done

- A1. Diagnostic logs land (U1) and reveal the break point.
- A2. Quest manifest data is verified (U2) and either ruled out or corrected.
- A3. Single craft advances development-quest progress by exactly 1.
- A4. Batch of 3 successful crafts advances progress by exactly 3.
- A5. Failed crafts are counted or skipped correctly per the quest's expectation.
- A6. The three regression tests (U4) pass; `cargo test --test gameplay_tests` green.

## Sources / Research

- Source openspec change: `openspec/changes/fix-batch-craft-quest-progress/` (proposal.md, design.md, tasks.md, specs/quest-factory-events/spec.md) — translated during the openspec sunset (`docs/plans/2026-06-22-001`).
- `crates/emukc_gameplay/src/game/factory.rs` — `create_slotitem`, the quest-event emission loop.
- `crates/emukc_gameplay/src/game/quest/update.rs` — `update_quest_progress_for_action`.
- `crates/emukc_model/src/thirdparty/quest/matcher.rs` — `QuestActionEvent`, `Kc3rdQuestCondition`, `matches_event`.
- `src/bin/net/router/kcsapi/api_req_kousyou/createitem.rs` — the development handler, `api_multiple_flag`, `crafted_mst_ids`.
- `docs/solutions/architecture-patterns/quest.md` — migrated quest capability contract (event-matching invariants this plan must stay consistent with).
