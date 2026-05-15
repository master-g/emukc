---
title: Fix Audit Findings from Recent Commits
type: fix
status: active
date: 2026-05-15
---

# Fix Audit Findings from Recent Commits

## Summary

修复对最近 5 个 commits（`38c5501`、`8a66419`、`3f8b1db`、`18624a8`、`df75fd4`）caveman-review 审计发现的问题：2 个 🔴 bugs（Unknown 静默降级 → 静默完成 quest / 静默存盘为 Oneshot）、4 个 🟡 risks（`From<i64>` 吞掉错误、`TODO(#0)` 无追踪机制、`#[expect]` 删除未验证、unreachable invariant 未运行时检查）、若干 🔵 nits（codex 加载缓存、helper 提取、ctx binding、test gating）。

核心决策：用户选择 **TryFrom 改造** 处理 Unknown variant — 不再静默降级。`From<i64>` 在 workspace 内**无调用者**（dead code），直接删除；改造点在实际解析边界 `From<List>`（解析 tsunkit JSON 时的真实入口）。

---

## Problem Frame

Caveman-review 在最近 5 个 commits 中识别了 6 类问题：

1. **Unknown silent degradation in quest parser** — `RequirementsCategory::Unknown` 在 `extract_conditions` 返回 `vec![]`，在 `extract_list` 中被 `flat_map` 静默吞掉。tsunkit 数据中出现未知 category 时，整个 quest 被构造为 `And(vec![])` —— 下游条件检查器把空 `And` 视作"无条件"，quest 立即可被领取。**生产数据风险。**
2. **Unknown period silently persists as Oneshot** — `Kc3rdQuestPeriod::Unknown` 在 DB `From<Period>` 反向转换时被合并到 `Oneshot`。Unknown period 的 quest 一旦写入 DB，永远不会重置，即使后续修复 manifest 也无法恢复。
3. **Dead `From<i64>` impls swallow errors** — `Kc3rdQuestPeriod::from(i64)` / `Kc3rdQuestCategory::from(i64)` 在 workspace 内无调用者（grep 验证），但保留了 `_ => Unknown` 静默降级路径。死代码 + 反模式。
4. **`TODO(#0)` 无追踪机制** — 19 处 `// TODO(#0)` 已规范化但 `#0` 是占位符。无 CI 检查或过期机制，会永久驻留。
5. **CLI `#[expect(clippy::large_enum_variant)]` 删除未验证** — `Commands` enum 含 `Battle(BattleArgs)` 等大变体，删除 `#[expect]` 可能让 `cargo clippy --workspace -- -D warnings` 失败。
6. **Test infra**：(a) `Codex::load_without_cache_source(".data/codex")` 每个 test 重复磁盘加载（11 个 test 模块）；(b) `unwrap()` 在 codex 加载失败时给出不可读错误；(c) `SeededRng::new` doc 说"for testing"但未 cfg(test) 门控；(d) `SortieBattleSession::profile_id` cfg_attr 不必要复杂；(e) `slot_deprive::get_slot_mut` / `presets::slot` 的 `unreachable!()` 仅靠注释承载 invariant；(f) `expedition.rs` 重复 `Idle` 错误分支。

---

## Requirements

- R1. `RequirementsCategory::Unknown` 不再产生空 `And(vec![])` quest — quest 构造失败时跳过该 quest 并 warn，而非静默注册可领取条目（修复 🔴 bug #1）
- R2. `Kc3rdQuestPeriod::Unknown` 不再静默存盘为 `Oneshot` — DB `From<Period>` 反向不再合并 Unknown，且不引入新 `Period` DB 变体（修复 🔴 bug #2）
- R3. `Kc3rdQuestPeriod::Unknown` / `Kc3rdQuestCategory::Unknown` enum variant 仍保留（运行时降级出口），但解析路径改为显式 `TryFrom<i64>` 语义；dead `From<i64>` impl 删除（用户决策：TryFrom 改造）
- R4. `TODO(#0)` 加 CI 守卫，防止 `#0` 永久驻留
- R5. `cargo clippy --workspace -- -D warnings` 通过（验证 `large_enum_variant` 不再触发）
- R6. Codex 加载在测试套件内只发生一次（perf）
- R7. Codex 加载失败给出可操作错误信息
- R8. 不引入功能行为变化 — 纯防御性 / 正确性修复
- R9. 全部 `cargo test`、`cargo test -p emukc_gameplay`、`cargo test -p emukc_bootstrap` 通过

