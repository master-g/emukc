---
title: "fix: Sortie and battle gameplay audit findings"
type: fix
status: completed
date: 2026-05-03
---

# fix: Sortie and battle gameplay audit findings

## Summary

Fix 7 gameplay-layer bugs: stale sortie state leaking across sorties, day shelling torpedo display, unmarried ships exceeding level 99, remodel HP not restored, training cruiser repair time, map 1-3 routing, and overkill damage capping against enemies in sorties.

---

## Problem Frame

用户报告 7 个 bug：地图出击进度回港后残留导致二次出击飞到 boss；昼战 DD 显示使用鱼雷攻击；未结婚舰船超 99 级；改造后 HP 不满；教练船入渠时间错误；1-3 地图移动未按有向图边；对敌无法打出过量伤害。根因分析已在 ce-debug 会话中完成，本计划针对各根因制定修复方案。

---

## Requirements

- R1. `start_sortie` 必须清理前次出击残留的 pending battle/result 状态
- R2. 昼战炮击阶段显示装备 ID 不得包含鱼雷类型
- R3. `married` 标志不得从 `api_lv > 99` 自动推导，必须追踪实际 ring 使用
- R4. 改造后舰船 `api_nowhp` 必须等于新上限 `api_maxhp`
- R5. 教练船（CT, stype=21）入渠时间必须使用专属公式
- R6. 地图移动在无 routing rules 时必须正确遵循 `next_cells` 有向边
- R7. 出击中对敌伤害必须显示原始伤害值（含过量），而非以当前 HP 截断

---

## Scope Boundaries

- 不修改 `emukc_battle` crate 内部架构（已有独立 plan `001` 处理）
- 不实现 ring 结婚机制（仅修复 married 标志推导逻辑）
- 不修改 1-3 bootstrap 地图数据（仅修复路由选择逻辑；地图数据问题需独立排查）
- 不修改 practice 演习的伤害显示（演习中截断是正确的 KanColle 行为）

### Deferred to Follow-Up Work

