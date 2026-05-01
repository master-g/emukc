## Why

批量开发 (api_multiple_flag=1) 时，开发相关任务进度不推进。初步代码审查显示 `create_slotitem` 已对每个成功 item 调用 `update_quest_progress_for_action`，但用户报告任务不前进。需深入定位事件匹配、任务条件解析、或计数逻辑的具体断裂点。

## What Changes

- **增加开发失败事件的 quest 追踪**: 当前仅成功开发 (mst_id > 0) 发送 `SlotItemConstructed` 事件。部分日常任务可能期望计算开发尝试次数（含失败）
- **验证 `Kc3rdQuestCondition::Factory` 与 `SlotItemConstructed` 的匹配路径**: 确认 quest manifest 中开发类任务的 condition 类型是否与当前 matcher 一致
- **增加 `Factory2` 类型支持**: 检查是否存在 Factory2 (type 11) 类任务需要不同的匹配逻辑

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `quest-factory-events`: 开发类 quest 事件匹配逻辑，可能需要新增 development-attempt 事件类型或扩展现有 SlotItemConstructed 的匹配范围

## Impact

- `crates/emukc_model/src/thirdparty/quest/matcher.rs` — QuestActionEvent, Kc3rdQuestCondition 匹配逻辑
- `crates/emukc_gameplay/src/game/factory.rs` — create_slotitem 中 quest event 发送逻辑
- `crates/emukc_gameplay/src/game/quest/update.rs` — update_quest_progress_for_action
- `src/bin/net/router/kcsapi/api_req_kousyou/createitem.rs` — 开发 handler
- 任务定义数据（quest manifest）

## Non-goals

- 不修改 quest 奖励系统
- 不修改开发成功率计算
- 不重构 quest 整体架构