---

## Scope Boundaries

### In Scope

- `crates/emukc_bootstrap/src/parser/tsunkit_quest/` Unknown category 处理
- `crates/emukc_db/src/entity/profile/quest/mod.rs` Period 反向转换
- `crates/emukc_model/src/thirdparty/quest/mod.rs` `From<i64>` 删除
- `tests/gameplay_tests.rs` Codex caching + 错误信息
- `crates/emukc_battle/src/random.rs` `SeededRng::new` 门控/doc
- `crates/emukc_gameplay/src/game/battle/sortie/mod.rs` `profile_id` 简化
- `crates/emukc_gameplay/src/game/compose/slot_deprive.rs` `unreachable!` debug_assert
- `crates/emukc_gameplay/src/game/presets/slot.rs` `unreachable!` debug_assert
- `crates/emukc_gameplay/src/game/expedition.rs` Idle helper 提取
- `crates/emukc_model/src/profile/{kdock,ndock}.rs` ctx binding nit
- CI 添加 `TODO(#0)` 守卫

### Deferred to Follow-Up Work

- 将 19 处 `TODO(#0)` 替换为真实 GitHub issue 编号（需先创建 issues）
- 全量 `unwrap`/`expect` → `Result` 迁移（沿用 [`2026-05-15-001` 计划](./2026-05-15-001-refactor-rust-best-practices-violations-plan.md) 的范围决策）

### Outside this product's identity

- 重构 quest condition 评估器以更优雅处理空条件 — 无需用户可见的行为改动
- 引入新的 `Period::Unknown` DB 列变体 — 现有 schema 已稳定

---

## Key Technical Decisions

### D1. Unknown variant 处理：解析侧 fail-fast，运行时侧降级保留

**决策**：

- 解析路径（tsunkit/wikiwiki bootstrap）：未知 category 时**跳过该 quest 并 warn**，不构造空 `And` 条件。这是用户选择的"TryFrom 改造"语义在实际边界上的实现。
- 运行时路径（DB read 出现 stale Unknown）：保留 `Kc3rdQuestPeriod::Unknown` enum variant，作为 codex/manifest 漂移时的安全降级出口。但 DB 反向转换 `From<Kc3rdQuestPeriod> for Period` 不再静默合并到 Oneshot —— 改为返回 `Result` 或在调用点 expect。

**Why**: 用户明确选择 TryFrom（推荐项）。但 workspace 调用图显示 `From<i64>` 无调用者，true TryFrom 改造意味着删除 dead code，而非改造调用链。真实的解析边界是 `From<List>`（types.rs:241）和 `From<Frequency>`（types.rs:411）—— 前者已经走 `Unknown` 路径，需要改成在 `to_kc3rd_quest` 层级失败；后者是穷举的，不需要 Unknown。

**See origin**: 用户审计选择"TryFrom 改造（推荐）"。

### D2. `Kc3rdQuestRequirement::And(vec![])` 永远不被构造

**决策**：当 `RequirementsCategory::Unknown` 出现时，`Requirements::to_requirements` 返回 `Result<Kc3rdQuestRequirement, ParseError>`；调用栈 `to_kc3rd_quest` 改为 `Result<Option<Kc3rdQuest>, _>`，Unknown 时返回 `Ok(None)` 跳过该 quest，并 warn。

**Why**: 静默生成 `And(vec![])` = 任何 quest 条件检查器把"全部满足"判定为真 → quest 立即可领取。这是审计中最严重的 🔴 bug。

### D3. `From<i64>` for `Kc3rdQuest{Category,Period}` 删除

**决策**：删除两个 impl。enum variant `Unknown` 保留（默认 fallback、`Default` impl、stale DB 数据）。

**Why**: `grep -rn "Kc3rdQuestPeriod::from\|Kc3rdQuestCategory::from"` 在 workspace 内无匹配（除 def 行）。dead code 删除比改造 TryFrom 更干净。如未来需要 `i64 → enum`，作者会通过 type-checker 直接发现没有 impl，做出选择。

### D4. CI 守卫使用 `! grep` 而非自定义工具