- Ring/结婚道具使用机制 → 独立 feature
- 1-3 地图 routing rules 数据补全（若缺失）→ 独立 fix
- 出击会话超时自动清理 → 独立 feature
- 联合舰队出击 → 独立 openspec

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_gameplay/src/game/sortie.rs` — 出击生命周期，sortie_goback_port、start_sortie、next_sortie、sortie_battle_impl
- `crates/emukc_gameplay/src/game/sortie_store.rs` — SortieStore 及 SortieRepository trait
- `crates/emukc_battle/src/targeting.rs` — `DAY_SURFACE_DISPLAY_TYPES`、`day_attack_display_ids`
- `crates/emukc_db/src/entity/profile/ship/mod.rs` — `From<KcApiShip> for ActiveModel` (line 261: married 推导)
- `crates/emukc_gameplay/src/game/ship/mod.rs` — `update_ship_impl` (line 679: married 推导)
- `crates/emukc_gameplay/src/game/sortie_result.rs` — `update_sortie_result_stats`、`calculate_sortie_ship_exp`
- `crates/emukc_gameplay/src/game/compose/remodel.rs` — `remodel_impl`、HP 恢复
- `crates/emukc_model/src/codex/repair.rs` — `cal_ship_docking_cost`、船型修正系数
- `crates/emukc_gameplay/src/game/map_route.rs` — `evaluate_route_destination`、`select_route_from_cells`
- `crates/emukc_battle/src/types/runtime.rs` — `apply_damage`、伤害截断逻辑
- `crates/emukc_battle/src/simulation/shelling.rs` — `simulate_shelling_side`、damage 字段填充

### Institutional Learnings

- 无直接相关 learnings

### External References

- KanColle wiki (wikiwiki.jp) — 练习巡洋舰入渠时间公式、昼战显示规则。CT 入渠公式参考: wikiwiki.jp/kancolle/入渠（練習巡洋艦は特殊式 `lv*5+30`，無 sqrt 項）

---

## Key Technical Decisions

- **KD1. Sortie state cleanup**: 在 `start_sortie` 中调用 `clear_pending_sortie_runtime_state` 而非修改 sortie_goback_port。前者是防御性修复（无论前次是否正常回港都清理），后者只覆盖正常退出路径。
- **KD2. married flag**: 完全移除 KcApiShip → ActiveModel 转换中的 `api_lv > 99` 推导，改为 `ActiveValue::NotSet`（新建时默认 false）或保留 DB 现有值。`update_ship_impl` 同理。
- **KD3. Overkill damage**: `apply_damage` 已返回 `(raw_damage, effective)` 元组。修复只需在 caller 侧对敌方目标使用 `raw_damage` 而非 `effective` 填入 damage 数组。不修改 `apply_damage` 本身。
- **KD4. Map routing fallback**: 当前 `next_cells[0]` 硬选第一条边。改为当 `next_cells.len() > 1` 且无 routing rules 时，若 `selected_cell_id` 为 None，则从 `next_cells` 中随机选择出口（而非盲选首边）。若 `selected_cell_id` 为 `Some` 且合法，返回所选。KanColle 原生行为是服务器端随机路由（非用户选择），故采用随机而非抛错。
- **KD5. CT repair**: 在 `cal_ship_docking_cost` 的 match 分支中为 `KcShipType::CT` 添加专属 time_base 计算，与 wikiwiki 一致。

---

## Open Questions

### Resolved During Planning

- Bug 4 (HP 不满) 的根因：`cal_ship_status` 在 `api_lv >= 100` 时触发婚姻 HP bonus 计算（`max_hp.min(min_hp + bonus)`），这可能错误地将改造后 maxhp 限制在低于应有的值。修复：在 remodel 流程中，`cal_ship_status` 之后显式重算 `api_maxhp` 正确值。
- Bug 1 根因：确认为 `start_sortie` 未调用 `clear_pending_sortie_runtime_state`。

### Deferred to Implementation

- Bug 6 的确切修复策略取决于 1-3 是否已有 routing rules 定义。若已定义但未匹配，问题在 `evaluate_route_destination`；若未定义，问题在 `select_route_from_cells`。实现时需检查 bootstrap 数据。

---

## Implementation Units

- U1. **Clear stale sortie runtime state on new sortie**

**Goal:** 防止前次出击的 pending battle/result 残留污染新出击

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/sortie.rs`

**Approach:**
在 `start_sortie()` 中 `insert_active` 之前调用 `clear_pending_sortie_runtime_state(self.sortie_store(), profile_id)`，确保任何残留的 pending result 和 pending battle session 被清除。

**Patterns to follow:**
- 与 `sortie_goback_port` (line 862) 中使用相同的清理函数

**Test scenarios:**
- Happy path: 正常 start_sortie → sortie_battle → sortie_battle_result (non-boss) → sortie_goback_port → start_sortie — 第二次 start 无残留
- Error path: start_sortie → sortie_battle（未提交 result，模拟断线）→ start_sortie — 旧 pending 被清理，新出击正常
- Integration: start_sortie → sortie_battle → sortie_battle_result (boss) → start_sortie — 新出击 enemy 数据正确

**Verification:**
- 二次出击时 locked_enemy_composition 来自新地图而非旧 sortie
- pending result 和 pending battle 在 start_sortie 后均为 None

---

- U2. **Remove torpedo types from day shelling display**

