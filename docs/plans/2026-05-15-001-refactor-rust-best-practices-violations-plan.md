---
title: Refactor: Fix Rust Best Practices Violations
type: refactor
status: completed
date: 2026-05-15
---

# Refactor: Fix Rust Best Practices Violations

## Summary

分阶段修复 Rust best practices 审计发现的问题。按优先级分 5 个 implementation unit：机械性 clippy warnings → `#[allow→expect]` 迁移 → 急切求值模式修复 → library crate 中 `panic!`/`unreachable!` 替换 → `// TODO` 格式规范化。`unwrap`/`expect` 全量迁移不纳入本计划范围（764 处），仅处理 library crate 中最危险的实例。

---

## Problem Frame

Rust best practices 审计发现 2 个 clippy warnings、764 处 `unwrap`/`expect`、~9 处 production `panic!`/`unreachable!`、~19 处裸 `// TODO`、多处模式违规（`#[allow]`、急切求值、`&Vec<T>`、Copy 上 clone）。违反 Apollo Rust Best Practices Handbook Ch1-8 多条规则。

---

## Requirements

- R1. 清零所有 clippy warnings（当前 2 个，均在 `sortie/mod.rs`）
- R2. `#[allow(clippy::...)]` 迁移为 `#[expect(clippy::...)]` 并附理由注释
- R3. `map_or`/`ok_or` 急切求值改为 `_else` 变体
- R4. `&Vec<T>` 函数参数改为 `&[T]`
- R5. `.clone()` on Copy type 改为直接使用
- R6. 不必要的 `&mut` 引用降级为 `&`
- R7. Library crate 中 `panic!`/`unreachable!` 替换为 `Result` 传播或 `let ... else`
- R8. `// TODO` 加 issue 引用格式 `TODO(#xxx)`
- R9. 不引入新行为变更 — 纯重构

---

## Scope Boundaries

- 不做全量 `unwrap`/`expect` → `Result` 迁移（764 处，需独立计划）
- 不创建 GitHub issues（仅加格式）
- 不改变错误类型层次结构
- 不改动 binary crate（`src/bin/`）中的 `unwrap`/`expect`
- 不添加新依赖
- 不改公共 API 签名（除 `&Vec<T>` → `&[T]` 外部可见变更）

### Deferred to Follow-Up Work

- Binary crate `unwrap`/`expect` 逐步迁移：独立迭代
- `missing_docs` 覆盖率提升：独立计划

---

## Context & Research

### Relevant Code and Patterns

- Workspace lints 已配置于根 `Cargo.toml` `[workspace.lints]`
- Library crates 使用 `thiserror`，`anyhow` 仅限 examples
- `unsafe_code = "deny"` 已生效
- Gameplay `_impl` pattern：`_impl` 函数接受 `C: ConnectionTrait`，由 public trait 调用

### External References

- Apollo GraphQL Rust Best Practices Handbook (Ch1-9)

---

## Key Technical Decisions

- **Library crate 优先**：`emukc_battle`、`emukc_gameplay`、`emukc_model`、`emukc_bootstrap`、`emukc_cache` 中的违规先处理
- **Binary crate 宽松**：`src/bin/` 中 `unwrap`/`expect` 暂不动（Ch4 允许 binary 更宽松）
- **panic 替换策略**：`panic!("msg")` → 返回 `Result<T, E>` 或 `let ... else { return Err(...) }`，视调用链上下文而定
- **`&Vec<T>` → `&[T]` 是唯一外部可见签名变更**：调用者侧不需要改动（`&Vec<T>` auto-deref to `&[T]`）

---

## Implementation Units

### U1. Fix Remaining Clippy Warnings (2 items)

**Goal:** 清零 `cargo clippy --workspace` 输出中的全部 warnings

**Requirements:** R1, R5, R9

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie/mod.rs` — doc backticks (line 1254) + clone on Copy type (line 1350)

**Approach:**
- doc backticks: 修复 markdown 中缺少反引号的类型名
- clone on Copy: 去掉 `.clone()`

**Test scenarios:**
- Happy path: `cargo clippy --workspace` 返回 0 warnings

**Verification:**
- `cargo clippy --workspace 2>&1 | grep "warning:" | wc -l` 输出 0

---

### U2. Migrate `#[allow(...)]` to `#[expect(...)]`

**Goal:** 所有 `#[allow(clippy::...)]` 和 `#[allow(unused...)]` 改为 `#[expect(...)]` 并附理由注释

