## Why

战斗结束回港时，port API 偶尔返回个位数的 bucket 和其他资源数量。`update_materials_impl` 对 material record 执行 read→replenish→save，无任何并发保护。与 quest reward、ndock、supply 等其他写操作共享同一张 material 表，SeaORM 默认无 optimistic lock，存在 last-write-wins 丢失更新风险。

## What Changes

- **诊断 material 写入竞态**: 追踪所有 material 写入路径，确认并发读-改-写是否导致数据丢失
- **为 material 操作添加并发保护**: 使用 SeaORM optimistic locking（version column）或 `SELECT ... FOR UPDATE` 防止并发丢失更新
- **验证 `apply_self_replenish` 边界条件**: 确认时间差计算不会因时间回拨、溢出等产生异常值

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `material-concurrency`: material 表的并发写入保护，防止 read-modify-write 丢失更新

## Impact

- `crates/emukc_db/src/entity/profile/material.rs` — SeaORM entity，可能需要添加 version column
- `crates/emukc_gameplay/src/game/material.rs` — update_materials_impl, add_material_impl, deduct_material_impl
- `crates/emukc_model/src/profile/material.rs` — Material struct, apply_self_replenish
- `src/bin/net/router/kcsapi/api_port/port.rs` — port handler

## Non-goals

- 不修改 material 回复速率或 cap 逻辑
- 不修改 ship-level 燃料/弹药消耗
- 不引入分布式锁或外部依赖
