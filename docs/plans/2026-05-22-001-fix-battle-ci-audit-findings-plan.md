---
title: "fix: Battle CI Audit Findings (May 2026)"
date: 2026-05-22
status: active
scope: standard
depth: fix
---

# fix: Battle CI Audit Findings

## Problem Frame

2026-05-20 审计发现 battle CI / special attack 系统 7 个问题（1 HIGH + 3 MEDIUM + 2 LOW + 1 INFO）。本文档覆盖其中 5 个可修复项的实施方案（pre-cap/post-cap 文档已完善，不在此计划内）。INFO 级跨模块重复不在此范围。

## Scope Boundaries

### In Scope

- 删除 `check_nagato_mutsu` 重复触发，修正优先级
- 实现装备 LoS 精确求和替代裸索敌近似
- 修正 chuuha 判定边界（排除 taiha）
- 删除 `DayAttackType::CarrierCI` 死分支
- 替换 `special_attack_skip` 线性搜索为 bool 数组

### Deferred to Follow-Up Work

- 跨模块工具函数重复（`count_main_guns` 等）— 抽取到 `battle_utils.rs` 单独做
- 已有 plan `.opencode/plans/2026-05-20-002` 中的 P2 项与本计划重叠，执行后那个 plan 对应项可标记完成

---

## Key Technical Decisions

| 决策 | 选择 | 理由 |
|------|------|------|
| Nagato/Mutsu 优先级修复 | 删除 `check_nagato_mutsu`，保留 `check_nagato_broadside` 内部处理 | `check_nagato_broadside` 已正确判断 Mutsu K2 并返回 102，`check_nagato_mutsu` 造成同一攻击类型重复触发机会 |
| LoS 精确计算 | 新增 helper 遍历 `slot_items` 求和 `mst.api_saku` | 已有 `codex.find::<ApiMstSlotitem>` 模式可复用 |
| Chuuha 边界 | `hp_ratio > 0.25 && hp_ratio <= 0.5` | 大破 (≤25%) 不应享受中破加成 |

---

## Implementation Units

### U1. 删除 `check_nagato_mutsu` 重复触发

**Goal:** 消除 `special_attack.rs` 中 `check_nagato_mutsu` 函数及其在 candidates 中的引用。该函数与 `check_nagato_broadside` 对同一配置（长门 K2 + 陆奥 K2）返回相同的 NagatoMutsuBroadside (102)，导致 candidates vec 中出现重复条目，给予双重触发机会。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/special_attack.rs`

**Approach:**
1. 从 `try_special_attack` 的 `candidates` vec 中删除 `check_nagato_mutsu(codex, attackers, formation_id)` 调用（line ~549）
2. 删除 `check_nagato_mutsu` 函数定义（lines 577-620）
3. `check_nagato_broadside` 已正确处理 Mutsu K2 → 返回 `NagatoMutsuBroadside` (102) with 1.68x multiplier，无需修改

**Patterns to follow:** 5277b11 的 audit cleanup 风格 — 只删不改，不碰周围代码

**Test scenarios:**
- 重写 `nagato_mutsu_detection` 测试：改为调用 `check_nagato_broadside`，验证 type 102 + 1.68x multiplier 行为不变
- 现有 `nagato_broadside_detection` 测试继续通过（验证非 Mutsu BB 返回 101）
- `nagato_mutsu_flagship_produces_two_hits` 继续通过

**Verification:** `cargo test -p emukc_battle special_attack` 全部通过

---

### U2. 实现装备 LoS 精确求和

**Goal:** `ship_los_from_equipment` 从遍历装备 `mst.api_saku` 求和，替代使用 `ship.api_sakuteki[0]`（含裸值 + 改修）。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/day_cutin.rs`（修改 `ship_los_from_equipment`）

**Approach:**
1. 修改 `ship_los_from_equipment` 函数签名，新增 `codex: &Codex` 参数
2. 遍历 `ship.slot_items`，对每个装备通过 `codex.find::<ApiMstSlotitem>(&si.api_slotitem_id)` 获取 MST
3. 求和 `mst.api_saku` 值（仅有效装备，`api_slotitem_id > 0`）
4. 更新 `day_ci_trigger_rate` 中的调用点，传入 `codex`
5. 删除函数中的 "approximate" 注释

**Patterns to follow:** `count_type` helper 的 codex lookup 模式（同文件 line 62-73）

