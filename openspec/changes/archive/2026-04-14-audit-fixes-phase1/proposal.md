## Why

代码审计（docs/audit.md）确认了 5 个 bug 和多处测试覆盖缺口。这些问题影响战斗正确性（夜战伤害公式、沉船保护基数、路由选择偏置）、地图可玩性（EO 地图无法解锁）以及服务可靠性（Mutex poisoning 可锁死玩家）。本次变更集中修复所有已确认 bug，并补齐关键测试。

## What Changes

- **修复路由选择溢出偏置**：`select_route_target_for_roll` 在 roll 等于总权重时回退到首个键而非末尾键，导致概率质量分配不均。改为 `.last()` 与 `select_enemy_composition_for_roll` 一致。
- **移除夜战交战形态修正**：`calculate_night_damage` 错误地乘以 `engagement.modifier()`，与舰 C 文档冲突（夜战无阵形/交战形态修正）。移除该乘法。
- **修正沉船保护基数**：`apply_damage` 的保护公式使用 `current_hp` 而非 `entry_hp`，导致已受损舰船保护池缩小。改为 `entry_hp`。
- **实现 EO 地图先决条件**：`build_regular_prerequisites` 仅覆盖 no 2..=4，EO 地图（N-5, N-6 等）无解锁路径。补充 EO 解锁链。
- **替换 std::sync::Mutex 为 parking_lot::Mutex**：SortieStore 的 Mutex poisoning 可导致单个 panic 永久锁死玩家出击功能。
- **将浮点伤害公式改为纯整数运算**：消除跨平台确定性风险。
- **补齐测试**：路由溢出、EO 解锁、沉船保护、端到端 Boss 胜利→解锁。

## Capabilities

### New Capabilities

_None_

### Modified Capabilities

- `sortie`: 修正路由选择、夜战公式、沉船保护基数、Mutex 类型；影响 SortieOps 和 MapOps 行为

## Impact

- `crates/emukc_gameplay/src/game/map_route.rs` — 路由选择修复
- `crates/emukc_gameplay/src/game/battle/core.rs` — 夜战公式、沉船保护、整数伤害
- `crates/emukc_gameplay/src/game/sortie_store.rs` — Mutex 替换
- `crates/emukc_model/src/codex/map.rs` — EO 先决条件
- `crates/emukc_gameplay/tests/` — 新增测试
- `Cargo.toml` — 新增 `parking_lot` 依赖（如尚未引入）
