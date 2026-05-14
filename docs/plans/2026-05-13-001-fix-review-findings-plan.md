---
title: "fix: Code Review Finding — Battle Test Gaps and Style Fixes"
type: fix
status: active
date: 2026-05-13
origin: code review of commits aed649a..HEAD on feat/vibe (3 commits: b23e7f1, 2018bad, 39884b9)
---

# fix: Code Review Finding — Battle Test Gaps and Style Fixes

## Summary

将代码审查中发现的 2 个 manual 级测试缺口、2 个 safe_auto 级格式修复、以及 1 个文档更新转化为实现任务。所有改动限于 battle simulation 和 codex 测试层，不涉及生产逻辑变更。

---

## Requirements

- R1. 闭幕雷击中破拒绝需通过完整 `simulate_day` 流水线验证——前期阶段造成的中破伤害应正确阻止该舰参与闭幕雷击
- R2. 战列舰在 Shelling1 被击沉后 Shelling2 仍应触发（`has_bb_class_at_start` 快照语义），此行为需有明确测试覆盖
- R3. `ship.rs` 中 `cal_srate` 函数的混入 hard tab 应替换为 space
- R4. `execute_shelling2` 攻防顺序反转应有注释说明，防止未来维护者误"修复"
- R5. `docs/battle/rules.md` 需新增 Shelling2 BB-class gate 和闭幕雷击中破拒绝两条规则条目

---

## Scope Boundaries

- 不重构 `test_codex_with_enemy` 的磁盘加载模式（follows existing codebase convention）
- 不为 XBB 单独补测试（codex 可能无 XBB 数据，标记为 advisory）
- 不移动 `fleet_has_bb_class` 到新模块（organizational，不影响正确性）

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_battle/src/simulation/mod.rs`: `simulate_day` 入口、`execute_shelling2`、`execute_closing_torpedo`
- `crates/emukc_battle/src/targeting.rs`: `can_closing_torpedo_ship`（中破检查）、`fleet_has_bb_class`
- `crates/emukc_battle/src/state.rs`: `has_bb_class_at_start` 字段
- `crates/emukc_model/src/codex/ship.rs`: `cal_srate`（line 519 有 stray tab）
- `docs/battle/rules.md`: 已有规则注册表格式

### Institutional Learnings

- Plan `2026-05-12-003` 明确记录：BB-class flag 是 battle-start snapshot，BB 沉没仍触发 Shelling2
- 开幕雷击 deliberately damage-agnostic（回归防护，不可修改 `can_opening_torpedo_ship`）

---

## Implementation Units

### U1. Style: Hard Tab and Missing Comment

**Goal:** 修复 ship.rs 中的 stray hard tab 和 simulation/mod.rs 中 Shelling2 攻防反转缺少的注释。

**Requirements:** R3, R4

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_model/src/codex/ship.rs` (line 519: tab → spaces)
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (execute_shelling2 函数: 添加攻防反转注释)

**Approach:**
- ship.rs:519 将 `\t\t\t` 替换为对应层级的 space 缩进
- simulation/mod.rs: 在 `execute_shelling2` 的 `if enemy_first` 分支前加一行注释说明 KanColle 的交替规则

**Test scenarios:**
- Test expectation: none — 纯格式/注释变更，无行为影响

**Verification:**
- `cargo fmt --check` 通过
- `cargo clippy --workspace` 无新 warning

---

### U2. Integration Test: Chuha Rejection Through simulate_day

**Goal:** 验证闭幕雷击中破拒绝在完整 battle pipeline 中生效。一艘 DD 在 Shelling1 被打到中破，闭幕雷击阶段应排除该舰。

**Requirements:** R1

**Dependencies:** U1 (避免测试与格式修复冲突)

**Files:**
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (tests module)

**Approach:**
- 构造：友军 DD (高 raisou, 低 soukou) + 敌舰 (高 karyoku)
- 运行 `simulate_day`，检查 `simulation.packet.raigeki`
- 敌舰火力需足够高使 DD 在 Shelling1 被打到 HP ≤ 50%
- 如果 DD 被击沉（sinking protection），需调整参数使 DD 存活但中破
- 断言 `raigeki` 为 None 或 DD 的 index 不在雷击参与者中

**Execution note:** 先写 failing test，确认 pipeline 行为后再调整参数

**Test scenarios:**
- Happy path: DD starts healthy, takes chuha damage in shelling1, closing torpedo packet is None or excludes DD
- Edge case: DD exactly at chuha boundary (hp * 2 == maxhp) after shelling1
- Regression guard: same DD at shoha (hp * 2 > maxhp) still participates in closing torpedo

**Verification:**
- `cargo test -p emukc_battle` 全部通过
- 新增测试明确区分 chuha-rejected 和 shoha-accepted 两种情况

---

### U3. Integration Test: Sunk BB Still Triggers Shelling2

**Goal:** 验证 `has_bb_class_at_start` 快照语义——BB 在 Shelling1 被击沉后 Shelling2 仍然执行。

**Requirements:** R2

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (tests module)

**Approach:**
- 构造：友军 BB (低 soukou, 无 raisou) + 敌舰 DD (极高 karyoku，足以在 Shelling1 击沉 BB)
- 运行 `simulate_day`
- 断言 `simulation.packet.hougeki2.is_some()` 即使 BB 在 Shelling1 被击沉
- 需注意 sinking protection：若 is_sortie=true 且 BB 非 taiha 入场，BB 不会被击沉。可能需要用 is_sortie=false 或用 enemy-side BB

**Execution note:** 先确认 sinking protection 的影响范围，可能需要用 enemy BB 作为被击沉方

**Test scenarios:**
- Happy path: Enemy BB takes lethal damage from friendly fleet in shelling1, shelling2 still fires (hourai_flag[2] == 1)
- Alternative: Friendly BB with is_sortie=false takes lethal damage, shelling2 still fires

**Verification:**
- `cargo test -p emukc_battle` 全部通过
- 测试注释明确说明"battle-start snapshot"语义

---

### U4. Update Battle Rules Register

**Goal:** 在 `docs/battle/rules.md` 的 Implemented 表中新增两条规则。

**Requirements:** R5

**Dependencies:** U2, U3 (测试落地后更新 status)

**Files:**
- Modify: `docs/battle/rules.md`

**Approach:**
在 Implemented 表末尾追加：

| Rule ID | Phase | Statement | Confidence | Sources | Status |
|---------|-------|-----------|------------|---------|--------|
| `shelling.bb_class_gate_for_second_round` | `DayShelling` (Shelling2) | Shelling2 仅当任一方在战斗开始时拥有 BB 级舰船 (FBB/BB/BBV/XBB) 时执行 | `B` | `wikiwiki.jp`, `en.kancollewiki.net`, 本地测试 | Implemented |
| `torpedo.closing_rejects_chuha` | `ClosingTorpedo` | 中破 (HP ≤ 50%) 舰船无法参与闭幕雷击阶段 | `B` | `wikiwiki.jp`, `en.kancollewiki.net`, 本地测试 | Implemented |

**Test scenarios:**
- Test expectation: none — 文档变更

**Verification:**
- 文件格式与现有条目一致
- Rule ID 命名遵循已建立的 `phase.description` 模式

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| U2 测试参数难以让 DD 在 Shelling1 精确打到中破 | 使用高 karyoku 敌舰 + 低 soukou DD，多调几轮 seed；或直接在 ship 构造后手动设置 nowhp 模拟中破状态（但这样就不是 pipeline 测试） |
| U3 sinking protection 阻止 BB 被击沉 | 用 enemy-side BB 或 is_sortie=false 绕过保护 |
