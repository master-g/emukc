## Context

用户报告批量开发 (api_multiple_flag=1) 不推进开发类任务进度。代码层面 `create_slotitem` 已对每个成功 item 调用 `update_quest_progress_for_action`，但任务计数不前进。

### 当前代码流程

```
createitem handler (createitem.rs)
  ├── api_multiple_flag=1 时 upper=3
  ├── 随机生成 crafted_mst_ids (成功=id, 失败=-1)
  └── state.create_slotitem(pid, &crafted_mst_ids, &costs)

create_slotitem (factory.rs:129-140)
  ├── 扣除材料
  ├── 创建装备
  └── for id in mst_id.iter():
       └── if *id > 0:  ← 仅成功 item
            update_quest_progress_for_action(SlotItemConstructed { item_mst_id })

update_quest_progress_for_action (quest/update.rs)
  ├── 查询所有 activated quest
  └── 对每个 quest: condition.matches_event(event) → apply(event)
```

### 可能的断裂点

1. **事件类型不匹配**: `Kc3rdQuestCondition::Factory(SlotItemConstruction(count))` 匹配 `QuestActionEvent::SlotItemConstructed`，但任务 manifest 中 condition 可能是其他类型
2. **失败开发不计数**: 某些日常任务计数开发尝试（含失败），当前代码仅发送成功事件
3. **批量开发消费了一次材料**: `api_multiple_flag=1` 时材料 ×3，但 quest event 仅按成功 item 发送。如果任务期望按"开发次数"而非"成功数"计数，则批量 3 次只发 0~3 个事件
4. **quest manifest condition 类型差异**: Factory vs Factory2，或 condition 结构中 count 初始值不正确

## Goals / Non-Goals

**Goals:**
- 定位开发类任务不推进的确切原因
- 修复匹配或计数逻辑

**Non-Goals:**
- 不重构 quest 系统
- 不修改 quest manifest 数据格式

## Decisions

### Decision 1: 添加诊断日志

在以下关键点添加 trace 日志：
- `createitem handler`: 记录 api_multiple_flag、crafted_mst_ids
- `update_quest_progress_for_action`: 记录 event 类型、匹配的 quest 列表、counter 变化
- `condition.matches_event`: 记录匹配结果

通过日志确认断裂发生在哪一步。

### Decision 2: 验证 quest manifest 数据

检查用户触发的是哪个具体 quest，确认其 condition 定义是否包含 `SlotItemConstruction` type。

## Risks / Trade-offs

- [Risk] 问题可能在 quest manifest 数据而非代码 → Mitigation: 先查数据再改代码
- [Risk] 不同 quest 可能需要不同事件类型 → Mitigation: 按需扩展事件枚举
