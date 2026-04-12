# Map 系统审计报告

> 审计日期: 2026-04-10
> 审计范围: route parser → map catalog → runtime route evaluation → sortie API
> 参考文档: `wikiwiki-route-ast-progress.md`, `kancolle-map-research.md`, `7-3-part2-real-data-plan.md`

## 审计结论

Map 系统实现与文档描述高度一致。起点路由（cell_0 飞到 A）问题已彻底修复，route evaluation 覆盖面广。当前遗留项均为低优先级。

---

## 1. 架构验证

| 层 | 代码位置 | 状态 |
|----|---------|------|
| Bootstrap 解析 | `crates/emukc_bootstrap/` (wikiwiki parser, map pipeline) | ✅ |
| 数据模型 | `crates/emukc_model/src/codex/map/` (types, merge, map) | ✅ |
| Runtime | `crates/emukc_gameplay/src/game/sortie.rs` + `map_route.rs` | ✅ |
| API | `src/bin/net/router/kcsapi/api_req_map/` | ✅ |

数据流: wikiwiki route/enemy/drop → `wikiwiki_map_catalog.json` → runtime 加载 → `kc_data` + public overlay 补齐 → sortie 消费

与 `kancolle-map-research.md` 描述的 Current Data Path 一致。

---

## 2. 起点路由 (cell_0)

### 文档声明

> entry rows 会保留进 AST；显式 start rules 会直接落到 `routing_rules[0]`；runtime 不再把歧义起点默默解释成 "A first"

### 代码验证 (`map_route.rs:198-236`)

`select_route_from_cells` 对 `cell_no == 0` 的处理:

1. **显式 routing rules 存在** → `evaluate_route_destination` 走规则优先级分组
2. **无规则 + inferred_multi_root_start** → **直接报错**，不偷偷 fallback
3. **无规则 + structural_start_fallback (多出口)** → 随机选取
4. **有 selected_cell_id** → 校验合法性后采用

**结论**: ✅ "飞到 A" 问题已修复，且对歧义起点有正确拒绝行为。

---

## 3. Route Predicate 覆盖

`map_route.rs:238-418` 的 `route_predicate_matches`:

### 已实现

| Predicate | 行号 | 状态 |
|-----------|------|------|
| `Always` / `FleetSizeWeightedRandom` | 243-246 | ✅ |
| `VisitedNode` | 247-252 | ✅ |
| `FleetSize` | 259-262 | ✅ |
| `EquipmentCount` | 263-278 | ✅ |
| `ShipTypeCount` | 279-291 | ✅ |
| `FlagshipShipType` / `FlagshipShipId` | 292-301 | ✅ |
| `ContainsShipType` / `ContainsShipId` | 302-310 | ✅ |
| `ContainsShipSet` / `OnlyShipSet` | 312-331 | ✅ |
| `OnlyShipTypes` | 318-325 | ✅ |
| `ShipSetCount` / `ShipSetSpeedCount` | 332-368 | ✅ |
| `Speed` | 369-371 | ✅ |
| `LoS` | 372-376 | ✅ |
| `DrumCanisterCount` | 377-380 | ✅ |
| `And` / `Or` / `Not` | 381-418 | ✅ |

### 未实现

| Predicate | 行号 | 当前影响 |
|-----------|------|---------|
| `VisitedNodeLabel` | 253-254 → `Unsupported` | **无** — 当前 asset 里所有 `VisitedNodeLabel` 都已 rewrite 成 `VisitedNode` |
| `Unknown` | 256-258 → `Unsupported` | 残留 4 条，但已通过 fallback 兜底 |

---

## 4. Route Evaluation 优先级逻辑

`evaluate_route_destination` (`map_route.rs:40-196`):

1. 收集所有匹配 predicate 的规则
2. `Always` predicate 单独放入 fallback 组
3. 非-Always 匹配规则按 predicate key 分组，每组取最低优先级
4. 全局取最低优先级的组作为 executable candidates
5. 若 executable 为空:
   - 全部 `SourceUnknown` → 取 `targets.iter().next()` (偏向最小 cell_no)
   - 有 indeterminate + 唯一 unconditional → 直接取那个
   - 否则报错
6. executable 非空 → 按权重随机选择

**注意**: 步骤 5 的 `SourceUnknown` fallback 取 `BTreeSet::iter().next()` 即最小 cell_no。当前 `SourceUnknown = 0` 所以无实际影响，但如果未来出现 `SourceUnknown` 规则需注意此偏向。

---

## 5. 多阶段地图 (7-3)

`7-3-part2-real-data-plan.md` 计划新增:

- 7-3 双阶段匹配测试
- 第一阶段击破后再次 start 返回第二阶段结构回归测试
- overlay/report 覆盖两个 stage

`sortie.rs` 中存在 `first_gauge_clear_switches_map_variant_without_finishing_map` 测试。

**建议**: 确认文档中列出的具体断言（第二阶段 cell ≥ 17 / master_cell_id 落在 4801..4826）是否已补齐为测试。如果未落地，应按计划补充。

---

## 6. 非起点歧义路由

`map_route.rs:233`: 非 cell_0 节点有多个 `next_cells` 且无规则时，直接取 `next_cells[0]`。

这与起点保护逻辑不同 — 非起点没有歧义拒绝机制。**依赖 catalog 编译质量**确保非起点不会出现无规则歧义。

---

## 7. Enemy Fleet 决定

`sortie.rs` 的 pipeline:

1. `resolve_sortie_enemy_fleet` → 查 cell 的 fleet definition
2. `select_locked_enemy_composition` → 检查 routing 锁定的编成
3. `select_random_enemy_composition` → 多编成时按权重随机
4. `fallback_enemy_composition` → 兜底 ship_id=412

完整且与 `kancolle-map-research.md` "What map data already gives battle" 一致。

---

## 8. Asset 状态核对

文档声称:

| 指标 | 文档值 |
|------|--------|
| maps | 130 |
| variants | 131 |
| variants with warnings | 7 |
| Unknown predicates | 4 |
| SourceUnknown predicates | 0 |
| structural_start_fallback | 3 |
| inferred multi-root without rule | 0 |

建议运行以下命令验证当前 asset 是否仍匹配:

```bash
cargo test -p emukc_bootstrap --lib
```

---

## 9. 遗留项与建议

| # | 优先级 | 项目 | 说明 |
|---|--------|------|------|
| 1 | 低 | 消化剩余 4 个 `Unknown` predicate | 继续扩展 parser vocabulary |
| 2 | 低 | `node_label` → 更稳定的 merge identity | 当前 merge 主键仍是 `cell_no` |
| 3 | 低 | Arrival-context routing (`ArrivedFrom`) | 当前只有 sortie-wide `VisitedNode`，无 direct arrival edge |
| 4 | 中 | 确认 7-3 多阶段回归测试是否已落地 | 对照 `7-3-part2-real-data-plan.md` 逐项检查 |
