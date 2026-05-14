---
title: "fix: 审计发现修正（测试加固 + 死代码清理）"
type: fix
status: completed
date: 2026-05-10
---

# fix: 审计发现修正（测试加固 + 死代码清理）

## Summary

修正 plan 010-002/010-003 代码审计发现的问题：kouku 测试 `if fdam > 0` 守卫可导致测试静默通过、日战测试存在死代码和无效断言、缺少伤害实际发生的验证。

---

## Requirements

- R1. kouku 测试必须确保伤害发生，不能因 `if fdam > 0` 守卫静默通过
- R2. 日战端到端测试清理死代码并添加有效断言
- R3. kouku display_damage 回归防护注释

---

## Scope Boundaries

- 不修改生产代码
- 不重构测试结构或抽取公共 helper
- 不修改 enemy_ship.rs 或 map_route.rs 测试（审计无问题）
- 不修改集成测试（sortie_battle.rs 审计无问题）

---

## Key Technical Decisions

- **KD1**: 显式装备轰炸机而非依赖 RNG 种子 — 测试不应依赖特定种子才产生伤害
- **KD2**: 移除 `hougeki_fdam` 死代码和 `total_displayed` 无效计算，替换为有意义的断言

---

## Implementation Units

### U1. kouku 测试加固：消除 `if fdam > 0` 静默通过

**Goal:** 三个 kouku 测试在无伤害时静默通过。显式装备轰炸机确保空袭伤害发生，移除 `if` 守卫。

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/kouku.rs` (tests module)

**Approach:**

三个受影响测试的 enemy CVL 都未装备轰炸机 — 需要用 `first_slotitem_mst_by_type` 获取轰炸机并装备：

1. `kouku_fdam_uses_display_damage_not_raw_under_protection` (line 608)
   - 当前: enemy CVL 无装备，依赖 RNG 产生伤害
   - 修复: 装备 dive bomber，移除 `if fdam > 0` 守卫，直接断言
   - 同时设置 enemy CVL 高等级（99）以确保足够火力

2. `kouku_fdam_equals_actual_hp_loss_at_full_hp` (line 642)
   - 当前: 同上
   - 修复: 同上 — 装备轰炸机，移除 `if` 守卫

3. `kouku_edam_can_exceed_enemy_hp_overkill` (line 703)
   - 当前: `if edam > 0` 守卫
   - 修复: friendly CVL 已有装备但 api_onslot 未设 — 设置 `api_onslot = [18, 0, 0, 0, 0]`，移除 `if` 守卫

**Patterns to follow:**
- `kouku.rs:486-511` — `kouku_stage1_reports_nonzero_losses_when_planes_present` 测试模式
- `simulation/mod.rs:394-398` — `airbattle_mode_still_runs_kouku` 中 CVL 装备轰炸机的模式：
  ```rust
  let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
  carrier.slot_items = vec![slotitem_with_mst_id(bomber_id)];
  carrier.ship.api_onslot = [18, 0, 0, 0, 0];
  ```

**Test scenarios:**
- 修改后的测试必须断言伤害 > 0（防止回归到静默通过）
- 修改后的测试断言与原测试意图一致（display_damage 行为）

**Verification:**
- `cargo test -p emukc_battle kouku`

---

### U2. 日战端到端测试清理

**Goal:** 修复 `day_battle_display_damage_consistent_across_all_phases` 的死代码和无效断言，以及 `day_battle_all_friendly_survive_under_protection` 缺少伤害验证。

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (tests module, lines 519-636)

**Approach:**

1. `day_battle_display_damage_consistent_across_all_phases`:
   - 移除死代码 `let hougeki_fdam: i64 = 0;`
   - 移除无用 `total_displayed` 计算
   - 保留并加固现有断言：DD 存活 + HP loss < entry HP
   - 添加断言：至少一个阶段的 fdam > 0（确保测试有意义）

2. `day_battle_all_friendly_survive_under_protection`:
   - 添加断言：至少一艘友方舰船损失了 HP（确保敌人确实造成了伤害）
   - 同时验证敌人火力配置（`api_karyoku[0] = 200`）确实产生了攻击

**Patterns to follow:**
- 同文件 `sortie_day_battle_enables_midnight_when_both_sides_survive` — 简洁的断言模式

**Verification:**
- `cargo test -p emukc_battle simulation`

---

### U3. kouku display_damage 回归防护注释

**Goal:** 在 kouku.rs 伤害累加点添加注释，标记 display_damage 的正确使用，防止再次回归。

**Requirements:** R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/kouku.rs` (production code, lines ~269-270 and ~311-312)

**Approach:**

在两处 `display_damage` 调用处添加注释：

```rust
// display_damage returns dealt for friendly defenders (sinking protection),
// raw for enemy defenders. Must NOT accumulate raw_dmg directly — see 2026-05-03-004.
let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
output.damage[target_idx] += display;
```

**Verification:**
- `cargo test -p emukc_battle`
