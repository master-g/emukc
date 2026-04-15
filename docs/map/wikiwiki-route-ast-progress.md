# Wikiwiki Route Parser Status

> 这份文档只记录 route parser / runtime map IR 的当前状态。敌舰与 battle 侧的联动见 `plan.md`。

## TL;DR

- repo-tracked `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json` 已经是常规图的主语义资产，当前规模为 **130 maps / 131 variants**。
- parser 现在会把 `出撃` / `出撃ポイント` / `スタート` / `Start` 统一规范化为 canonical entry node `Start`，并在可能时编译成 `routing_rules[0]`。
- `MapCellDefinition` 现在保留 `node_label`；`first_progress_cell_no()` 只对**确定单入口** variant 返回值，不再把歧义起点偷偷降级成“取第一个”。
- 当前 asset 中 `Unknown = 4`、`SourceUnknown = 0`、`variants_with_warnings = 7`，其中 `structural_start_fallback = 3`。
- 当前 repo asset 里，`cell_0` 的“推断型多起点但无 start rule”残留已经降到 **0**。
- 当前 repo asset 里仅有 **4** 条 route-history 规则会看 sortie 已访问节点（4-5 / 5-5 / 7-4）；它们已经编译成 runtime `VisitedNode`，但仍**没有**“只看本次从哪条边进入当前点”的 direct arrival-edge IR。

## Current Architecture

### 1. Parse-time entry normalization

- `parse_node_label()` 识别并规范化以下入口文本：
	- `出撃`
	- `出撃ポイント`
	- `スタート`
	- `Start`
- 入口节点统一映射到 `ENTRY_NODE_LABEL = "Start"`。
- `Start` **不是**普通 wikiwiki 路点；它是 route IR 里的专用 entry endpoint。

### 2. Graph assembly no longer treats entry as a normal cell

- `route.rs` 在构建 node graph / BFS 编号时，会把 `Start` 从普通节点集合中排除。
- 这样可以避免把入口点误当成 `A` / `1` 之类的普通可见格子。

### 3. Start routing is compiled into runtime map data

- `parser/wikiwiki_map/mod.rs` 会把 `Start -> cell_0` 建立为 canonical 映射。
- 如果 wikiwiki 路由表里存在显式起点分支，则会生成：
	- `cell_0.next_cells`
	- `routing_rules[0]`
- 如果 wikiwiki 没有显式起点规则：
	- 先记录 `missing_start_routes` / `inferred_multi_root_start:*`
	- 再由 merge 阶段尝试用 `kc_data` 的结构化起点补齐

### 4. Structural start fallback is explicit, not silent

- `crates/emukc_model/src/codex/map/merge.rs` 现在会在 wikiwiki 起点缺失时，使用 `kc_data` 的 `route.from == None` 作为结构化补全来源。
- 成功补全后：
	- 删除 `missing_start_routes` / `inferred_multi_root_start:*`
	- 增加 `structural_start_fallback`
- 这表示“当前 start edge 可信，但来源不是 wikiwiki 显式规则，而是客户端结构数据补齐”。

### 5. Runtime no longer treats cell 0 as "pick the first edge"

- `start_sortie()` 现在和普通分支一样，统一走 `evaluate_route_destination()`
- `cell_0` 解析优先级为：
	1. 显式 `routing_rules[0]`
	2. 结构化 start fallback
	3. 报错（对于仍然歧义的 inferred multi-root start）

当前行为边界：

- **显式 start rules**：按规则执行
- **`structural_start_fallback` + 多出口**：允许 runtime 在这些结构化出口中随机选取
- **仅靠 inferred multi-root、没有显式/结构化规则**：runtime 直接拒绝，而不是再偷偷走第一个节点

## What Is Actually Fixed

### The old "always jump to A" bug is no longer the default

之前的问题链条是：

1. wikiwiki 的 `出撃` 行在 parse 时被丢弃
2. bootstrap 用 graph root 推断 `cell_0.next_cells`
3. runtime 对 `cell_0` 直接取 `next_cells[0]`

现在这条链已经被拆掉：

- entry rows 会保留进 AST
- 显式 start rules 会直接落到 `routing_rules[0]`
- runtime 不再把歧义起点默默解释成 “A first”

### Tests no longer encode the same bad assumption

- boss-path 相关测试不再把 `first_progress_cell_no()` 当成“肯定存在的起始点”
- sortie 流程测试现在按 runtime-valid path 推进，而不是静态拿 `next_cells[0]`

## Current Boundaries

### 1. Unknown predicates still remain

当前 asset 里还剩 **4** 个 `Unknown` predicate。它们说明 parser 仍然没有完全吃掉所有 wikiwiki route prose 变体。

已知剩余问题主要是：

- 少数特殊 composition / history / prose 条件文本
- 少数 wikiwiki 表述与当前 AST predicate vocabulary 还未对齐
- 当前残留主要集中在 **4-2** 这类“随机 + 额外空母系条件提示”的复合说明

这已经不是“起点飞到 A”的问题，而是更细粒度的 route condition 覆盖率问题。

### 2. `node_label` 已保留，但 stable semantic identity 仍是半完成

- `MapCellDefinition.node_label` 已经进入 runtime map catalog
- `cell_0` 的起点语义也已经独立出来

但当前**通用 merge 主键**仍然主要依赖 `cell_no`，不是完全由语义 label 驱动。也就是说：