**Goal:** 昼战炮击阶段不再显示鱼雷装备 ID

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/targeting.rs`

**Approach:**
从 `DAY_SURFACE_DISPLAY_TYPES` 常量中移除 `KcSlotItemType3::Torpedo` 和 `KcSlotItemType3::SubmarineTorpedo`。鱼雷装备仅在雷击阶段显示，不应出现在炮击阶段的 `api_si_list` 中。

**Patterns to follow:**
- 与 `NIGHT_TORPEDO_TYPES` 分离（夜战有独立常量）

**Test scenarios:**
- Happy path: DD 装备鱼雷但无主炮 → day_attack_display_ids 返回 [-1]（无匹配装备）
- Happy path: DD 装备主炮 + 鱼雷 → day_attack_display_ids 返回主炮 ID（非鱼雷）
- Happy path: CLT 装备鱼雷 + 甲标的 → day_attack_display_ids 返回鱼雷... wait, no. CLT 如果有鱼雷但没有 guns, 也应该显示 [-1]。修正：CLT with 甲标的 only → [-1]。CLT with 主炮 + 鱼雷 → 主炮 ID。

**Verification:**
- `DAY_SURFACE_DISPLAY_TYPES` 不再包含 `Torpedo` 或 `SubmarineTorpedo`
- 现有 targeting 测试通过

---

- U3. **Fix married flag derivation from level**

**Goal:** `married` 字段不再从 `api_lv > 99` 自动推导

**Requirements:** R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_db/src/entity/profile/ship/mod.rs`
- Modify: `crates/emukc_gameplay/src/game/ship/mod.rs`
- Modify: `crates/emukc_model/src/codex/ship.rs`

**Approach:**
1. `From<KcApiShip> for ActiveModel` (mod.rs:261): `married: ActiveValue::Set(value.api_lv > 99)` → `married: ActiveValue::NotSet` — 新建 ship 时默认为 DB default (false)，更新时保留现有值
2. `update_ship_impl` (ship/mod.rs:679): `m.married = s.api_lv > 99` → 删除此行 — 不覆盖 DB 中的 married 值
3. `recalculate_ship_status_with_model` (ship/mod.rs:774): `am.married = ActiveValue::Set(api_ship.api_lv > 99)` → `am.married = ActiveValue::NotSet` — 不覆盖 DB 中的 married 值
4. `cal_ship_status` (codex/ship.rs:277): 此处的 `api_lv >= 100` 婚姻 HP bonus 条件由 U4 负责修复（改为检查实际 married 状态）

**Patterns to follow:**
- `ActiveValue::NotSet` 在 SeaORM 中表示 "不修改此列"

**Test scenarios:**
- Happy path: 新建 Lv1 ship → married = false
- Happy path: ship Lv99, gain EXP → level stays at 99, married stays false
- Regression: 已结婚 ship (married = true) → save/load 后 married 仍为 true
- Edge case: ship at Lv99 with enough EXP for Lv101 — level capped at 99, married stays false

**Verification:**
- 未结婚舰船无论等级多高，`married` 始终为 `false`
- 现有 gameplay 集成测试通过

---

- U4. **Fix remodel HP not restored to new maximum**

**Goal:** 改造后舰船 HP 正确恢复至新上限

**Requirements:** R4

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/compose/remodel.rs`
- Modify: `crates/emukc_model/src/codex/ship.rs`

**Approach:**
1. 在 `remodel_impl` 中 `cal_ship_status` 调用后，显式将 `new_ship.api_nowhp = new_ship.api_maxhp` — 此逻辑当前已存在于 line 199，确认其位置正确，在所有 kyouka 应用之后
2. 排查 `cal_ship_status` 的婚姻 HP bonus (line 277-286) 是否错误修改了 maxhp：若旧舰等级 >= 100 但未结婚，bonus 仍触发，可能将 maxhp 设为错误值。修复: `KcApiShip` 当前无 `married` 字段，采用方案为 `cal_ship_status` 增加 `married: bool` 参数。所有调用点（`remodel_impl`、`codex.new_ship`、`battle 测试辅助`、`practice 敌舰构造` 等约 12 处）传入 `false`（敌方/新造/未结婚舰）或 DB 的 `ship_model.married` 值。

**Patterns to follow:**
- `remodel_impl` 中 kyouka 转移和 cal_ship_status 的现有顺序

**Test scenarios:**
- Happy path: Lv50 舰改造 → nowhp == maxhp (新舰型的 full HP)
- Happy path: 有 HP 近代化改修的舰改造 → nowhp == maxhp (含改修 bonus)
- Edge case: Lv100 未结婚舰改造 → maxhp 不含婚姻 bonus，nowhp 正确
- Edge case: Lv100+ 已婚舰改造 → maxhp 含婚姻 bonus，nowhp 正确

**Verification:**
- `remodel_impl` 执行后 ship 的 `api_nowhp == api_maxhp`
- 改造前后 HP 值变化符合 KanColle 预期

---

- U5. **Fix training cruiser repair time**

**Goal:** 教练船（练习巡洋舰）入渠时间使用正确公式

**Requirements:** R5

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_model/src/codex/repair.rs`