**Requirements:** R2, R9

**Dependencies:** None (可与 U1 并行)

**Files:**
- Modify: `src/lib.rs:14` — `#[allow(unused_imports)]`
- Modify: `crates/emukc_dylib/src/lib.rs:3-4` — `#[allow(unused_imports)]`, `#[allow(clippy::single_component_path_imports)]`
- Modify: `crates/emukc_cache/src/kache.rs:127` — `#[allow(clippy::result_large_err)]`
- Modify: 其他 `#[allow(...)]` 实例（需 grep 确认完整列表）

**Approach:**
- `#[allow(lint)]` → `#[expect(lint)]`
- 每处上方加一行注释说明原因，如 `// Reason: re-exported for downstream crate convenience`
- 确认 `#[expect]` 语义：如果 lint 不再触发，编译器会 warning（比 `#[allow]` 更安全）

**Test scenarios:**
- Happy path: `cargo clippy --workspace` 通过，无新 warnings
- Edge case: 如果某处 `#[allow]` 对应的 lint 实际不存在，`#[expect]` 会报错 → 此时删除该属性

**Verification:**
- `grep -rn '#\[allow(' --include='*.rs' crates/ src/ | grep -v '#\[expect'` 返回空（test 文件中的 `#[allow(dead_code)]` 除外）

---

### U3. Fix Eager Evaluation Patterns

**Goal:** `map_or`/`ok_or` 急切求值改为 `_else` 变体，`&Vec<T>` 改 `&[T]`

**Requirements:** R3, R4, R9

**Dependencies:** U1 (确保 clippy 基线干净)

**Files:**
- Modify: `crates/emukc_model/src/profile/ndock.rs` — 5 处 `map_or(0, ...)` 和 `map_or("0".to_owned(), ...)`
- Modify: `crates/emukc_model/src/profile/kdock.rs` — 8 处
- Modify: `crates/emukc_model/src/codex/ship.rs` — `map_or(1, ...)`
- Modify: `crates/emukc_bootstrap/src/parser/kc3kai/quote.rs` — `ok_or(ParseError::X("...".to_string()))`
- Modify: `crates/emukc_cache/src/kache.rs` — `ok_or(Error::MissingField("...".to_owned()))` + `&Vec<String>` 返回类型
- Modify: `crates/emukc_bootstrap/src/parser/kcwiki/ship.rs` — 多处 `map_or(0, ...)`

**Approach:**
- `.map_or(allocating_expr, f)` → `.map_or_else(|| allocating_expr, f)` 仅当默认值有分配（`String::new()`、`format!()`、`to_string()`、`to_owned()`）
- `.map_or(0, f)` 等字面量默认值 → 保持不动（无分配，`_else` 不必要）
- `.ok_or(ExprWithAlloc)` → `.ok_or_else(|| ExprWithAlloc)`
- `fn foo() -> &Vec<T>` → `fn foo() -> &[T]`（调用者侧 auto-deref 兼容）

**Test scenarios:**
- Happy path: `cargo test --workspace` 全部通过
- Edge case: 确认 `&Vec<T>` → `&[T]` 变更后所有调用者编译通过

**Verification:**
- `cargo test --workspace` 通过
- `cargo clippy --workspace` 无新 warnings

---

### U4. Replace `panic!`/`unreachable!` in Library Crates

**Goal:** Library crate 中的 `panic!` 和 `unreachable!` 替换为安全的 `Result` 传播

**Requirements:** R7, R9

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_gameplay/src/game/compose/slot_deprive.rs` — `unreachable!()` (production)
- Modify: `crates/emukc_gameplay/src/game/expedition.rs` — `unreachable!()` (production)
- Modify: `crates/emukc_gameplay/src/game/ship/mod.rs` — `unreachable!()` (production)
- Modify: `crates/emukc_gameplay/src/game/presets/slot.rs` — `unreachable!()` (production)
- Modify: `crates/emukc_battle/src/outcome.rs` — `panic!(...)` (debug-only, gated behind `#[cfg(debug_assertions)]`)
- Modify: `crates/emukc_model/src/thirdparty/quest/mod.rs` — `panic!` in `From<i64>` impls (public API concern)
- Modify: `crates/emukc_bootstrap/src/parser/tsunkit_quest/types.rs` — `panic!` (production)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/` — 多处 `panic!` (需区分 test/production)

**Approach:**
- 每处单独分析上下文，选择替换策略：
  - `unreachable!()` → `let ... else { return Err(...) }` 或保持 `unreachable!()` 如果确实逻辑上不可能（加注释说明理由）
  - `panic!` in parser/builder → `?` 传播或返回 `Result`
- `#[cfg(test)]` 内的 panic/unreachable 不在 scope 内（测试中使用 panic 是合理的）
- `outcome.rs` 的 `#[cfg(debug_assertions)]` panic 是有意设计的安全网，需要特殊处理（保持 debug 断言或改为统一的日志+Result 方案）
- `quest/mod.rs` 的 `From<i64>` impl 中 panic：改为 `TryFrom` 是 public API breaking change，需评估影响或添加 fallback variant（如 `Unknown(i64)`）保持 `From` infallible
- 所有新增 error variant 使用 `thiserror`（与 crate convention 一致）

