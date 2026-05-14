---
title: "test: 地图系统综合集成测试（路由、敌人配置、战斗流程）"
type: refactor
status: completed
date: 2026-05-10
---

# test: 地图系统综合集成测试（路由、敌人配置、战斗流程）

## Summary

为 map → route → enemy fleet → battle 管道创建三层集成测试：路由规则评估（条件路由、拓扑回退、next_cells 优先级）、敌人配置（舰队选择、后备逻辑、composition 权重）、战斗端到端流程（HP 追踪、大破保护、display_damage 一致性）。测试复用现有 test_utils 基础设施，不引入新框架。

---

## Requirements

- R1. 路由规则测试覆盖：条件路由匹配、LoS 分流、拓扑过滤回退、未知 predicate 降级、规则优先级排序
- R2. 敌人配置测试覆盖：cell → fleet 映射、后备 composition、加权随机选择、空数据边界
- R3. 战斗流程端到端测试覆盖：完整日夜战流程、HP 追踪、大破保护生效、各阶段 display_damage 一致性（炮击/雷击/航空战）
- R4. 所有测试使用现有基础设施（test_utils、SeededRng、sample_ship、make_cell），不创建新测试框架

---

## Scope Boundaries

- 不重构现有测试（sortie_tests.rs、map_route.rs 内测试）
- 不添加 wikiwiki parser 或 bootstrap pipeline 测试
- 不添加演习/远征战斗测试
- 不创建新的 test utility crate
- 不修改生产代码（除非测试需要新增 pub 可见性）

### Deferred to Follow-Up Work

- 多 gauge 地图阶段转换测试（已有部分覆盖于 sortie_tests.rs）
- 多 gauge HP 计量器状态机测试（已有部分覆盖于 sortie_tests.rs）
- 地图解锁前置条件链测试（已有覆盖于 tests/gameplay_tests/map/unlock.rs）

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_battle/src/test_utils.rs` — sample_ship、first_ship_mst_by_type、make_test_ship 等辅助函数
- `crates/emukc_gameplay/src/game/map_route.rs:832-1694` — 现有路由测试及 make_cell、make_los_context 辅助函数
- `crates/emukc_gameplay/src/game/sortie_tests.rs` — 现有 sortie 测试及 enemy_test_codex、weaken_for_midnight 辅助函数
- `crates/emukc_battle/src/simulation/kouku.rs` — 航空战测试模式
- `crates/emukc_battle/src/simulation/mod.rs:281-517` — 战斗模拟测试模式
- `tests/gameplay_tests/map/unlock.rs` — 集成测试模式（new_context + new_profile + HasContext）

### Institutional Learnings

- 集成测试断言必须验证具体状态，不能只用 `is_ok()`/`is_err()`
- 使用 `TestSortieStore`（非 GLOBAL_SORTIE_STORE）实现测试隔离
- 地图路由非确定性需要 retry loop 或固定种子 RNG
- 战斗验证是诊断工具，非运行时检查 — 测试需要显式调用

---

## Key Technical Decisions

- **KD1**: 路由和敌人配置测试作为 crate-internal `#[cfg(test)]` 模块 — 不需要数据库，隔离性好，速度快
- **KD2**: 战斗端到端测试作为 crate-internal 模块（emukc_battle）— 只需 Codex + SeededRng，无需数据库
- **KD3**: 全链路集成测试（sortie → battle → result → next cell）放在 `tests/gameplay_tests/map/` — 需要 DB + Codex
- **KD4**: 使用 `SeededRng::new(seed)` 确保战斗结果确定性 — 可重复测试

---

## Implementation Units

### U1. 路由规则集成测试

