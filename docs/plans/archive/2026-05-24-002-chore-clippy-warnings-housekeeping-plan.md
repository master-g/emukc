---
title: "Clippy 警告清理与深层问题评估"
created: 2026-05-24
status: completed
type: chore
depth: lightweight
---

# Clippy 警告清理与深层问题评估

## 问题概述

`cargo clippy --workspace` 产生 34 个警告，分布在 4 个 crate 中。需要评估每类警告是否为安全的机械修复，还是暴露了更深层的代码质量问题。

## 警告分类总览

| 类别 | 数量 | 严重性 | 评估 |
|------|------|--------|------|
| doc missing backticks | 10 | P3 | 纯机械修复 |
| match arms identical bodies | 10 | P2 | 需确认：是故意合并还是遗漏了分支逻辑 |
| let...else 可重写 | 7 | P3 | 机械修复 |
| collapsible if | 4 | P3 | 机械修复 |
| semicolon 缺失 | 1 | P3 | 机械修复 |
| too_many_arguments (13/7) | 1 | P2 | 设计气味，但不紧急 |
| dead_code (token 未读) | 1 | P2 | WIP 代码，非 bug |

## 深层问题分析

### 1. `match_same_arms` — 10 个 battle crate 中的相同 match 分支

**位置：** `emukc_battle/src/simulation/day_cutin.rs` (3), `night.rs` (6), `special_attack.rs` (1)

**风险：** 这些相同分支可能表示：
- (A) 故意合并 — 两种 CI 类型/夜战特殊攻击确实应产生相同结果
- (B) 遗漏 — 其中一个分支应该在后续实现中分化行为

**决策：** 逐个检查。如果确认是故意合并，用 `|` 模式合并消除警告；如果是遗漏，记录为 TODO。之前 battle CI audit 已确认了这些分支的正确性，预期大多数是 (A)。

### 2. `too_many_arguments` — `shelling.rs:push_attack` 有 13 个参数

**位置：** `crates/emukc_battle/src/simulation/shelling.rs:207`

**分析：** 函数接收 7 个 `&mut Vec` 输出缓冲区 + 6 个输入参数。函数体仅做 push 操作，逻辑简单。虽然参数数量是设计气味，但：
- battle 代码刚经过 CI audit 修复，重构有回归风险
- 函数行为简单明确，无复杂控制流

**决策：** 本次不重构。在函数上添加 `#[allow(clippy::too_many_arguments)]` 抑制警告。如果未来 `shelling` 模块需要扩展，再考虑引入 builder 或参数结构体。

### 3. `dead_code` — `PaymentSession.token` 从未被读取

**位置：** `src/bin/state/payment_store.rs:10`

**分析：** `token` 字段在 `payment_create` 中被写入，但后续 `confirm_payment`、`cancel_payment`、`payment.html` 均不读取它。`payment.html` 模板中的 token 来自 `GameSession.token`，非 `PaymentSession.token`。

这是支付系统 WIP 的残留 — 可能原本计划用于 confirm/cancel 时的 token 校验，但尚未实现。

**决策：** 添加 `#[expect(dead_code)]` 标注意图。支付系统完成时再做 token 校验或移除字段。

## 实施单元

### U1. 机械修复 32 个 clippy 警告

**Goal：** 清除所有可安全自动修复的警告。

**Files：**
- `crates/emukc_battle/src/damage.rs`
- `crates/emukc_battle/src/simulation/day_cutin.rs`
- `crates/emukc_battle/src/simulation/night.rs`
- `crates/emukc_battle/src/simulation/shelling.rs`
- `crates/emukc_battle/src/simulation/special_attack.rs`
- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`
- `crates/emukc_bootstrap/src/make_list/source/kcs/sound_rules.rs`
- `crates/emukc_gameplay/src/game/map_route.rs`
- `src/bin/net/router/game.rs`
- `src/bin/net/router/social/confirm_payment.rs`
- `src/bin/net/router/social/payment_create.rs`
- `src/bin/state/payment_store.rs`

**Approach：**
1. 运行 `cargo clippy --fix --workspace --allow-dirty` 自动修复机械类警告 (backticks, let...else, collapsible if, semicolon)
2. 手动检查 `match_same_arms` — 将相同分支用 `|` 模式合并
3. `push_attack` 添加 `#[allow(clippy::too_many_arguments)]`
4. `PaymentSession.token` 添加 `#[expect(dead_code)]`
5. `cargo fmt --all` 确保格式一致

**Patterns to follow：** 项目现有代码风格 — soft tabs, `#[expect]` 优先于 `#[allow]`（当字段确实有未来用途时）。

**Test scenarios：**
- `cargo clippy --workspace` 输出 0 warnings
- `cargo test --workspace` 全部通过
- `cargo fmt --all --check` 无格式差异

**Verification：** `cargo clippy --workspace` 返回 "No issues found"。

## 范围边界

**本次不做：**
- 重构 `push_attack` 参数为结构体（低优先级，battle 代码刚审计过）
- 实现 `PaymentSession.token` 校验逻辑（支付系统 WIP）
- 添加新的 clippy lint 配置（当前默认配置足够）