**Approach:**
在 `cal_ship_docking_cost` 中，将 `KcShipType::CT` 从通用 1.0 组中分离，使用专属 time_base 公式：
- `lv < 12`: `lv * 10.0`（与通用一致）
- `lv >= 12`: `lv * 5.0 + 30.0`（CT 专属，不含 sqrt 项）

CT 的 `ship_type_mod` 保持 1.0（不额外放大）。

**Patterns to follow:**
- 现有 `time_base` 分段逻辑（line 42-46）

**Test scenarios:**
- Happy path: CT Lv1, 10 HP lost → time matches CT formula
- Happy path: CT Lv50, 5 HP lost → time matches CT formula (lv*5+30, no sqrt)
- Edge case: CT Lv11 vs Lv12 → level boundary uses correct formula branch
- Regression: DD/CL/CLT repair time unchanged by CT fix

**Verification:**
- CT 入渠时间显著短于同等级 CL（因 CT HP 低 + 公式简化）
- `cargo test -p emukc_model` 通过

---

- U6. **Fix map movement to follow directed graph edges**

**Goal:** 无 routing rules 时正确沿 next_cells 有向边移动，非盲选首边

**Requirements:** R6

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`

**Approach:**
1. `select_route_from_cells` (line 197-234): 当 `next_cells.len() > 1` 且 `cell_no != 0` 时，当前逻辑 `Ok(current.next_cells[0])` 静默选首边。改为：
   - 若仅有单一出口，直接返回（同现行）
   - 若多出口且 `selected_cell_id` 为 `Some` 且在 `next_cells` 中，返回所选（同现行）
   - 若多出口且 `selected_cell_id` 为 `None`，需要根据是否存在 routing rules 决定：有 rules 的 cell 已在 `evaluate_route_destination` 中处理；无 rules 的多出口 cell 应抛错或随机选择。保留随机选择但确保从 `next_cells` 中随机（而非硬编码 [0]）
2. 在 `evaluate_route_destination` 的 `SourceUnknown` 回退路径（line 92-109）中，fallback 目标应为 `current.next_cells` 成员的随机选择，而非 rules 中定义的 targets。

**Patterns to follow:**
- `select_random_enemy_composition` 的加权随机选择模式

**Test scenarios:**
- Happy path: cell 有 1 个 next → 自动选择唯一出口
- Happy path: cell 有 2 个 next, selected_cell_id = Some(valid) → 返回所选
- Edge case: cell 有 3 个 next, selected_cell_id = None → 随机选择其中之一（非固定首边）
- Error path: selected_cell_id = Some(invalid) → 返回错误

**Verification:**
- 1-3 地图多次出击能看到不同路线（非固定路径）
- `cargo test -p emukc_gameplay` 通过

---

- U7. **Enable overkill damage display against enemies in sorties**

**Goal:** 出击中对敌伤害显示原始值（含过量伤害），不以当前 HP 截断

**Requirements:** R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/shelling.rs`
- Modify: `crates/emukc_battle/src/simulation/torpedo.rs`
- Modify: `crates/emukc_battle/src/simulation/night.rs`
- Modify: `crates/emukc_battle/src/simulation/asw.rs`

**Approach:**
`apply_damage` 已返回 `(raw_damage, effective)`。各 attack simulation 函数以不同方式区分攻击方，需逐模块适配：

