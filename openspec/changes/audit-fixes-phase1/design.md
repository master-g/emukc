## Context

代码审计确认 5 个 bug 和测试缺口。这些 bug 分布在 map_route（路由选择）、battle/core（夜战公式、沉船保护）、codex/map（EO 先决条件）、sortie_store（Mutex 类型）四个模块。所有修复均为局部改动，不涉及架构变更。

## Goals / Non-Goals

**Goals:**

- 修复 5 个已确认 bug，使行为与舰 C 文档/参考实现一致
- 将浮点伤害公式转为纯整数运算以保证跨平台确定性
- 补齐路由溢出、EO 解锁、沉船保护的单元/集成测试
- 消除 Mutex poisoning 导致玩家永久锁死的风险

**Non-Goals:**

- 不改进伤害公式精度（改修强化值、空母公式、CL 轻炮补正等留待后续 phase）
- 不实现联合舰队、LBAS、特殊舰 OASW 等未实现功能
- 不重构 SortieStore 为持久化存储（仅替换 Mutex 类型）
- 不处理 Clippy 警告中的非致命问题
- 不回退格式化变更

## Decisions

### D1: 路由溢出修复 — 使用 `.last()` 替代 `.next()`

`select_route_target_for_roll` 在 roll >= 总权重时回退到第一个键。参考 `select_enemy_composition_for_roll`（sortie.rs:1491）使用 `.last()`。直接对齐即可。

**替代方案**：增加 `if roll == 0 { return first }` 特判。但 `.last()` 语义更清晰，且与参考实现一致。

### D2: 夜战公式 — 直接移除 `* engagement.modifier()`

舰 C 文档明确夜战无交战形态修正。一行移除，无歧义。

### D3: 沉船保护基数 — 使用 `entry_hp`

`apply_damage` 中 taiha 判定已正确使用 `entry_hp`（line 196），仅保护公式基数使用了 `current_hp`。统一为 `entry_hp`，保证同一场战斗中先受伤的舰船不被不公平对待。

### D4: EO 先决条件 — 在 `build_regular_prerequisites` 中扩展循环

当前循环 `no in 2..=4` 仅覆盖主图。扩展为遍历所有已定义地图，按 area 分组，对每个 area 内的地图按 no 排序建立解锁链。需查询 Codex 中该 area 下已定义的 map 数量。

### D5: Mutex 替换 — `parking_lot::Mutex`

`std::sync::Mutex` 在 panic 时 poison，后续 lock 都会失败。`parking_lot::Mutex` 不 poison，且性能更好。替换后移除所有 `.unwrap()` 改为 `.lock()` 直接返回 MutexGuard（parking_lot 的 lock 返回非 Result）。

**替代方案**：`tokio::sync::RwLock`。不选——SortieStore 操作都是短临界区，不需要异步锁，且 parking_lot 的 `Mutex` 在同步场景性能最优。

### D6: 浮点→整数 — 保护公式重写

原公式 `floor(0.5 * h + 0.3 * rand_part)` → 整数版 `(h / 2) + (rand_part * 3) / 10`。语义相同，消除 f64 舍入差异。

### D7: 测试策略

- 路由溢出：单元测试 `select_route_target_for_roll` roll=总权重
- EO 解锁：单元测试 `prerequisite_for(15)` 等返回正确先决
- 沉船保护：单元测试 `apply_damage` 覆盖旗舰/非旗舰/大破入场
- 夜战公式：已有测试修正期望值
- Mutex：编译通过即可

## Risks / Trade-offs

- **[EO 解锁链完整性]** → Codex 数据中若某 area 的 EO 地图有跳号（如 1-5 存在但 1-4 不存在），需确认解锁链是否仍正确。缓解：EO 解锁依赖同 area 上一号地图（1-5 → 1-4），而非固定路径。
- **[整数公式精度]** → `(h/2) + (rand_part*3)/10` 在小数值时与原浮点公式可能有 ±1 差异。缓解：差异在 ±1 范围内，对游戏体验无影响。
- **[parking_lot 依赖]** → 新增 `parking_lot` crate 依赖。缓解：parking_lot 是 Rust 生态标准选择，被广泛使用。