**Test scenarios:**
- Happy path: `cargo test --workspace` 通过
- Error path: 新增 error variant 能正确传播到调用者
- Edge case: 之前 panic 的场景现在返回 `Err`，调用者能处理

**Verification:**
- `cargo test --workspace` 通过
- Library crate 中 `panic!`/`unreachable!` 数量显著减少
- `grep -rn 'panic!\|unreachable!()' --include='*.rs' crates/ | grep -v test | grep -v '#\[cfg(test)\]'` 列出残余项及理由

---

### U5. Normalize `// TODO` Comments

**Goal:** 所有裸 `// TODO` 加 issue 引用格式

**Requirements:** R8

**Dependencies:** None (可与其他单元并行)

**Files:**
- Modify: `crates/emukc_battle/src/targeting.rs` — 8 处 (lines 42, 51, 56, 61, 579, 586, 593, 600)
- Modify: `crates/emukc_battle/src/damage.rs` — 1 处
- Modify: `crates/emukc_gameplay/src/user/account.rs` — 1 处
- Modify: `crates/emukc_gameplay/src/game/quest/consume.rs` — 1 处
- Modify: `src/bin/net/router/kcsapi/api_get_member/questlist.rs` — 1 处
- Modify: 其余处（需 grep 确认完整列表，预估约 19 处总计）
- Preserve: `route_condition.rs` 已使用 `TODO(expiry: 2027-01)` 格式，不改动

**Approach:**
- `// TODO: description` → `// TODO(#0): description` (暂用 `#0` 占位，表示待创建 issue)
- 不创建实际 GitHub issues
- 统一格式：`// TODO(#xxx): description` — `xxx` 为 issue 编号，暂无则为 `#0`

**Test scenarios:**
- Happy path: `grep -rn '// TODO' --include='*.rs' crates/ src/ | grep -v 'TODO(#'` 返回空

**Verification:**
- 所有 `// TODO` 行均包含 `TODO(#` 前缀

---

## System-Wide Impact

- **Interaction graph:** U3 的 `&Vec<T> → &[T]` 变更影响返回类型，调用者通过 auto-deref 兼容，无需改动
- **Error propagation:** U4 新增 error variants，调用者需匹配新 variant 或通过 `#[from]` 自动转换
- **State lifecycle risks:** 无 — 纯重构，不改变业务逻辑
- **API surface parity:** `&Vec<T> → &[T]` 是唯一外部可见签名变更
- **Unchanged invariants:** 所有公共 API 行为不变（`panic` → `Result` 除外，但调用者原本就应处理错误）

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| U4 panic→Result 引起调用链传播变更 | 每处单独分析，利用 `#[from]` 自动转换 |
| U4 `quest/mod.rs` From→TryFrom 是 API breaking change | 添加 fallback variant 保持 `From` infallible |
| U4 `outcome.rs` debug-only panic | 排除出 U4 scope，保持 `#[cfg(debug_assertions)]` 设计 |
| U2 `#[allow]` 实际 154 处非 4 处 | 实施时 grep 过滤，test 文件排除 |
| `&Vec<T> → &[T]` 破坏下游 | Auto-deref 保证兼容，`cargo test` 验证 |
| `#[expect]` 对不存在的 lint 报错 | 编译器 warning 会暴露，删除无效属性 |

---

## Sources & References

- Apollo GraphQL Rust Best Practices Handbook
- Clippy lint reference: https://rust-lang.github.io/rust-clippy/master/
- 审计结果：本会话前序分析
