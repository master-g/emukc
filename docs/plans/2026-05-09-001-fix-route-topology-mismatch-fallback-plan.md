---
title: fix: Route topology mismatch should fallback to random, not block gameplay
type: fix
status: active
date: 2026-05-09
---

# fix: Route topology mismatch should fallback to random, not block gameplay

## Summary

当 wikiwiki routing_rules 目标单元格存在但不在 kcdata `next_cells` 有向边中时，gameplay 层应回退到 `next_cells` 平均概率随机路由，而非返回错误阻断游玩。

---

## Problem Frame

**设计意图**:
1. kcdata 提供权威地图 topology（有向图 `next_cells`）
2. wikiwiki 作为路由规则来源（`routing_rules`）
3. bootstrap 层合并二者，规则不匹配 topology 时输出警告
4. gameplay 层执行路由，规则不匹配时使用默认随机路由，不阻断游玩

**当前行为**:
- `map_route.rs:148, 257` 过滤掉 `to_cell_no` 不在 `next_cells` 的规则
- `map_route.rs:259-263` 过滤后无候选时返回错误
- 结果：游玩被阻断

**期望行为**:
- 规则不匹配时回退到 `select_route_from_cells` 使用 `next_cells` 平均概率随机路由

---

## Requirements

- R1. 当 `candidate_targets` 为空时，不返回错误，而是回退到 `next_cells` 随机选择
- R2. 保持 `RuleTargetNotInNextCells` 警告输出（数据质量信号）
- R3. 不修改 bootstrap 层逻辑（kcdata 仍为权威 topology）

---

## Scope Boundaries

- **In scope**: `map_route.rs` 中 `evaluate_route_destination` 和 `evaluate_route_candidate_count` 的回退逻辑
- **Out of scope**: bootstrap 层验证逻辑、kcdata 解析、wikiwiki 解析

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_gameplay/src/game/map_route.rs:319-358` — `select_route_from_cells` 已实现 `next_cells` 随机选择逻辑
- `crates/emukc_gameplay/src/game/map_route.rs:148` — `evaluate_route_candidate_count` 过滤逻辑
- `crates/emukc_gameplay/src/game/map_route.rs:254-263` — `evaluate_route_destination` 过滤和错误返回逻辑

### Existing Pattern

`select_route_from_cells` 函数已实现期望的回退行为：
- 空 `next_cells` → 错误
- 单个目标 → 直接返回
- 多个目标 → 随机选择

---

## Key Technical Decisions

- **KD1**: 复用现有 `select_route_from_cells` 函数作为回退路径，而非重新实现随机逻辑

---

## Implementation Units

### U1. Fix evaluate_route_destination fallback

**Goal:** 当 routing_rules 过滤后无候选目标时，回退到 `next_cells` 随机选择

**Requirements:** R1, R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`
- Test: `crates/emukc_gameplay/src/game/map_route.rs` (同文件内测试模块)

**Approach:**

修改 `evaluate_route_destination` 函数第 259-263 行：

```rust
// Before:
if candidate_targets.is_empty() {
    return Err(GameplayError::WrongType(format!(
        "cell {} has no executable route in topology",
        current.cell_no,
    )));
}

// After:
if candidate_targets.is_empty() {
    // Routing rules don't match topology — fallback to next_cells random selection
    return select_route_from_cells(current, stage, selected_cell_id);
}
```

**Test scenarios:**

- **Happy path**: 规则匹配 topology 时正常执行加权随机选择
- **Edge case**: `candidate_targets` 为空但 `next_cells` 非空时，回退到随机选择
- **Edge case**: `candidate_targets` 为空且 `next_cells` 为空时，返回错误
- **Integration**: 模拟 map 13 cell 1 场景（规则引用不存在的边），验证不阻断

**Verification:**
- `cargo test -p emukc_gameplay` 通过
- 新测试用例覆盖回退路径

---

### U2. Fix evaluate_route_candidate_count fallback

**Goal:** 当 routing_rules 过滤后无候选目标时，返回 `next_cells.len()` 而非 `candidate_targets.len()`

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`

**Approach:**

修改 `evaluate_route_candidate_count` 函数第 145-155 行：

```rust
// Before:
let candidate_targets: BTreeSet<i64> = executable
    .iter()
    .map(|rule| rule.to_cell_no)
    .filter(|cell_no| current.next_cells.contains(cell_no))
    .collect();

if candidate_targets.is_empty() {
    current.next_cells.len()
} else {
    candidate_targets.len()
}

// After: (逻辑已正确，但注释说明这是 fallback 行为)
if candidate_targets.is_empty() {
    // Routing rules don't match topology — fallback to next_cells count
    current.next_cells.len()
} else {
    candidate_targets.len()
}
```

**Note:** 此函数当前行为已正确 — 返回 `next_cells.len()` 作为候选数。仅需添加注释说明意图。

**Test scenarios:**

- **Happy path**: 规则匹配时返回 `candidate_targets.len()`
- **Edge case**: 规则不匹配时返回 `next_cells.len()`

**Verification:**
- `cargo test -p emukc_gameplay` 通过

---

## System-Wide Impact

- **Interaction graph**: 仅影响 `evaluate_route_destination` 调用路径
- **Error propagation**: 移除一个错误返回路径，改为回退
- **State lifecycle risks**: 无
- **API surface parity**: 无其他接口需同步修改
- **Unchanged invariants**: `next_cells` 仍为权威 topology；`routing_rules` 仍为条件路由规则

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| 回退逻辑隐藏数据质量问题 | 保持 `RuleTargetNotInNextCells` 警告输出 |
| 回退到错误的路径 | `select_route_from_cells` 已有完整测试覆盖 |

---

## Verification

```bash
# Run gameplay tests
cargo test -p emukc_gameplay

# Run integration tests
cargo test --test gameplay_tests
```