**决策**：在 `.github/workflows/` 加一行 `! grep -rn 'TODO(#0)' --include='*.rs' crates src` 作为 CI step。失败 → 阻止合入新的 `TODO(#0)`。

**Why**: 19 个现有的 `TODO(#0)` 占位符是过渡状态。守卫确保新代码不会再引入未追踪 TODO。已有的 19 个先 grandfather（在守卫加之前必须先把它们替换为真实 issue 号 —— 这部分 deferred 出本计划）。

⚠️ **Sequencing 风险**：U10 必须在所有 `TODO(#0)` 被替换为真实 issue 号**之后**才能合入，否则 CI 立即红。因此 U10 单独成 unit 并标记 `Dependencies: 创建 19 个 GitHub issues` —— 实际执行时该 unit 可能延后到本计划之外。

### D5. 不改变 `Kc3rdQuestPeriod::Unknown` 的 DB 持久化语义本身

**决策**：DB `Period` enum schema 不动。`From<Kc3rdQuestPeriod> for Period` 改为 `TryFrom`，Unknown → `Err`。所有调用点要么通过 D1/D2 杜绝 Unknown 进入 DB 写路径，要么 `expect("unknown period filtered upstream")`。

**Why**: 引入新 DB 变体需要 migration、schema 协调，超出 fix 范围。把 Unknown 拒绝在 write boundary 是更小的改动。

---

## Implementation Units

### U1. 让解析阶段拒绝 Unknown category quest

**Goal**: tsunkit Unknown category 不再生成可领取的空条件 quest。

**Requirements**: R1, R8

**Dependencies**: 无

**Files**:
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs` (modify)
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs` (modify `to_kc3rd_quest`、`parse`)
- `crates/emukc_bootstrap/src/parser/error.rs` (可能加新 variant `UnknownCategory`)
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs` 测试（新增 unit test）

**Approach**:
1. `Requirements::to_requirements` 签名改为 `Result<Kc3rdQuestRequirement, ParseError>`。
2. `extract_conditions` 内 `RequirementsCategory::Unknown` 分支返回 `Err(ParseError::Generic("unknown requirement category"))`。
3. `TsunkitQuestValue::to_kc3rd_quest` 返回 `Result<Option<Kc3rdQuest>, ParseError>`。Unknown → `Ok(None)`，warn 包含 `wiki_id` / `game_id`。
4. `parse()` 函数 `filter_map` 跳过 `Ok(None)`，propagate `Err`。
5. 删除 `extract_conditions` 中已不可达的 `RequirementsCategory::Unknown => vec![]` 分支。

**Patterns to follow**: 现有 `parse()` 内的 `filter_map` 和 `error.rs` 的 `ParseError` 变体风格。

**Test scenarios**:
- 包含 `category: "unknown_xyz"` 的 tsunkit quest JSON → `parse()` 返回的 map 不包含该 quest，且 `tracing` warn 触发
- Top-level Unknown category 的 quest（已被序列化层拦截，作为 sanity）
- Nested `list[].category = "unknown"` → 整个父 quest 被跳过
- 已有合法 quest 不受影响（regression）
- Test expectation: 至少 3 个新增 unit test + 1 个回归 test

**Verification**:
- `cargo test -p emukc_bootstrap` 全绿
- 在 fixture JSON 中注入 `unknown_xyz` 后 `parse()` 不返回该 quest

---

### U2. DB Period 反向转换不再静默吞 Unknown

**Goal**: `Kc3rdQuestPeriod::Unknown` 不再静默存盘为 `Oneshot`。

**Requirements**: R2, R8

**Dependencies**: U1（U1 之后正常 bootstrap 路径不会再产生 Unknown，但 stale DB 数据仍可能残留 —— U2 守住 write boundary）

**Files**:
- `crates/emukc_db/src/entity/profile/quest/mod.rs` (modify `From<Kc3rdQuestPeriod> for Period`)
- 上游所有把 `Kc3rdQuestPeriod` 写入 DB 的调用点（grep 定位）

**Approach**:
1. `From<Kc3rdQuestPeriod> for Period` → `TryFrom<Kc3rdQuestPeriod> for Period`，`Unknown` → `Err(...)`。
2. 调用点：通过 grep 找出 `.into::<Period>()` / `Period::from(quest_period)` 的位置，要么 expect（U1 之后理论不可能触发），要么 propagate。
3. 错误类型可使用 `crate::Error` 已有变体或新增 `UnknownQuestPeriod`。

**Patterns to follow**: 现有 `entity/profile/quest/mod.rs` 中的 enum 转换风格。

**Test scenarios**:
- `Kc3rdQuestPeriod::Unknown.try_into::<Period>()` → `Err(_)`（覆盖每个分支）
- 全部已知 variant 仍正确 → `Ok(...)`
- Round-trip：`Period → Kc3rdQuestPeriod → Period` 对所有合法 Period 保持
- Test expectation: 1 个穷举单元 test

**Verification**:
- `cargo test -p emukc_db` 全绿
- `cargo test -p emukc_gameplay` 全绿（确保上游调用点都正确处理）

---

### U3. 删除 dead `From<i64>` impls

**Goal**: 移除 workspace 无调用者的 dead code 与对应静默降级路径。

**Requirements**: R3, R8

**Dependencies**: U1（确保 Unknown 解析路径已经迁移）

**Files**:
- `crates/emukc_model/src/thirdparty/quest/mod.rs` (delete `impl From<i64> for Kc3rdQuestCategory`、`impl From<i64> for Kc3rdQuestPeriod`)

**Approach**:
1. 删除两个 impl 块。
2. 保留 `Unknown` enum variant（仍被 `Default` 和 stale DB 路径引用）。
3. 保留 `to_api_type` 中的 `Unknown => Other` 分支。

**Patterns to follow**: 仅删除 dead code，不引入新结构。

**Test scenarios**:
- Test expectation: none — 仅删除 dead code，无行为变更，编译即验证

**Verification**:
- `cargo build --workspace` 通过
- `grep -rn "Kc3rdQuestPeriod::from\|Kc3rdQuestCategory::from" crates src` 无匹配

---

### U4. 验证 CLI clippy 干净

**Goal**: 确认 `8a66419` 删除 `#[expect(clippy::large_enum_variant)]` 后 lint 仍 clean。如果不 clean，box 大变体。