**Test scenarios:**
- 带装备的 BB（2 主炮 + AP 弹 + 水偵）→ trigger rate 应基于装备 LoS 而非裸索敌
- 裸船（无装备）→ LoS equip = 0
- 带高 LoS 装备（大型电探 api_saku=10）vs 低 LoS 装备（小型电探 api_saku=3）→ 高 LoS 船触发率更高

**Verification:** `cargo test -p emukc_battle day_cutin` 全部通过

---

### U3. 修正 chuuha 判定边界排除 taiha

**Goal:** 夜战 CI 触发率 chuuha 加成仅在 HP 25%-50%（中破）时生效，HP ≤25%（大破）不享受加成。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
1. `night_ci_trigger_rate`（~line 457）：将 `hp_ratio <= 0.5` 改为 `hp_ratio > 0.25 && hp_ratio <= 0.5`
2. `dd_ci_trigger_rate`（~line 333）：同样将 `hp_ratio <= 0.5` 改为 `hp_ratio > 0.25 && hp_ratio <= 0.5`
3. 两处 chuuha_mod/chuuha_bonus 逻辑保持一致

**Patterns to follow:** 已有的 `is_flagship_healthy` / `is_companion_healthy` HP 判定风格

**Test scenarios:**
- HP 50%（刚好 chuuha）→ 享受加成
- HP 30%（chuuha）→ 享受加成
- HP 25%（taiha 边界）→ 不享受加成
- HP 10%（taiha）→ 不享受加成
- 同一测试覆盖 DD CI 和标准 CI 两条路径

**Verification:** `cargo test -p emukc_battle night` 全部通过，新增 taiha 排除测试

---

### U4. 删除 `DayAttackType::CarrierCI` 死分支

**Goal:** 移除 `day_ci_damage_multiplier()` 中不可达的 `CarrierCI` match arm。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/day_cutin.rs`

**Approach:**
1. `day_ci_damage_multiplier` 函数（~line 46）：将 `DayAttackType::CarrierCI => 1.25` arm 改为 `unreachable!()` 或 `_ => unreachable!()` 兜底
2. `DayAttackType::CarrierCI` 枚举 variant 保留（`resolve_day_attack` 用作返回类型），仅标记此函数中不可达
3. CarrierCI 路径在 `resolve_day_attack` 中直接使用 `sub.damage_multiplier()`，不经过此函数

**Patterns to follow:** 5277b11 删除 `day_ci_accuracy_multiplier` 死代码的方式

**Test scenarios:**
- 现有 carrier CI 测试不受影响（`carrier_ci_fba_detection` 等）
- `cargo clippy -p emukc_battle` 无新增 warning

**Verification:** `cargo test -p emukc_battle day_cutin` + `cargo clippy -p emukc_battle` 0 warnings

---

### U5. 替换 `special_attack_skip` 线性搜索

**Goal:** 将 `Vec<usize>` + `contains()` 线性搜索替换为固定大小 bool 数组。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/shelling.rs`

**Approach:**
1. 将 `let mut special_attack_skip = Vec::new()` 替换为 `let mut special_attack_skip = [false; 6]`（舰队最大 6 船）
2. 填充时：`for &idx in &result.participant_indices { special_attack_skip[idx] = true; }`
3. 检查时：`if special_attack_skip[idx] { continue; }` — O(1) 替代 O(n)

**Patterns to follow:** 项目内已有的 fixed-size 数组风格（如 `api_onslot: [i64; 5]`）

**Test scenarios:**
- Nelson Touch（3 参与者，index 0/2/4）→ 普通炮击跳过这 3 个位置
- 无特殊攻击时 → 所有位置正常炮击
- `cargo clippy` 无 warning

**Verification:** `cargo test -p emukc_battle shelling` + `cargo clippy -p emukc_battle` 0 warnings

---

## Risks

| 风险 | 影响 | 缓解 |
|------|------|------|
| LoS 精确计算改变 CI 触发率 | 可能影响已有测试的 seed-based 断言 | 测试用 `resolve_day_attack` 循环 50 seeds 模式，不依赖精确 rate 值 |
| Chuuha 边界修改影响大破船夜战行为 | 大破船 CI 触发率下降 | 大破船本身多数无法行动（`can_attack_night_ship` 检查），影响有限 |
