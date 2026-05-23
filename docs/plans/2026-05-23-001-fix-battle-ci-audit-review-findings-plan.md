---
title: "fix: Battle CI Audit Review Findings (May 2026)"
date: 2026-05-23
status: active
scope: standard
depth: fix
---

# fix: Battle CI Audit Review Findings

## Problem Frame

Code review（run `20260523-184244-c53bcc5e`）发现 day/night CI 系统实现 2 个 P1 bug + 1 个 P2 bug。本文档覆盖 3 个可修复项。死代码和跨模块重复已标记但延后处理。

## Scope Boundaries

### In Scope

- 修正 `los_fleet_term` floor 求值顺序（P1）
- 修正 day CI 使用 `api_lucky[0]` 而非 `api_lucky[1]`（P1）
- 修正夜战敌方伤害显示值（P2）

### Deferred to Follow-Up Work

- 跨模块装备 helper 重复（`count_main_guns` 等）— 抽取到 `battle_utils.rs`
- `damage.rs` 死代码（`calculate_single_slot_airstrike_damage`）
- `targeting.rs` 死代码（TODO-marked helpers）
- `types/domain.rs` NightBattleParams 死字段
- `shelling.rs` fleet_los Total_LoS 精确计算（wiki 公式更复杂，当前 `api_sakuteki[0]` 求和是近似值）

---

## Key Technical Decisions

| 决策 | 选择 | 理由 |
|------|------|------|
| los_fleet_term floor | 将 floor 包裹整个表达式 `(sqrt + /10).floor()` | wiki 公式: `⌊√(Total_LoS) + Total_LoS/10⌋`，floor 作用于整个和 |
| luck 字段选择 | `api_lucky[1]`（含装备运值） | 夜战 CI 已用 `api_lucky[1]`，日战应一致；wiki `⌊√(Luck)⌋` 未区分 base/total |
| 敌方夜战伤害显示 | 使用 `display_damage()` | 友方已正确使用，敌方应一致 |

---

## Implementation Units

### U1. 修正 `los_fleet_term` floor 求值顺序

**Goal:** `los_fleet_term` 中 floor 仅应用于 `sqrt`，未包裹整个表达式，导致 fleet LoS 贡献偏大。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/day_cutin.rs`

**Approach:**
1. `los_fleet_term` 函数（~line 327-330）：将 `f.sqrt().floor() + f / 10.0` 改为 `(f.sqrt() + f / 10.0).floor()`
2. 调用点 `day_ci_trigger_rate` 中额外有 `.floor()` — 检查是否重复 floor，如果是则删除调用点的冗余 `.floor()`

**Wiki proof:** en.kancollewiki.net 公式 `LoS_Fleet = ⌊√(Total_LoS) + Total_LoS/10⌋` — floor 包裹整个和，非仅 sqrt。

**Test scenarios:**
- fleet_los=100 → 旧: `⌊10⌋ + 10.0 = 20.0`，新: `⌊10 + 10.0⌋ = 20`（此例无差异）
- fleet_los=50 → 旧: `⌊7.07⌋ + 5.0 = 12.0`，新: `⌊7.07 + 5.0⌋ = ⌊12.07⌋ = 12`（此例无差异）
- fleet_los=33 → 旧: `⌊5.74⌋ + 3.3 = 9.05`，新: `⌊5.74 + 3.3⌋ = ⌊9.04⌋ = 9`（有差异！旧=9.05 新=9）
- 添加单元测试验证非整数 sqrt 情况下的 floor 行为

**Verification:** `cargo test -p emukc_battle day_cutin`

---

### U2. 修正 day CI luck 字段

**Goal:** `day_ci_trigger_rate` 使用 `api_lucky[0]`（裸运），应改为 `api_lucky[1]`（含装备运值），与夜战 CI 保持一致。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/day_cutin.rs`

**Approach:**
1. `day_ci_trigger_rate` 函数（~line 299）：将 `ship.ship.api_lucky[0]` 改为 `ship.ship.api_lucky[1]`
2. 注释说明 `[1]` 含装备运值，与夜战一致

**Wiki proof:** en.kancollewiki.net 公式 `⌊√(Luck)⌋` 未区分 base/total，但夜战 CI 同公式使用 total luck（`api_lucky[1]`），日战应一致。wikiwiki.jp 和 kcwiki 确认 luck 包含装备加成。

**Test scenarios:**
- 船运=20 + 装备运=10 → 旧: `⌊√20⌋ = 4`，新: `⌊√30⌋ = 5` → 触发率应提升
- 装备运=0 时行为不变（`api_lucky[0] == api_lucky[1]`）
- 添加测试验证有装备运时 trigger rate 高于无装备运

**Verification:** `cargo test -p emukc_battle day_cutin`

---

### U3. 修正夜战敌方伤害显示值

**Goal:** 夜战敌方 CI 攻击的伤害列表使用 `dealt`（实际伤害），但友方攻击使用 `raw`（overkill 显示）。为保持对称，敌方攻击也应显示 `raw` 伤害。

注意：`display_damage()` 对 `is_friendly=true` 的 defender 返回 `dealt`（敌方攻击循环中 defender 是友方），因此不能直接套用友方模式。

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
1. `night.rs` ~line 718：将 `hit_damages.push(dealt)` 改为 `hit_damages.push(raw_dmg)`
2. 友方攻击使用 `display_damage(&enemy[target_idx], raw_dmg, dealt)` 返回 `raw`（因为 defender `is_friendly=false`），敌方攻击直接 `push(raw_dmg)` 达到相同效果

**Test scenarios:**
- 敌方 CI 命中保护（dealt < raw_dmg）→ 显示值应为 `raw_dmg`（overkill 效果）
- 敌方正常命中（无保护，dealt == raw_dmg）→ 显示值不变

**Verification:** `cargo test -p emukc_battle night`

---

## Risks

| 风险 | 影响 | 缓解 |
|------|------|------|
| los_fleet_term 修正降低 day CI 触发率 | 部分舰船组合触发率微降 | floor 差异仅 0-1 点，影响极小 |
| api_lucky 修正提升 day CI 触发率 | 有装备运的舰船触发率提升 | 符合 wiki 预期行为，是 bug 修正 |
| 夜战显示值改为 raw_dmg 可能改变 API 响应 | 敌方攻击显示 overkill 伤害而非实际伤害 | 与友方攻击行为对称，不影响 HP 计算 |
