## Context

Port handler 每次请求调用 `update_materials`，执行 read→replenish→save。无并发保护。

### 所有 material 写入路径

| 路径 | 操作 | 事务保护 |
|------|------|----------|
| `api_port/port` → `update_materials` | 自動回復 | 自身事务 |
| `quest_clear_and_claim_reward` → `add_material_impl` | 奖励发放 | 自身事务 |
| `createitem` → `deduct_material_impl` | 开发消耗 | 自身事务 |
| `createship` → `deduct_material_impl` | 建造消耗 | 自身事务 |
| `ndock/highspeed` → `deduct_material_impl` | 即修消耗 | 自身事务 |
| `supply` → `deduct_material_impl` | 补给消耗 | 自身事务 |
| `remodel` → `deduct_material_impl` | 改修消耗 | 自身事务 |

每个路径都在独立事务中执行 read→modify→save。SQLite 在 WAL 模式下支持并发读但写串行化。但 SeaORM 的 `save()` 没有 optimistic lock，两个事务的读可以在对方写之前完成，导致 last-write-wins。

### 触发场景

```
T1: quest_clear_and_claim_reward 读取 material (fuel=3000, bucket=50)
T2: api_port/port update_materials 读取 material (fuel=3000, bucket=50)
T1: quest 添加 fuel+100, bucket+3 → save (fuel=3100, bucket=53)
T2: replenish (时间差小, fuel+0) → save (fuel=3000, bucket=50) ← 覆盖 T1!
```

bucket/torch/devmat/screw 不参与 replenish 计算，但 `apply_self_replenish` 仍然会 save 整个 record（包含这些字段）。如果 T1 在 T2 的 save 之前修改了 bucket，T2 的 save 会覆盖 T1 的 bucket 增加。

但用户说资源"变为个位数"，初始值 bucket=3, torch=3, devmat=5。如果 material record 被意外 re-initialize，就会恢复到初始值。

### apply_self_replenish 边界检查

```rust
let diff = now.timestamp_millis() - material.last_update_primary.timestamp_millis();
if diff >= self.primary_resource_regenerate_rate {  // 60_000ms = 1min
    let replenish = diff / self.primary_resource_regenerate_rate;
    for resource in [&mut material.fuel, &mut material.ammo, &mut material.steel] {
        if *resource < soft_cap {
            *resource += replenish;
            ...
        }
    }
    material.last_update_primary = now;
}
```

如果 `last_update_primary` 远在未来（时钟错误），`diff` 为负，`diff >= rate` 为 false，不会触发回复。`replenish` 为负数时不进入 if，也不会减少资源。

## Goals / Non-Goals

**Goals:**
- 诊断确切的 material 数据丢失根因
- 添加并发保护防止丢失更新
- 验证 apply_self_replenish 边界条件安全

**Non-Goals:**
- 不修改回复速率或 cap 逻辑
- 不引入外部锁服务

## Decisions

### Decision 1: 使用 SeaORM optimistic locking

在 material entity 添加 `version` column（integer，每次 save +1）。`update_materials_impl` 读时记录 version，save 时 WHERE version = old_version。如果 affected rows = 0，说明被并发修改，重试。

### Decision 2: 添加 material 变更日志

在 `update_materials_impl`、`add_material_impl`、`deduct_material_impl` 的入口和出口添加 trace 日志，记录完整的 8 个字段值。用于确认数据丢失发生的时间点和上下文。

## Risks / Trade-offs

- [Risk] optimistic lock 增加写冲突时的重试 → Mitigation: material 操作频率低，冲突概率小
- [Risk] 需要数据库 migration 添加 version column → Mitigation: SQLite ALTER TABLE 添加列
- [Risk] 可能根因不是竞态而是 re-initialize → Mitigation: 日志先确认，再决定方案
