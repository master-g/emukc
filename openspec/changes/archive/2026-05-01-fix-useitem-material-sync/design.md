## Context

`api_get_member/useitem` 和 `api_get_member/require_info` 返回玩家的消耗品列表，包括 bucket(id=1)、torch(id=2)、devmat(id=3)、screw(id=4) 以及其他特殊道具。这 4 种资源同时存在于 `material` 表和 `use_item` 表中。

当前问题：
- `material` 表是这 4 种资源的唯一有效写入路径（所有 quest/expedition/ndock/factory/consume 路径均通过 `add_material_impl` / `deduct_material_impl`）
- `use_item` 表中这 4 种资源不存在有效写入路径（quest/expedition 均按 `Material` 类别路由到 `add_material_impl`，不写 `use_item` 表）
- `api_get_member/useitem` 和 `api_get_member/require_info` 均通过 `get_use_items_impl` 直接查 `use_item` 表，返回过期或不存在的数据
- 客户端在回港后调用 useitem 或 require_info 接口，用过期数据覆盖正确的 material 显示

### 数据流现状

```
写入路径                              material 表   use_item 表
──────────────────────────────────    ──────────    ──────────
quest reward (Material category)       ✓ +N          ✗
expedition reward (Bucket/Torch/...)   ✓ +N          ✗
ndock highspeed (bucket-1)             ✓ -1          ✗
factory createship (torch-1)           ✓ -1          ✗
consume_use_item (medal→bucket)        ✓ +N          ✗
pay_item (store purchase)              ✓ +N          ✗
```

注意：quest reward 的 `apply_single_reward` 按 `Kc3rdQuestRewardCategory` 路由：
- `Material` 类别 → `add_material_impl`（bucket/torch/devmat/screw 走此路径）
- `UseItem` 类别 → `add_use_item_impl`（其他道具走此路径）
两者互斥，不存在 bucket/torch/devmat/screw 的冗余 `add_use_item_impl` 调用。

读取路径：
```
api_port/port → get_materials → material 表 ✓
api_get_member/useitem → get_use_items_impl → use_item 表 ✗ (过期)
api_get_member/require_info → get_use_items_impl → use_item 表 ✗ (过期)
```

## Goals / Non-Goals

**Goals:**
- `api_get_member/useitem` 和 `api_get_member/require_info` 对 bucket/torch/devmat/screw 返回 material 表的值
- 消除 material 表和 use_item 表之间这 4 种资源的数据分歧

**Non-Goals:**
- 不修改 material 表 schema
- 不修改 use_item 表 schema
- 不修改 apply_self_replenish / apply_hard_cap 逻辑
- 不重构 use_item 的 init/wipe 流程

## Decisions

### Decision 1: 在 get_use_items_impl 共享层合并 material 数据

在 `get_use_items_impl`（`crates/emukc_gameplay/src/game/use_item.rs`）中，增加 material 表查询和合并逻辑。此函数被 `useitem` 和 `require_info` 两个端点共同调用，在共享层修复可避免逻辑重复和遗漏。

合并逻辑：
- 查询 material 表获取 bucket/torch/devmat/screw 的当前值
- 查询 use_item 表获取其他道具
- 遍历 use_item 查询结果，对 mst_id 为 1(Bucket)/2(Torch)/3(DevMat)/4(Screw) 的条目，用 material 表的对应值替换 count
- material 表中这 4 个值的映射：`material.bucket → mst_id=1`, `material.torch → mst_id=2`, `material.devmat → mst_id=3`, `material.screw → mst_id=4`
- 若 use_item 表无这 4 种资源的记录，补充条目

### Decision 2: 保留 use_item 表中的存量记录

不删除 use_item 表中已有的 bucket/torch/devmat/screw 记录。这些记录可能由早期版本或手动操作创建。`get_use_items_impl` 在合并时会用 material 值覆盖它们的 count，所以存量记录不会造成问题。

不删除的原因：避免额外的 DELETE 操作，且这些记录的存在不影响正确性。

### Decision 3: 不修改 add/deduct 路径

保持 add_material_impl / deduct_material_impl 不变。它们已经是这 4 种资源的唯一有效写入路径，不存在冗余的 `add_use_item_impl` 调用需要移除。

### Decision 4: get_use_items_impl 需要 access 到 material 数据

`get_use_items_impl` 当前签名只接受 `ConnectionTrait`，不查 material 表。修改方案：
- 新建 `get_use_items_with_material_impl` 函数，接受额外参数用于查询 material 表
- 或扩展 `get_use_items_impl` 签名以接受 material 数据

此决策待实现时根据调用方便利性确定。

## Risks / Trade-offs

- [Risk] use_item 表中 bucket/torch/devmat/screw 的记录会逐渐与 material 表不一致 → Mitigation: get_use_items_impl 不再读取这些记录的 count，用 material 值覆盖
- [Risk] 如果未来有代码依赖 use_item 表中这 4 种资源的 count 做逻辑判断，会读到错误值 → Mitigation: 在代码中添加注释说明这些值不可靠，权威源在 material 表
- [Risk] incentive 数据如果以 `IncentiveType::UseItem` 类型存储 bucket/torch/devmat/screw，会导致只写 use_item 表而不写 material 表，修复后 useitem 反而读到未更新的值 → Mitigation: 验证 incentive 数据中是否存在此类分类，若存在则需同时修改 incentive 写入路径
