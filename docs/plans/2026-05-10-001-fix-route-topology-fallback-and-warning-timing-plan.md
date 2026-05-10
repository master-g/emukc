---
title: fix: 路由拓扑不匹配回退、警告时机、客户端崩溃
type: fix
status: active
date: 2026-05-10
supersedes: 2026-05-09-001-fix-route-topology-mismatch-fallback-plan.md
---

# fix: 路由拓扑不匹配回退、警告时机、客户端崩溃

## 摘要

地图路由系统三个相关问题：(1) `evaluate_route_destination` 在路由规则不匹配 `next_cells` 拓扑时返回错误，而非回退到随机选择，导致客户端崩溃；(2) 拓扑验证警告在运行时（codex 加载）发出，而非在 bootstrap 数据处理阶段；(3) 运行时错误 `"cell X has no executable route in topology"` 与第 1 点根因相同。

---

## 问题背景

地图 1-3（map_id 13）的条件路由规则的 `to_cell_no` 目标是有效单元格，但不在 `next_cells` 中（kcdata 拓扑仅记录无条件边）。当 gameplay 层评估路由时，匹配的规则全部被 `next_cells.contains()` 过滤掉，`evaluate_route_destination` 返回 `Err` 而非回退到 `select_route_from_cells`。客户端收到 `svdata={"api_result":-1,...}` 后崩溃。

另外，`RuleTargetNotInNextCells` 警告每次服务器启动（codex 加载）时都会发出。这些是数据质量信号，应属于 bootstrap 阶段，而非运行时。

---

## 需求

- R1. 路由规则过滤后候选为零时，通过 `select_route_from_cells` 回退到 `next_cells` 随机选择，而非返回错误
- R2. 拓扑验证警告（`SelfLoop`、`Unreachable`、`RuleTargetNotInNextCells`）在 bootstrap 阶段发出，而非每次 codex 加载时
- R3. 客户端因 "cell X has no executable route in topology" 崩溃的问题得到解决（与 R1 相同修复）

---

## 范围边界

- Bootstrap 流水线验证逻辑不变 — kcdata 仍为权威拓扑来源
- `evaluate_route_candidate_count` 已正确处理回退（候选为空时返回 `next_cells.len()`）— 无需修改
- `MapVariantDefinition` 的 `parse_warnings` 字段存在但未使用；本计划不复用它
- 已有计划 `2026-05-09-001` 被本计划取代

---

## 上下文与研究

### 相关代码与模式

- `crates/emukc_gameplay/src/game/map_route.rs` — 路由评估，`evaluate_route_destination` 和 `evaluate_route_candidate_count`
- `crates/emukc_model/src/codex/map.rs` — `MapDefinition::validate()`、`MapValidationWarning` 枚举
- `crates/emukc_model/src/codex/mod.rs:252-267` — codex 加载时的运行时验证循环
- `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` — 最终地图目录组装，`MapCatalogBuildReport`
- `crates/emukc_bootstrap/src/map_pipeline/report.rs` — `MapCatalogBuildReport` 结构体，含 `fanout_rules_dropped` 字段
- `select_route_from_cells` 已实现所需的回退行为：从 `next_cells` 随机选择

### 关键数据

地图 13（1-3）失败规则：
- Cell 1 → cell 2：`next_cells=[4,5]`，规则目标 cell 2（经 1→4→2 可达，非直接边）
- Cell 3 → cells 4,8：`next_cells=[6]`，规则目标非直接可达
- Cell 8 → cell 4：`next_cells=[7,9,13]`，规则目标 cell 4 非直接可达

---

## 关键技术决策

- **KD1**: 复用 `select_route_from_cells` 作为回退 — 已测试，已处理单目标/多目标/空目标场景
- **KD2**: 将验证移至 bootstrap 的 `assemble_final_map_catalog` — 数据处理时发出一次警告，向 `MapCatalogBuildReport` 添加验证计数，移除 codex 加载时的运行时验证循环
- **KD3**: `MapValidationWarning` 枚举保留在 `emukc_model` — 数据模型的正确归属，bootstrap 只是更早调用它

---

## 待解决问题

### 规划阶段已解决

- **第 1 点和第 3 点是否相关？** 是。错误 `"cell 3 has no executable route in topology"` 是 `map_route.rs:261` 的精确错误字符串，由相同根因触发（规则过滤后候选为零）。

### 延迟到实现阶段

- 无

---

## 实现单元