**Requirements**: R5, R8

**Dependencies**: 无

**Files**:
- `src/bin/cli/mod.rs` (条件性修改 — 仅 lint 触发时改)

**Approach**:
1. 运行 `cargo clippy --workspace -- -D warnings`。
2. 若通过 → unit 完成（验证步骤）。
3. 若失败 → 找出最大的 `Commands` variant，`Box<XxxArgs>`。

**Patterns to follow**: 已有的其他 clippy fix（参考 `8a66419` commit）。

**Test scenarios**:
- Test expectation: none — 静态 lint 验证，无运行时行为变化

**Verification**:
- `cargo clippy --workspace -- -D warnings` 退出码 0

---

### U5. Test 套件 Codex 缓存 + 可读错误

**Goal**: 11 个 test 模块共享一次 codex 加载；codex 缺失时给出 actionable 错误。

**Requirements**: R6, R7

**Dependencies**: 无

**Files**:
- `tests/gameplay_tests.rs` (modify `TestContext::new`)

**Approach**:
1. 用 `tokio::sync::OnceCell<Codex>` 或 `std::sync::OnceLock<Codex>` cache codex 全局加载。DB 和 sortie store 仍 per-test。
2. `unwrap()` → `.expect("Codex load failed; run `cargo run -- bootstrap` first to populate .data/codex/")`。

**Patterns to follow**: `tokio::sync::OnceCell` 在异步 init 上比 `std::sync::OnceLock` 更合适（codex load 是同步但封装成 async helper 后简化）。

**Test scenarios**:
- 现有 11 个 test 模块全部正常（regression）
- Test expectation: 现有 gameplay test 套件 — 不新增，仅确保 perf 改善后行为不变

**Verification**:
- `cargo test --test gameplay_tests` 全绿
- `time cargo test --test gameplay_tests` 整体时间下降（人工观察，无硬性 SLO）

---

### U6. `unreachable!()` 加 `debug_assert!` 守卫

**Goal**: invariant 在 debug build 下可被运行时违反检测出来。

**Requirements**: R8

**Dependencies**: 无

**Files**:
- `crates/emukc_gameplay/src/game/compose/slot_deprive.rs` (modify L24)
- `crates/emukc_gameplay/src/game/presets/slot.rs` (modify L246)

