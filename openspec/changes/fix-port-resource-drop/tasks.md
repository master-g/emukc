## 1. 添加诊断日志

- [ ] 1.1 在 `update_materials_impl` (`crates/emukc_gameplay/src/game/material.rs`) 入口/出口添加 trace 日志，记录完整 8 字段值 + last_update 时间戳
- [ ] 1.2 在 `add_material_impl` 入口/出口添加 trace 日志
- [ ] 1.3 在 `deduct_material_impl` 入口/出口添加 trace 日志
- [ ] 1.4 用日志复现 bug，确认数据丢失发生在哪个写入路径

## 2. 确认根因

- [ ] 2.1 分析日志确认是并发覆盖还是 re-initialize 或其他原因
- [ ] 2.2 检查 `Material → ActiveModel` 转换是否丢失字段
- [ ] 2.3 检查 SeaORM save() 在 SQLite WAL 模式下的并发行为

## 3. 添加并发保护

- [ ] 3.1 在 material entity (`crates/emukc_db/src/entity/profile/material.rs`) 添加 `version` column (i64, default 0)
- [ ] 3.2 更新 `Material` struct 和 `From` impl
- [ ] 3.3 `update_materials_impl` 读时记录 version，save 时 WHERE version = old_version，affected rows = 0 时重试
- [ ] 3.4 `add_material_impl` 和 `deduct_material_impl` 同样添加 optimistic lock

## 4. 验证 apply_self_replenish 安全性

- [ ] 4.1 确认 `diff` 为负数时不会减少资源（当前仅跳过回复，不会减少 ✓）
- [ ] 4.2 确认 `apply_hard_cap` 只降不增，且 cap 值合理

## 5. 测试

- [ ] 5.1 添加测试：并发 update_materials 不丢失数据
- [ ] 5.2 添加测试：quest reward + update_materials 并发不丢失 bucket
- [ ] 5.3 cargo test 全量通过
