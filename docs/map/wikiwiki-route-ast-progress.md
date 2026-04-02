# Wikiwiki Route Parser Status

这份文档只保留 route parser / runtime map IR 的当前状态。敌舰属性调查单独放在 `kancolle-map-research.md`。

## TL;DR

- repo-tracked `wikiwiki_map_catalog.json` 已经是常规图的主地图语义源。
- runtime 消费的是 flat `RouteRule` / `RoutePredicate`，不是 AST runtime。
- 当前 repo asset 统计结果：
  - `36` 张地图
  - `37` 个 variant
  - `Unknown = 0`
  - `SourceUnknown = 0`
  - `parse_warnings = 0`

## 当前架构

关键文件：

- `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/resolver.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/drop.rs`
- `crates/emukc_model/src/codex/map.rs`
- `crates/emukc_gameplay/src/game/sortie.rs`

当前链路是：

1. parser 从 wikiwiki 页面抽 route / enemy / drop 表。
2. route parser 在内部保留 AST。
3. AST 在 bootstrap 阶段被编译成 runtime flat rule。
4. repo asset 落成 `MapCatalog`。
5. runtime 用 `MapVariantDefinition.routing_rules` 执行选路。

## 已经成立的事实

### 1. route runtime 已经打通

`gameplay::evaluate_route_destination()` 已经会基于：

- 当前 cell
- 当前 variant
- `FleetRouteContext`
- `RouteRule`
- `RoutePredicate`

做目标 cell 选择。

目前 runtime 已覆盖的 predicate 族包括：

- fleet size
- ship type / ship id / ship set
- flagship ship type / flagship ship id
- speed
- LoS
- drum canister
- equipment count
- visited node
- weighted random

### 2. parser AST 仍然只存在于 bootstrap

当前没有 AST runtime。

也就是说：

- parser 复杂度放在 `emukc_bootstrap::parser::wikiwiki_map`
- runtime 仍然只认识 `RouteRule` / `RoutePredicate`

这是当前代码里明确的边界，不是过渡态注释。

### 3. route warning 收口已经完成当前阶段

当前 repo asset 里已经没有：

- `RoutePredicate::Unknown`
- `RoutePredicate::SourceUnknown`
- `parse_warnings`

所以当前主问题已经不再是“常规图 route parser 还没收口”。

## runtime 仍保留的边界

虽然当前 asset 不再依赖 fallback，但代码侧仍保留了受控降级路径：

- 唯一 `Always` fallback 时允许退回
- 当前 cell 的规则都不支持且玩家显式选了合法 target 时，允许受控通过
- 纯 `SourceUnknown` 情况下允许退化为候选 target 选择

这些逻辑还在 `crates/emukc_gameplay/src/game/sortie.rs`，但**当前 repo asset 已不再需要它们才能走通常规图主链路**。

## 当前 asset 的定位

当前地图资产链路的定位已经比较清楚：

- wikiwiki asset = 主语义源
- `kc_data` = 结构补充源

这也意味着：

- parser 变更如果影响 normalize 结果，通常应该连同 asset 一起更新
- `kc_data` 仍可能影响 wikiwiki 未覆盖地图的结构结果

## 当前还没做的事情

### 1. drop runtime 仍然是最小实现

当前只做到了 ship drop 主链路，没有做：

- `api_get_useitem`
- `api_get_slotitem`
- no-drop / rate control
- 更细的活动限定规则

### 2. variant 语义仍然偏常规图

当前已经足够支撑：

- 常规图多阶段
- clear 后 variant 切换
- gauge defeat count

但还没有抽象成活动海域通用 selector，例如：

- event rank
- event phase
- 更复杂的 stage graph

### 3. 敌舰属性不是这条线的 blocker

当前 wikiwiki map asset 已经有 enemy composition 的 `ship_ids`，但“敌舰完整属性 / 敌装”缺口属于 battle master-data 问题，不属于 route parser 主线。

## 维护建议

如果继续沿当前路线推进，建议保持下面几条约束：

1. parser 内部可以继续保留 AST，但 runtime 先不引入 AST 执行器。
2. 每次 normalize 后都检查：
   - `Unknown`
   - `SourceUnknown`
   - `parse_warnings`
3. 遇到新的 wikiwiki 文本变体，优先补 parser / fixture，而不是扩大 runtime fallback。
4. parser 或 asset 语义变更时，同步更新 `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`。

## 推荐验证命令

```bash
cargo test -p emukc_bootstrap wikiwiki_map -- --nocapture
cargo test -p emukc_bootstrap load_map_catalog -- --nocapture
cargo test -p emukc_gameplay route_predicate_matches -- --nocapture
cargo test -p emukc_gameplay route_rules -- --nocapture
cargo test -p emukc_gameplay repo_wikiwiki_asset_supports_real_map_boss_progression -- --nocapture
cargo run --bin emukcd -- wikiwiki-map normalize --data-root .data/temp --output crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json
```