**Approach**:
1. `slot_deprive::get_slot_mut`：在 `match slot_idx` 之前 `debug_assert!(matches!(slot_idx, Some(0..=4) | None), "caller must validate slot_idx in 0..=4")`。`unreachable!()` 保留作为 release build fallback。
2. `presets::slot` 同模式：`debug_assert!(i < 5, "i bounded by min(len, 5)")`。

**Patterns to follow**: Rust 标准 invariant 守卫 — `debug_assert!` + 注释。

**Test scenarios**:
- 现有 `presets`/`compose` 测试通过（regression）
- 不新增 — `debug_assert!` 命中只在 caller 误用时触发，不构造误用是测试不该覆盖的场景
- Test expectation: regression only

**Verification**:
- `cargo test -p emukc_gameplay` 全绿

---

### U7. `expedition.rs` Idle 错误分支提取 helper

**Goal**: 消除两处近相同的 `Idle => Err(...)` 重复。

**Requirements**: R8

**Dependencies**: 无

**Files**:
- `crates/emukc_gameplay/src/game/expedition.rs` (modify L277、L292)

**Approach**:
1. 在 `expedition.rs` 文件作用域加 `fn idle_status_err(fleet_id: i64, ctx: &str) -> GameplayError { ... }`。
2. 两处 `Idle` 分支调用 `return Err(idle_status_err(fleet_id, "..."))`。

**Patterns to follow**: 同文件其他错误构造模式。

**Test scenarios**:
- 现有 expedition test 全部通过（regression）
- Test expectation: regression only — pure refactor

**Verification**:
- `cargo test -p emukc_gameplay expedition` 全绿

---

### U8. `SeededRng::new` 文档/门控修正

**Goal**: doc 准确描述实际可见性。

**Requirements**: R8

**Dependencies**: 无

**Files**:
- `crates/emukc_battle/src/random.rs` (modify L43 doc)

**Approach**:
1. 检查 `SeededRng::new` 是否在 prod path 被使用：grep `SeededRng::new`。
2. 如仅 test 使用 → 在 impl block 上加 `#[cfg(test)]` 或 `#[cfg(any(test, feature = "test-utils"))]`，doc 不变。
3. 如 prod 也用 → 把 doc 中的 "for testing" 删掉。

**Patterns to follow**: 仓库其他 test-only helper 的 cfg 风格。

**Test scenarios**:
- `cargo build --workspace` 通过
- `cargo test --workspace` 通过
- Test expectation: regression only

**Verification**:
- 编译通过；语义对应实际可见性

---

### U9. `SortieBattleSession::profile_id` 简化

**Goal**: 删除 `#[cfg_attr(not(test), expect(dead_code))]` 复杂表达式。

**Requirements**: R8

**Dependencies**: 无

**Files**:
- `crates/emukc_gameplay/src/game/battle/sortie/mod.rs` (modify L39)

**Approach**:
1. 检查 `profile_id` 字段实际使用：grep `\.profile_id` in `SortieBattleSession`。
2. 如仅 test 读 → `#[cfg(test)]` 字段 + 构造点也加 `#[cfg(test)]`；或保留 prod 字段 + 删除 `#[expect]` 配合 `_` 前缀。
3. 如 prod 也读 → 完全删除 `#[expect]`。

**Patterns to follow**: `cfg(test)` 字段在仓库其他地方的处理风格。

**Test scenarios**:
- `cargo build --workspace` 通过
- `cargo test -p emukc_gameplay` 全绿
- Test expectation: regression only

**Verification**:
- 编译 + clippy clean

---

### U10. CI guard for `TODO(#0)`

**Goal**: 阻止新代码引入未追踪的 `TODO(#0)`。

**Requirements**: R4

**Dependencies**: 在合入前，所有现有 19 个 `TODO(#0)` 已被替换为真实 GitHub issue 号（**deferred to follow-up**，所以本 unit 实际可能晚于 U1-U9 几周才合入）

**Files**:
- `.github/workflows/<existing-rust.yml>` (add CI step)

**Approach**:
1. 在已有 Rust CI workflow（lint job）追加一步：
   ```yaml
   - name: Block TODO(#0) placeholders
     run: |
       if grep -rn 'TODO(#0)' --include='*.rs' crates src; then
         echo "Found TODO(#0) placeholders. Replace with real issue numbers."
         exit 1
       fi
   ```