**Goal:** 为 map_route 模块补充集成级路由评估测试，覆盖条件路由、拓扑过滤、LoS 分流的完整流程。

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`（测试模块）

**Approach:**

在现有 `map_route.rs` 测试模块中新增测试组。复用已有的 `make_cell`、`make_los_context`、`make_variant` 辅助函数。构建包含多规则、多分支的完整 stage definition，验证路由评估的端到端行为。

**Patterns to follow:**
- `map_route.rs:832-1694` — 现有路由测试模式
- `make_cell(cell_no, next_cells)` — cell 构造辅助函数

**Test scenarios:**

Happy path:
- 多条件规则按优先级匹配：高优先级规则命中时忽略低优先级规则
- LoS 分流：不同 LoS 值路由到不同目标 cell
- 规则 from_cell 匹配当前 cell 时正确选中

Edge cases:
- 所有规则被拓扑过滤后回退到 `select_route_from_cells` — 验证返回 next_cells 中的合法目标
- `next_cells` 为空且无规则匹配 — 验证返回错误
- 多条规则有相同优先级 — 验证加权随机选择
- 路由规则引用的 to_cell_no 不在 next_cells 中但存在于 cells — 验证拓扑过滤正确排除

Integration:
- 构建模拟地图 13 的 stage（cell 1→{4,5}, cell 3→{6}），验证条件路由 + 拓扑回退的组合行为

**Verification:**
- `cargo test -p emukc_gameplay map_route`
- 所有新测试通过

---

### U2. 敌人配置集成测试

**Goal:** 为敌人舰队选择和后备逻辑补充测试，覆盖 cell 映射、composition 权重、空数据边界。

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie/enemy_ship.rs`（测试模块，需新增 `#[cfg(test)]`）

**Approach:**

在 `enemy_ship.rs` 新增 `#[cfg(test)] mod tests` 模块。复用 `sortie_tests.rs` 中的 `enemy_test_codex` 模式构建测试 Codex。测试 `resolve_sortie_enemy_fleet`、`select_random_enemy_composition`、`build_sortie_enemy_ships` 的完整行为。

**Patterns to follow:**
- `sortie_tests.rs` — `enemy_test_codex()` 辅助函数模式
- `sortie_tests.rs` — `build_sortie_enemy_ship` 测试模式

**Test scenarios:**

Happy path:
- `resolve_sortie_enemy_fleet` 在 cell 有 enemy_fleet 数据时返回正确舰队
- `select_random_enemy_composition` 按权重选择 composition（固定种子验证确定性）
- `build_sortie_enemy_ships` 从 composition.ship_ids 创建正确的敌人列表

Edge cases:
- cell 不在 enemy_fleets 中 → 后备舰队使用深渊驱逐イ级（ID 1501），不使用友方舰娘 ID
- composition.ship_ids 为空 → 使用后备 ID 1501
- `build_sortie_enemy_ship` 对深渊 ID 使用 `new_enemy_ship` 路径
- `build_sortie_enemy_ship` 对非深渊 ID 使用 `new_ship` 路径并发出 warn

Integration:
- 构建完整 MapVariantDefinition（含 cells、routing_rules、enemy_fleets），验证从 variant 获取敌人舰队的端到端流程
- 验证后备敌人舰队的 ship_ids 全部可通过 `new_enemy_ship` 成功创建

**Verification:**
- `cargo test -p emukc_gameplay enemy`
- 所有新测试通过

---

### U3. 战斗 display_damage 一致性测试

**Goal:** 验证炮击、雷击、航空战三个阶段对友方舰船的 display_damage 行为一致 — 保护生效时返回 dealt 而非 raw。

