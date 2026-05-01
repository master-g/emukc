## 1. 添加诊断日志

- [ ] 1.1 在 `create_slotitem` (`crates/emukc_gameplay/src/game/factory.rs`) 的 quest event 循环中添加 trace 日志，记录每个 event 的 mst_id
- [ ] 1.2 在 `update_quest_progress_for_action` (`crates/emukc_gameplay/src/game/quest/update.rs`) 添加 trace 日志，记录 event 类型、匹配的 quest 列表、每个 quest 的 counter 变化
- [ ] 1.3 在 `Kc3rdQuestCondition::matches_event` (`crates/emukc_model/src/thirdparty/quest/matcher.rs`) 添加 trace 日志，记录匹配结果

## 2. 验证 quest manifest 数据

- [ ] 2.1 确认用户触发的具体 quest ID，检查该 quest 在 Codex 中的 requirements 定义
- [ ] 2.2 确认 quest condition 类型是否为 `Kc3rdQuestConditionFactory::SlotItemConstruction`
- [ ] 2.3 确认 quest 的 period (daily/weekly/once) 和 activated status

## 3. 修复 quest 进度更新

- [ ] 3.1 根据诊断结果修复断裂点（可能需要扩展 event 类型、修正 matcher 逻辑、或修正 quest manifest 数据）
- [ ] 3.2 如果某些 quest 需要计算失败开发，添加 `DevelopmentAttempt` 事件类型并在 createitem handler 中发送

## 4. 测试

- [ ] 4.1 添加测试：单次开发推进 quest 进度
- [ ] 4.2 添加测试：批量开发（3 次）正确推进 quest 进度
- [ ] 4.3 添加测试：批量开发（含失败）正确处理 quest 进度