1. **shelling.rs** — 使用 `ShellingParams.attacker_is_enemy`（已存在）。当 `!attacker_is_enemy`（友方攻击）时使用 `raw_dmg`，敌方攻击使用 `dealt`。
2. **torpedo.rs** — 开幕/闭幕雷击使用独立的 friendly/enemy 循环，每个 `TorpedoHit` 通过 `TorpedoAttackerSide` 枚举标记攻击方。friendly 循环的 hits 使用 `raw_dmg`，enemy 循环的 hits 使用 `dealt`。
3. **night.rs** — 夜战仅有 friendly→enemy 攻击（单循环，`at_eflag=0`）。全部使用 `raw_dmg`。
4. **asw.rs** — 先制对潜有独立的 friendly（`at_eflag=0`）和 enemy（`at_eflag=1`）循环。friendly 循环使用 `raw_dmg`，enemy 循环使用 `dealt`。

**Sortie vs practice gate:** 以上所有模块仅在 sortie（`is_sortie = true`）时使用 `raw_dmg` 作为 display。practice 中友方对敌攻击仍使用 `dealt`（截断值）。`is_sortie` 标记从 `BattleRuntimeShip` 读取: `defenders[target_idx].is_sortie` 或在循环上下文中从 `BattleContext` 获取。

**Patterns to follow:**
- `shelling.rs` 的 `ShellingParams.attacker_is_enemy` 模式
- `torpedo.rs` 的 `TorpedoAttackerSide` 枚举模式
- `asw.rs` / `night.rs` 的 `at_eflag` 循环分离模式

**Test scenarios:**
- Happy path: 友方 ship 对敌方 target (HP=10) 造成 200 伤害 → damage 数组显示 200
- Happy path: 敌方 ship 对友方 target (HP=10) 造成 200 伤害 → damage 数组显示 capped 值（击沉保护后）
- Edge case: 伤害恰好等于剩余 HP → display == dealt == current_hp

**Verification:**
- 出击战斗 API response 中 `api_damage` 可包含超过目标当前 HP 的值
- `cargo test -p emukc_battle` 通过

---

## System-Wide Impact

- **Interaction graph:** U1 (sortie state) 影响所有出击 API 调用链；U3 (married) 影响所有 ship save 路径
- **Error propagation:** U1 消除了错误状态传播（stale pending 数据）
- **State lifecycle risks:** U3 改变了 `married` 字段的写入语义 — 需确保 ring/marriage 机制未来实现时正确设置。KD1 修复缩小了 stale 状态窗口，但 sortie_goback_port 失败与下次 start_sortie 之间仍有短暂窗口 — 当前无非出击 API 读取 SortieStore 的 pending 数据，风险可控。
- **API surface parity:** U7 (overkill) 改变了战斗 API 的 damage 数值范围 — 客户端应已支持（KanColle 原生行为）
- **Integration coverage:** U1、U3、U4 需通过 gameplay 集成测试验证
- **Unchanged invariants:** 击沉保护逻辑不变；practice 战斗 damage 显示不变；ship EXP 计算不变

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| U3 married 修复后 ring 机制未实现，舰船无法结婚 | 文档化：ring 机制需独立实现；当前 married 默认 false 且不可变 |
| U4 cal_ship_status 修改可能影响现有 ship status 计算 | 仅修改婚姻 HP bonus 的条件判断，影响面极小 |
| U6 随机路由可能改变用户可见行为 | 从 next_cells 随机是 KanColle 原生行为；selected_cell_id 路径不受影响 |
| U7 overkill 显示值变化可能破坏现有 battle 测试 | 逐模块适配测试中的 damage 断言 |

---

## Sources & References

- Debug session findings (ce-debug, 2026-05-03)
- `docs/plans/2026-05-03-001-fix-battle-crate-audit-findings-plan.md` — complementary plan for `emukc_battle` crate internals
- KanColle wiki (wikiwiki.jp) — 练习巡洋舰入渠公式、昼战炮击显示