**Requirements:** R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/kouku.rs`（测试模块）

**Approach:**

在 `kouku.rs` 现有测试模块中新增测试。使用 `test_utils::sample_ship` 和 `SeededRng`。构造低 HP 友方舰船 + 高伤害场景，验证大破保护生效时 api_fdam 反映实际扣除 HP。

**Execution note:** test-first — 先写失败测试，再配合 bug fix plan (2026-05-10-002 U2) 使其通过。

**Patterns to follow:**
- `kouku.rs:455-638` — 现有航空战测试模式
- `simulation/mod.rs:281-517` — 战斗模拟测试模式

**Test scenarios:**

Happy path:
- 友方舰船满 HP 时航空战 `api_fdam` 等于实际 HP 减少
- 敌方舰船 `api_edam` 可以超过原始 HP（过量击杀显示）

Edge cases:
- 友方舰船大破状态（entry_hp * 4 <= maxhp）下航空战：`api_fdam < raw_damage`（保护生效）
- 友方旗舰（index 0）永远不会被击沉：航空战后 `hp > 0`
- 友方非旗舰、非大破进入：航空战不致死

Integration:
- 完整 `simulate_day` 流程（含航空战 + 炮击 + 雷击）：验证所有阶段对同一友方舰船的 display_damage 一致（全部使用 dealt 而非 raw）
- 构造多轮航空战攻击同一目标：验证 api_fdam 累加的是 dealt 值

**Verification:**
- `cargo test -p emukc_battle kouku`
- `cargo test -p emukc_battle display_damage`

---

### U4. 战斗端到端流程测试

**Goal:** 验证完整战斗流程（日战 + 夜战）的 HP 追踪、大破保护、胜负判定。

**Requirements:** R3, R4

**Dependencies:** U3

**Files:**
- Modify: `crates/emukc_battle/src/simulation/mod.rs`（测试模块）

**Approach:**

在现有 `simulation/mod.rs` 测试模块中新增端到端流程测试。使用 `sample_ship` 构建完整舰队，`SeededRng` 确保确定性。验证 `simulate_day` 和 `simulate_night` 的完整输出。

**Execution note:** test-first — 先写覆盖保护逻辑的端到端测试。

**Patterns to follow:**
- `simulation/mod.rs:281-517` — 现有 simulate_day 测试模式

**Test scenarios:**

Happy path:
- 日战完整流程：kouku → shelling → torpedo，所有阶段 HP 变化总和等于 `api_fdam` + `api_frai` 等
- 日战后双方存活 → `midnight_possible = true`
- 夜战后 HP 正确更新

Edge cases:
- 友方舰船大破状态下完整日战：结束后所有友方舰船 `hp > 0`（保护生效）
- 友方旗舰永远不会被击沉（即使致命伤害）
- 一方全灭：`BattleOutcome` 正确反映胜/败
- 空敌人舰队：simulate_day 不 panic

Integration:
- 日战 + 夜战连续执行：HP 在两阶段间正确传递
- 友方舰队含 CVL（有飞机）：kouku 阶段正常执行，api_fdam 反映保护后的实际伤害
- 敌方舰队全灭后不再受后续攻击

**Verification:**
- `cargo test -p emukc_battle simulation`
- 所有新测试通过

---

### U5. 全链路集成测试（sortie → battle → result）

**Goal:** 在 integration test 层面验证从开始出击到战斗完成的完整链路。

**Requirements:** R3, R4

**Dependencies:** U1, U2, U3

**Files:**
- Create: `tests/gameplay_tests/map/sortie_battle.rs`
- Modify: `tests/gameplay_tests/map/mod.rs`

**Approach:**

新建 `sortie_battle.rs` 集成测试文件。使用 `new_context() + new_profile()` 模式创建完整上下文。执行 start_sortie → sortie_battle → sortie_battle_result 链路，验证战斗结果状态正确。

**Patterns to follow:**
- `tests/gameplay_tests/map/unlock.rs` — 集成测试模式
- `tests/gameplay_tests/map/retreat.rs` — 撤退测试模式

**Test scenarios:**

Happy path:
- 地图 1-1 出击：start → battle → result → next → boss kill，验证 boss_cell 到达和 HP 状态
- 战斗结果包含正确的敌方舰船 ID（深渊栖舰，非友方舰娘）

Edge cases:
- 地图 1-1 出击到达 boss 前的普通 cell：验证 enemy_fleet 正确加载
- 战斗后舰船 HP 低于出击前（验证 HP 持久化）
- 大破舰船在 sortie 中不会被击沉（保护生效）

**Verification:**
- `cargo test --test gameplay_tests map::sortie_battle`
- 所有新测试通过

---

## System-Wide Impact

- **Interaction graph:** 仅添加测试代码，不修改生产逻辑。若 enemy_ship.rs 需要新增 `#[cfg(test)]` 模块，仅增加 pub 可见性用于测试
- **Error propagation:** 无变化
- **State lifecycle risks:** 无 — 测试使用 in-memory DB 和 TestSortieStore
- **API surface parity:** 无变化
- **Integration coverage:** 本计划即集成覆盖的补充

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| 测试依赖 `.data/codex` 目录存在 | 现有测试已有此依赖，CI 环境应已 bootstrap |
| 航空战测试需先修复 display_damage bug（plan 2026-05-10-002 U2） | U3 标记为 test-first，测试先写，配合 bug fix 通过 |
| 集成测试路由非确定性需 retry loop | 使用固定种子 RNG 或 retry loop 模式（已有先例） |
| 新增测试可能增加 CI 时间 | crate-internal 测试无 DB 开销，集成测试数量少 |