### U1. 修复 evaluate_route_destination 回退逻辑

**目标：** 拓扑过滤后 `candidate_targets` 为空时，将错误返回替换为 `select_route_from_cells` 回退。

**需求：** R1, R3

**依赖：** 无

**文件：**
- 修改：`crates/emukc_gameplay/src/game/map_route.rs`

**方案：**

在 `evaluate_route_destination` 第 ~259-263 行，将：
```rust
if candidate_targets.is_empty() {
    return Err(GameplayError::WrongType(format!(
        "cell {} has no executable route in topology",
        current.cell_no,
    )));
}
```
替换为：
```rust
if candidate_targets.is_empty() {
    return select_route_from_cells(current, stage, selected_cell_id);
}
```

这与函数中已有的不确定规则处理方式一致（第 ~246 行：`return select_route_from_cells(current, stage, selected_cell_id)`）。

**测试场景：**
- 正常路径：匹配规则且目标在 `next_cells` 中 → 加权随机选择
- 边界情况：匹配规则但全部被过滤，`next_cells` 非空 → 回退随机选择成功
- 边界情况：匹配规则被过滤，`next_cells` 为空 → 通过 `select_route_from_cells` 返回错误
- 集成测试：模拟地图 13 cell 3 场景（规则引用不可达单元格），验证返回有效 `next_cells` 目标

**验证：**
- `cargo test -p emukc_gameplay`
- 新测试用例覆盖回退路径

---

### U2. 将拓扑验证从 codex 加载移至 bootstrap

**目标：** 在 bootstrap 组装阶段发出 `MapValidationWarning`，而非每次 codex 加载时。向 `MapCatalogBuildReport` 添加警告计数。

**需求：** R2

**依赖：** 无

**文件：**
- 修改：`crates/emukc_bootstrap/src/map_pipeline/assemble.rs`
- 修改：`crates/emukc_bootstrap/src/map_pipeline/report.rs`
- 修改：`crates/emukc_model/src/codex/mod.rs`

**方案：**

1. **向 `MapCatalogBuildReport` 添加 `topology_warnings` 字段**（`report.rs`）：
   - `topology_warnings: usize` — 验证警告计数
   - 更新 `Display` 实现，计数 > 0 时包含该值

2. **在 `assemble_final_map_catalog` 中运行验证**（`assemble.rs`）：
   - 组装最终目录后，遍历 `catalog.maps.values()` 并调用 `def.validate()`
   - 通过 `tracing::warn!` 记录每条警告
   - 记录含总计数的摘要行
   - 将计数存入 `MapCatalogBuildReport`

3. **移除 codex 加载时的运行时验证循环**（`codex/mod.rs`）：
   - 删除第 252-267 行的验证代码块

**遵循模式：**
- `MapCatalogBuildReport` 中已有的 `fanout_rules_dropped` 模式 — 拓扑警告采用相同方式

**测试场景：**
- 正常路径：拓扑有效的地图 → `topology_warnings: 0`，无警告日志
- 数据质量：规则不匹配的地图 → bootstrap 期间记录警告，报告含计数
- 回归：codex 加载不再在服务器启动时发出 `RuleTargetNotInNextCells` 警告

**验证：**
- `cargo test -p emukc_bootstrap`
- `cargo run -- bootstrap` 在 bootstrap 期间显示拓扑警告（如有）
- `cargo run`（serve）启动时不再发出 `RuleTargetNotInNextCells` 警告

---

## 系统级影响

- **交互图：** 仅 `evaluate_route_destination` 调用路径变化 — 无新回调或副作用
- **错误传播：** 移除一个错误路径，替换为回退。客户端不再在地图 1-3 崩溃
- **状态生命周期风险：** 无 — 回退使用无状态的 `select_route_from_cells`
- **API 接口一致性：** 无其他接口需修改
- **不变量：** `next_cells` 仍为权威拓扑；路由规则仍为条件覆盖；`evaluate_route_candidate_count` 已正确处理回退

---

## 风险与依赖

| 风险 | 缓解措施 |
|------|----------|
| 回退隐藏数据质量问题 | Bootstrap 阶段警告（U2）在数据处理时暴露问题 |
| 回退选择错误单元格 | `select_route_from_cells` 已在其他代码路径使用并有测试覆盖 |
| Bootstrap 警告在 CI 中被忽略 | `MapCatalogBuildReport` 中的警告计数会被记录并在 bootstrap 输出中可见 |