2. 不要 `--include` 排除 docs（让计划文档里的引用也算 — 但本计划提到 `TODO(#0)` 是文档型引用，需要在本计划中改写为 `` `TODO(#0)` `` 反引号包裹时 grep 不会匹配跨行；实际验证）。

**Patterns to follow**: 已有 CI workflow 步骤风格。

**Test scenarios**:
- Test expectation: none — CI 配置变更，不进入 cargo test
- 手工验证：本地 `grep -rn 'TODO(#0)' --include='*.rs' crates src` 返回 19 行（先），替换后返回 0 行

**Verification**:
- 在 CI 中故意提交一个 `TODO(#0)` 应红
- 实际生产合入时所有占位符已替换 → CI 绿

---

### U11. Nit: kdock/ndock ctx binding

**Goal**: 减少同一 `Option::as_ref()` 调用 5 次的样板。

**Requirements**: R8

**Dependencies**: 无

**Files**:
- `crates/emukc_model/src/profile/kdock.rs` (modify `From<ConstructionDock> for KcApiKDock`)
- `crates/emukc_model/src/profile/ndock.rs` (modify `From<RepairDock> for KcApiNDock`)

**Approach**:
1. 在 `From` impl 顶部 `let ctx = value.context.as_ref();`。
2. 后续字段 `ctx.map_or_else(...)` 替换原 `value.context.as_ref().map_or_else(...)`。

**Patterns to follow**: Rust idiomatic `let` binding for repeated chain.

**Test scenarios**:
- 现有 `kdock`/`ndock` 测试通过（regression）
- Test expectation: regression only

**Verification**:
- `cargo test -p emukc_model` 全绿

---

## Sequencing & Dependencies

```
U1 ──→ U2 ──→ U3
            
U4   U5   U6   U7   U8   U9   U11   (independent)

U10 ──→ (deferred until 19 TODOs replaced)
```

U1 → U2 → U3 是核心 bug 修复链。U4–U9、U11 互不依赖，可并行。U10 在 deferred 项目完成前不合入。

---

## Risks

| Risk | Mitigation |
|------|------------|
| U1 改动破坏 `to_kc3rd_quest` 调用方签名（很多地方调） | grep 全部调用点；调整 `parse()` 内 `filter_map` 即可，外部 API 不变 |
| U2 把 Unknown 转 `Err` 后某调用点 panic | 先 grep `From<Kc3rdQuestPeriod> for Period` 调用点，逐个 expect 或 propagate；U1 已堵住 write 路径 |
| U4 lint 实际仍触发 → box 变体改 ABI | 仅影响 internal CLI struct，无 wire format / DB schema 变化 |
| U5 OnceCell 引入异步初始化竞态 | 用 `tokio::sync::OnceCell` 而非 `std::sync::Once`；`get_or_init` 即可 |
| U10 CI 守卫 grep 字符串太宽，误报 doc/comment 中的 `TODO(#0)` 字面量 | 守卫只针对 `*.rs`；本计划文档使用反引号包裹的 `` `TODO(#0)` `` 不属 .rs，不会被 CI 扫描 |

---

## Verification Strategy

按 unit 顺序：

1. **U1** — `cargo test -p emukc_bootstrap`，注入 fixture 验证 unknown category quest 被跳过
2. **U2** — `cargo test -p emukc_db && cargo test -p emukc_gameplay`
3. **U3** — `cargo build --workspace`（编译验证 dead code 删除）
4. **U4** — `cargo clippy --workspace -- -D warnings`
5. **U5–U9, U11** — `cargo test --workspace`
6. **U10** — 本地 grep + CI dry-run

最终 sweep：`cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace`。

---

## Out-of-Scope Notes

- 19 个 `TODO(#0)` 替换为真实 issue 号 → 需先建 issues。U10 CI 守卫等这一步完成后再合入。
- 全量 `unwrap`/`expect` → `Result` 迁移（764 处）— 沿用 [`2026-05-15-001` 计划](./2026-05-15-001-refactor-rust-best-practices-violations-plan.md) 决策，不在本计划范围。
- 引入 `Period::Unknown` DB schema 变体 — 需要 migration，超出 fix 边界。
