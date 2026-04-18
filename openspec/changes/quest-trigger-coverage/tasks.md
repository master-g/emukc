## 1. QuestActionEvent 扩展

- [x] 1.1 在 `matcher.rs` 的 `QuestActionEvent::SlotItemScrapped` 中添加 `stars: i64` 字段
- [x] 1.2 在 `QuestActionEvent` 中添加 `ModernizationCompleted { target_ship_mst_id: i64, material_ship_mst_ids: Vec<i64> }` 变体
- [x] 1.3 在 `QuestActionEvent` 中添加 `EnemyShipSunk { ship_stype: i64 }` 变体
- [x] 1.4 在 `QuestActionEvent` 中添加 `SlotItemImproved { item_mst_id: i64, stars: i64 }` 变体

## 2. matches_event 扩展

- [x] 2.1 添加 `Scrap::SpecificItems` 匹配 `SlotItemScrapped` 的 arm
- [x] 2.2 添加 `Modernization` 匹配 `ModernizationCompleted` 的 arm
- [x] 2.3 添加 `Sink` 匹配 `EnemyShipSunk` 的 arm
- [x] 2.4 添加 `Factory::SlotItemImprovement` 匹配 `SlotItemImproved` 的 arm

## 3. apply_event 实现

- [x] 3.1 实现 `SpecificItems` 的 apply 逻辑：通过 Codex 查 equip type，匹配 item_type（Equipment 按 mst_id，EquipType 按 api_type[3]）+ stars 阈值，减 amount
- [x] 3.2 实现 `Modernization` 的 apply 逻辑：验证 target_ship 条件、material_ship 条件、batch_size，减 times
- [x] 3.3 实现 `Sink` 的 apply 逻辑：验证 Kc3rdQuestConditionShip 条件匹配 ship_stype，减 count
- [x] 3.4 实现 `SlotItemImprovement` 的 apply 逻辑：减 count（同 ShipConstruction 模式）

## 4. is_satisfied 修复

- [x] 4.1 修改 `progress.rs` 中 `Scrap::SpecificItems` 的 `is_satisfied`：从 `false` 改为检查所有 item 的 `amount == 0`

## 5. 触发点接入

- [x] 5.1 修改 `slot_item.rs:destroy_items_impl` — 传入 `item.level` 作为 `stars` 到 `SlotItemScrapped` 事件
- [x] 5.2 修改 `factory.rs:destroy_ship` — 更新 `SlotItemScrapped` 事件（舰上装备废弃），传入 stars 字段
- [x] 5.3 修改 `compose/powerup.rs:powerup_impl` — 在近代化改修后调用 `update_quest_progress_for_action`，传入 `ModernizationCompleted` 事件
- [x] 5.4 扩展 `SortieBattleResultSnapshot` — 增加 `enemy_ship_types: Vec<i64>` 和 `enemy_nowhps: Vec<i64>` 字段
- [x] 5.5 更新 `sortie.rs` 中所有 snapshot 构建点（日战、夜战、SP 夜战）填充 enemy_ship_types 和 enemy_nowhps
- [x] 5.6 在 `sortie.rs` 结果处理处（~line 567），对每艘 `enemy_nowhps <= 0` 的敌舰触发 `EnemyShipSunk { ship_stype }` 事件

## 6. 测试

- [x] 6.1 为 `SpecificItems` 的 `matches_event` / `apply_event` 编写单元测试（覆盖 Equipment 和 EquipType 两种匹配）
- [x] 6.2 为 `SpecificItems` 编写复合条件测试（SpecificItems + Consumption + ModelConversion 组合）
- [x] 6.3 为 `Modernization` 的 `matches_event` / `apply_event` 编写单元测试
- [x] 6.4 为 `Sink` 的 `matches_event` / `apply_event` 编写单元测试（验证 ShipType 匹配）
- [x] 6.5 为 `SlotItemImproved` 的 `matches_event` / `apply_event` 编写单元测试
- [x] 6.6 运行 `cargo test` 全量通过
- [x] 6.7 运行 `cargo clippy --workspace` 无新警告