- start-edge 恢复已经落地
- 更一般的“跨来源 stable node identity”仍是下一阶段工作

### 3. Arrival-context-sensitive routing is still not modeled

wikiwiki 上部分地图的分歧会依赖“从哪里进入当前点”。当前 IR 还没有把 arrival context 编成一级公民。

这不会再导致起点固定飞 A，但仍可能影响更复杂地图的后续 fidelity。

更准确地说，**当前 runtime 支持的是 sortie-wide route history，不是 direct arrival edge**：

- parser 会把 `Xマスを経由し、...分岐する`、`Xマスを経由済み`、`Xマス未経由` 先编成 `VisitedNodeLabel`
- `parser/wikiwiki_map/mod.rs` 会在 variant 组装后把它 rewrite 成 runtime `VisitedNode { cell_nos, visited }`
- `emukc_gameplay` 运行时只保存 `visited_cell_ids`，并用它求值

当前仍然**缺少**：

- `ActiveSortieState` / `FleetRouteContext` 里的 `previous_cell_id` / `arrival_from_cell_id`
- runtime predicate（例如 `ArrivedFrom { cell_nos }`）
- 只在源码显式写出“从 X 来到当前点”时才触发的 parser lowering

因此，今天的 engine 能表达“曾经经过 E”，但不能表达“这一步是从 E 直接进入当前点”。

#### Current repo asset audit

当前 repo-tracked normal map asset 里，唯一会看路线历史的规则就是下面这 4 条，且都已经落成 runtime `VisitedNode`：

- **4-5 default**：`K -> M` when `E` visited（`map_id=45`, `from_cell_no=9`, `to_cell_no=10`, `VisitedNode[3]`）
- **5-5 default**：`M -> O` when `N` visited（`map_id=55`, `from_cell_no=10`, `to_cell_no=15`, `VisitedNode[13]`）
- **5-5 default**：`N -> O` when `M` visited（`map_id=55`, `from_cell_no=13`, `to_cell_no=15`, `VisitedNode[10]`）
- **7-4 default**：`J -> K` when `D` visited（`map_id=74`, `from_cell_no=7`, `to_cell_no=9`, `VisitedNode[4]`）

这些例子说明“路线历史”在常规图里确实存在；但它们都还能用当前 `visited_cell_ids` 语义表达。**当前 repo asset 里还没有一个明确证据表明必须新增 immediate-arrival-only predicate 才能继续前进**。

反过来说，也不能从 graph 形状直接推导 direct arrival 语义：

- 4-5 的 `K`
- 5-5 的 `M` / `N`

都存在 loop / re-entry 可能，`曾经经过 E/N/M` 与 `这一步正好从 E/N/M 进入` 不是同一个概念。没有源码级显式文本时，不应该靠 graph 猜一个更强的 arrival 语义。

### 4. Non-start ambiguous cells still rely on precompiled structure

对 `cell_0` 的错误降级已经被移除；但对**非起点**节点，如果 asset 自身没有编译出规则、只剩多个结构化 `next_cells`，runtime 仍然依赖当前 catalog 的编译结果。

因此，下一步重点是继续减少 `Unknown` predicate，而不是再把更多 runtime fallback 塞进 engine。

## Warning Taxonomy

- `missing_start_routes`
	- wikiwiki 没写出显式 start rules，且 parser 没法直接得到起点分支
- `inferred_multi_root_start:X,Y`
	- 只能从 graph roots 推出多个起点候选，语义歧义
- `structural_start_fallback`
	- wikiwiki 没给出可直接使用的 start rules，但 `kc_data` 结构化起点已成功接管

## Maintenance Rules

- 任何 wikiwiki parser / route lowering 修改，只有在重新生成 `wikiwiki_map_catalog.json` 后才算生效
- 如果测试或本地运行加载的是 `.data/codex`，则还需要同步刷新 runtime codex；仅修改未提交的 `wikiwiki_map_catalog.json` 不会自动改变这些消费者看到的数据
- 如果新增了一类入口文本，必须同时更新：
	- parser normalization
	- parser regression fixtures
	- runtime start-routing tests
- 新增 route predicate 支持时，优先减少 `Unknown` / `variants_with_warnings`，不要再扩大 runtime 的 silent fallback 面

### If a future immediate-arrival rule appears

最小、低风险的 first slice 应该是：

1. parser 先保留 `ArrivedFromNodeLabel { node_labels }` 一类**仅用于 lowering** 的 label-level predicate
2. 在现有 label -> primary `cell_no` rewrite 之后，落成 runtime `ArrivedFrom { cell_nos }`
3. `ActiveSortieState` 增加 `arrival_from_cell_id`
4. `FleetRouteContext` 在 evaluate 当前节点时携带该 arrival edge
5. **只**对源码明确写出“从 X 来到当前点/由 X 进入”这类文本启用；不要从 predecessor / loop 结构反推

在没有这样的源码证据前，不要为了“也许需要”而扩 runtime IR，更不要把 direct arrival 退化成 silent graph guess。

## Recommended Validation

```bash
cargo test -p emukc_bootstrap --lib
cargo test -p emukc_gameplay --lib
cargo test -p emukc_gameplay --test sortie_battle
cargo run --bin emukcd -- wikiwiki-map normalize --data-root .data/temp --output crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json
```
