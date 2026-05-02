## Why

`api_get_member/useitem` 和 `api_get_member/require_info` 返回的 bucket/torch/devmat/screw 计数来自 `use_item` 表，但这些资源的实际变动只更新了 `material` 表。两表数据分歧后，客户端用 useitem/require_info 响应覆盖正确的 material 显示，导致回港后资源显示异常（如 bucket 跳回初始值）。

## What Changes

- 将 bucket/torch/devmat/screw 的权威数据源统一到 `material` 表
- `get_use_items_impl` 对这 4 种资源从 `material` 表读取，不再查 `use_item` 表
- 所有 add/deduct 操作对这 4 种资源只写 `material` 表（现状已如此，无需改动）

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `useitem-response`: useitem 和 require_info 返回 bucket/torch/devmat/screw 时从 material 表派生，而非 use_item 表

## Impact

- `crates/emukc_gameplay/src/game/use_item.rs` — `get_use_items_impl` 增加合并 material 数据逻辑
- `src/bin/net/router/kcsapi/api_get_member/useitem.rs` — 无变更（使用共享的 get_use_items）
- `src/bin/net/router/kcsapi/api_get_member/require_info.rs` — 无变更（使用共享的 get_use_items）
- `crates/emukc_gameplay/src/game/material.rs` — 无变更（已是权威数据源）

## Non-goals

- 不修改 material 表 schema（不加 version column，不加 optimistic lock）
- 不修改 apply_self_replenish 或 apply_hard_cap 逻辑
- 不合并 use_item 表到 material 表
- 不从 material 表移除 bucket/torch/devmat/screw 字段
