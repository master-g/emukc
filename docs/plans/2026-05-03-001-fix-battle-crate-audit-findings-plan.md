---
title: "fix: Battle crate audit findings — correctness, architecture, and code quality"
type: fix
status: active
date: 2026-05-03
---

# fix: Battle crate audit findings — correctness, architecture, and code quality

## Summary

Fix all findings from the `emukc_battle` crate architecture audit: 3 P0 correctness bugs (night CI damage pipeline, CI priority order, shelling speed order), 8 P1 issues (hourai_flag, CV shelling formula, torpedo payload sizing, etc.), and code quality improvements (BattleState encapsulation, types.rs split, DRY consolidation, magic number extraction).

---

## Problem Frame

架构审计发现 `emukc_battle` crate 存在 3 个严重正确性 bug（夜战 CI 伤害管线错误、CI 优先级顺序错误、炮击无视舰队速度），8 个高优先级问题，以及大量代码质量问题（辅助函数跨 3 模块重复、types.rs 838 行混杂领域类型与序列化类型、BattleState 聚合根字段全部 pub、魔法数字遍布）。这些问题影响战斗模拟的准确性和代码可维护性。

---

## Requirements

- R1. 夜战 CI 倍率必须在 soft-cap 之前施加（pre-cap），而非当前的防御减算之后（post-defense）
- R2. 夜战 CI 检测优先级顺序必须修正：主主主 > 主主副 > 鱼雷CI > 主鱼电
- R3. 炮击阶段顺序由舰队速度决定，非固定友方先攻
- R4. hourai_flag 数组索引必须正确映射到对应阶段
- R5. CV 炮击公式使用飞机总数（onslot），非槽位数
- R6. 雷击载荷数组大小动态生成，非硬编码 7
- R7. 炮击1 阶段必须有存活敌人前置检查
- R8. verify_protected_ships_alive 在 release 构建中仍执行
- R9. 辅助函数统一到单一位置，消除跨模块重复
- R10. 伤害公式中的魔法数字提取为命名常量
- R11. BattleState 字段私有化，通过方法暴露和变更
- R12. types.rs 拆分为领域类型、包类型、运行时类型
- R13. win_rank 字段类型从 String 改为枚举
- R14. 夜战 CI 触发率使用含装备运（api_lucky[1]）
- R15. 夜战 CI 补充中破触发率修正
- R16. 夜战敌方 damage_dealt 正确累计
- R17. NightBattleParams 的 formation/engagement 字段标注为明确不使用
- R18. ASW 类型奖励在仅有声纳时给予 0 而非 +13

---

## Scope Boundaries

- 不修改 `emukc_gameplay` 调用方（除非签名变更必须传导，如 win_rank 类型变更）
- 不修改已有 openspec 变更 `fix-battle-attack-system` 和 `harden-battle-refactor-followup` 涵盖的内容
- 不实现联合舰队完整支持（仅做 payload 动态化为前提）
- 不实现航空战 Stage 2 AA 精确化（已知简化，需独立 openspec）
- 不修改 `KcApiShip` 与领域模型的耦合（跨 crate 重构，独立规划）

### Deferred to Follow-Up Work

- 航空战 Stage 2 AA 逐舰模型 → 独立 openspec
- 联合舰队（12 船）完整战斗支持 → 独立 openspec
- KcApiShip 领域模型解耦 → 跨 crate 重构
- 夜战配置驱动阶段流（当前直接调用，未使用 BattleFlow）
- 接触机补充水侦/喷气侦察机支持

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_battle/src/types.rs` — 全部类型定义，838 行，需拆分
- `crates/emukc_battle/src/state.rs` — BattleState 聚合根，字段全部 pub
- `crates/emukc_battle/src/damage.rs` — 伤害公式，788 行，含魔法数字和重复辅助函数
- `crates/emukc_battle/src/targeting.rs` — 目标选择，912 行，含重复辅助函数
- `crates/emukc_battle/src/simulation/night.rs` — 夜战阶段，CI 检测和伤害计算
- `crates/emukc_battle/src/simulation/mod.rs` — 编排层，阶段执行和 BattleState 操作
- `crates/emukc_battle/src/simulation/kouku.rs` — 航空战，含重复辅助函数
- `crates/emukc_battle/src/simulation/torpedo.rs` — 雷击，硬编码 blank(7)
- `crates/emukc_battle/src/simulation/shelling.rs` — 炮击
- `crates/emukc_battle/src/outcome.rs` — 战果计算
- `crates/emukc_battle/src/config.rs` — 阶段流配置

### Existing OpenSpec Changes

- `openspec/changes/fix-battle-attack-system/` — 炮击显示类型、闭幕雷击白名单、开幕雷击、敌方 overkill
- `openspec/changes/harden-battle-refactor-followup/` — PracticeRepository、RNG 注入、CryptoRng 重命名

### External References

- KanColle wiki (wikiwiki.jp) — 夜战 CI 优先级、伤害公式、炮击顺序
- 审计报告（本次会话） — 21 个发现，P0-P3 评级

---

## Key Technical Decisions

- **KD1. Night CI multiplier placement**: 倍率在 `calculate_night_damage` 返回的基础伤害（post-cap, post-defense）之上施加改为在 damage 计算函数内部、cap 之前施加。需要修改 `calculate_night_damage` 接受可选 CI 参数，或在 simulation 层传递 pre-cap power 并施加倍率后再 cap。
- **KD2. Fleet speed calculation**: 从 `BattleRuntimeShip` 的 ship master data 读取 `api_soku`（速力），计算舰队合计速度判定先攻方。需要 `BattleRuntimeShip` 或 `BattleContext` 提供速度信息。
- **KD3. types.rs split strategy**: 拆为 `types/` 目录模块：`types/mod.rs`（re-export）、`types/domain.rs`（枚举、值对象）、`types/packet.rs`（API 包类型，保持 Serialize）、`types/runtime.rs`（BattleRuntimeShip 等运行时类型）。子阶段包类型（BattleKouku 等）保持 Serialize + api_ 前缀因为它们直接序列化为 JSON。
- **KD4. BattleState encapsulation**: 字段私有化，提供 setter 方法（`set_kouku()`, `set_hougeki1()` 等）和 `set_stage_flag(index, value)`、`set_hourai_flag(index, value)`。方法内添加阶段一致性校验。
- **KD5. win_rank type change**: `BattleOutcome.win_rank` 改为 `KcSortieResultRank` 枚举。需要在 `emukc_gameplay` 的 response builder 中调用 `.api_id()` 转为字符串。

---

## Open Questions

### Resolved During Planning

- Night battle formation/engagement: 确认为舰娘机制下夜战不使用（非 bug），需添加注释
- Packet 类型拆分：子阶段类型（BattleKouku 等）保持 Serialize 因为直接透传到 API；顶层包类型（BattlePacket 等）保持不 Serialize 因为通过 response builder 映射

### Deferred to Implementation

- 舰队速度阈值：KanColle 的速度分级（低速/高速/超高速）及先攻判定需查证 wiki
- CI 倍率修改后对现有测试的影响范围需在实现时评估

---

## Implementation Units

- U1. **Night CI damage pipeline and priority fix**

**Goal:** 修正夜战 CI 倍率施加位置（post-defense → pre-cap）、CI 优先级顺序（鱼雷CI > 主鱼电）、CI 触发率（基础运 → 含装备运）、补充中破触发率修正

**Requirements:** R1, R2, R14, R15

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`
- Modify: `crates/emukc_battle/src/damage.rs`
- Test: `crates/emukc_battle/src/simulation/night.rs` (inline tests)

**Approach:**
1. `detect_night_attack_type()`: 交换 `MainTorpRadar` 和 `TorpTorpTorp` 的检测顺序（鱼雷CI 移到主鱼电之前）
2. `calculate_night_damage()`: 新增参数 `ci_multiplier: Option<f64>`，在 `apply_cap()` 之前施加倍率：`apply_cap(basic_power * ci_multiplier, 360.0)`
3. `resolve_night_attack()` / `simulate_night_hougeki()`: 传递 CI 倍率到 damage 函数，移除 night.rs:347 的 post-defense 倍率乘法
4. `night_ci_trigger_rate()`: 将 `ship.ship.api_lucky[0]` 改为 `ship.ship.api_lucky[1]`（含装备运）
5. `night_ci_trigger_rate()`: 补充中破修正（HP <= 50% 时 +18 驱逐CI / +5 其他）

**Test scenarios:**
- Happy path: CI multiplier applied pre-cap with known values → verify damage matches expected formula
- Happy path: Ship with 2 torpedoes + radar → detect TorpTorpTorp (not MainTorpRadar)
- Happy path: Ship with night recon + Supremacy → night_recon_bonus = 9
- Edge case: Ship with high pre-cap power (500+) + CI multiplier → verify soft cap at 360 applied AFTER multiplier
- Edge case: CI trigger rate with base luck 50 vs total luck 80 → verify total luck gives higher rate
- Edge case: Chuuha ship (HP 25%) → verify CI trigger rate bonus applied

**Verification:**
- All existing night battle tests pass
- New test: pre-cap CI damage with known input → expected output
- New test: CI priority with mixed equipment → correct CI type detected

---

- U2. **Shelling fleet speed order**

**Goal:** 炮击阶段顺序由舰队速度决定，非固定友方先攻

**Requirements:** R3

**Dependencies:** U1 (避免在 night.rs 改动上冲突)

**Files:**
- Modify: `crates/emukc_battle/src/simulation/mod.rs`
- Modify: `crates/emukc_battle/src/types.rs` (add speed helper)
- Test: `crates/emukc_battle/src/simulation/mod.rs` (inline tests)

**Approach:**
1. 添加 fleet speed helper 函数：从 `BattleRuntimeShip` 读取 `api_soku`，计算舰队合速度
2. 在 `simulate_day()` 中，根据双方速度比较结果决定 shelling1 和 shelling2 的攻击方
3. `ShellingParams.attacker_is_enemy` 根据速度判定结果设置
4. 速度相同（含）时保持友方先攻（KanColle 默认行为）

**Test scenarios:**
- Happy path: Friendly faster → friendly shells first
- Happy path: Enemy faster → enemy shells first
- Edge case: Equal speed → friendly shells first (default)
- Edge case: All ships sunk before shelling → no shelling phase

**Verification:**
- Existing battle tests pass
- New test: enemy fleet with faster ships → enemy shelling1, friendly shelling2

---

- U3. **Simulation orchestration fixes (hourai_flag + shelling1 alive guard)**

**Goal:** 修正 hourai_flag 索引映射和炮击1存活检查

**Requirements:** R4, R7

**Dependencies:** U2 (同文件改动)

**Files:**
- Modify: `crates/emukc_battle/src/simulation/mod.rs`
- Test: `crates/emukc_battle/src/simulation/mod.rs` (inline tests)

**Approach:**
1. `execute_shelling1()`: `hourai_flag[0]` → `hourai_flag[1]`（KanColle API 中 hourai_flag[0] = 开幕雷击, [1] = 炮击1, [2] = 炮击2, [3] = 闭幕雷击）
2. `execute_shelling2()`: `hourai_flag[1]` → `hourai_flag[2]`
3. `execute_closing_torpedo()`: 确认使用 `hourai_flag[3]`
4. `execute_shelling1()`: 添加存活检查（与 execute_shelling2 一致）

**Test scenarios:**
- Happy path: Normal battle → hourai_flag = [1(opening), 1(shell1), 0(shell2 skip), 1(closing)]
- Happy path: Air battle → verify correct flag indices
- Edge case: All enemies die after opening torpedo → shelling1 skipped, hourai_flag[1] = 0

**Verification:**
- hourai_flag indices match KanColle API specification
- Shelling1 skipped when no alive enemies

---

- U4. **Damage formula fixes (CV shelling + ASW type bonus)**

**Goal:** 修正 CV 炮击公式使用飞机总数；修正 ASW 类型奖励

**Requirements:** R5, R18

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/damage.rs`
- Test: `crates/emukc_battle/src/damage.rs` (inline tests)

**Approach:**
1. `calculate_shelling_damage()`: CV 分支的 `bomber_count` 改为统计所有 bomber slot 的 `api_onslot` 总和（非 slot 数量）
2. `calculate_asw_damage()`: `type_bonus` 逻辑改为：有 ASW 航空机 → +8，有爆雷投射机 → +13，两者都有 → +8+13，都无 → +0

**Test scenarios:**
- Happy path: CV with 3 bomber slots (18+18+18 planes) → power uses 81 not 3
- Happy path: Ship with sonar only → ASW type_bonus = 0
- Happy path: Ship with depth charge + sonar → ASW type_bonus = 13
- Happy path: Ship with ASW aircraft → ASW type_bonus = 8

**Verification:**
- CV shelling damage significantly increased vs before
- Sonar-only ASW no longer gets free +13 bonus

---

- U5. **Torpedo payload dynamic sizing**

**Goal:** 雷击载荷数组大小动态生成

**Requirements:** R6

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/torpedo.rs`
- Test: `crates/emukc_battle/src/simulation/torpedo.rs` (inline tests)

**Approach:**
1. `simulate_opening_torpedo()`: `blank(7)` → `blank(friendly.len().max(enemy.len()))`
2. `simulate_raigeki()`: 同上
3. 考虑提取为常量 `MAX_FLEET_SIZE` 或动态计算

**Test scenarios:**
- Happy path: 6-ship fleet → payload arrays length 6
- Edge case: 1-ship fleet → no panic
- Edge case: 7-ship fleet (escort) → payload arrays length 7

**Verification:**
- No hardcoded 7 in torpedo module
- Fleet of any size 1-7 works without panic

---

- U6. **verify_protected_ships_alive release enforcement + night enemy damage_dealt**

**Goal:** 击沉保护验证在 release 中执行；夜战敌方 damage_dealt 正确累计

**Requirements:** R8, R16

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/outcome.rs`
- Modify: `crates/emukc_battle/src/simulation/night.rs`
- Test: `crates/emukc_battle/src/outcome.rs` (inline tests)

**Approach:**
1. `verify_protected_ships_alive()`: 将 `debug_assert!` 改为条件编译检查 — release 模式下使用 `log::error!` + 返回错误（或 panic with clear message），debug 模式保持 assert
2. `simulate_night_hougeki()` enemy loop: 添加 `ship.damage_dealt += total_dealt;`（与 friendly loop 对称）
3. `NightBattleParams`: 对 `formation_id`/`engagement` 字段添加文档注释说明夜战不使用这些参数

**Test scenarios:**
- Happy path: All protected ships alive → no error
- Error path: Protected ship has HP 0 → error logged in release / assert in debug
- Happy path: Night battle enemy attack → enemy damage_dealt > 0

**Verification:**
- Release build logs violation instead of silently ignoring
- Night battle enemy ships accumulate damage_dealt

---

- U7. **DRY consolidation and magic number extraction**

**Goal:** 消除跨模块辅助函数重复；提取魔法数字为命名常量

**Requirements:** R9, R10

**Dependencies:** U4 (先修 damage.rs 的 bug，再做重构)

**Files:**
- Create: `crates/emukc_battle/src/helpers.rs` (shared helpers)
- Modify: `crates/emukc_battle/src/damage.rs`
- Modify: `crates/emukc_battle/src/targeting.rs`
- Modify: `crates/emukc_battle/src/simulation/kouku.rs`
- Modify: `crates/emukc_battle/src/lib.rs` (add module)

**Approach:**
1. 创建 `helpers.rs`，包含共享辅助函数：`ship_mst()`, `ship_type()`, `has_slotitem_type()`, `has_active_asw_aircraft()`, `is_airstrike_attack_type()`, `is_air_combat_type()`
2. `damage.rs` 和 `kouku.rs` 删除私有副本，改为 `use crate::helpers::*`
3. `targeting.rs` 保留函数定义并标记 `pub(crate)`，`helpers.rs` 直接 re-export 或调用
4. `damage.rs` 顶部定义命名常量：
   - `SHELLING_CAP: f64 = 220.0`
   - `TORPEDO_CAP: f64 = 180.0`
   - `NIGHT_CAP: f64 = 360.0`
   - `ASW_CAP: f64 = 170.0`
   - `AIRSTRIKE_CAP: f64 = 170.0`
   - `DEFENSE_COEFF_A: f64 = 0.7`
   - `DEFENSE_COEFF_B: f64 = 0.6`
   - `AIRSTRIKE_POWER_BONUS: f64 = 25.0`
   - `KOUKU_AA_DIVISOR: f64 = 400.0`
5. 删除 `damage.rs` 中被 `#[allow(dead_code)]` 标注的 `calculate_single_slot_airstrike_damage` 和 `is_airstrike_attack_type`（已有 kouku.rs 版本）
6. 删除 `targeting.rs` 中被 `#[allow(dead_code)]` 标注的夜战显示常量和函数（NIGHT_MAIN_GUN_TYPES 等）

**Test expectation:** none — 纯重构，行为不变。现有测试应全部通过。

**Verification:**
- `cargo test -p emukc_battle` 全部通过
- `grep -r "is_airstrike_attack_type" crates/emukc_battle/src/` 仅出现在 helpers.rs 或 targeting.rs（单一定义）
- 无 `#[allow(dead_code)]` 在辅助函数上

---

- U8. **BattleState encapsulation**

**Goal:** BattleState 字段私有化，通过方法暴露和变更，聚合根强制不变量

**Requirements:** R11

**Dependencies:** U3 (同文件 state.rs 消费者 simulation/mod.rs)

**Files:**
- Modify: `crates/emukc_battle/src/state.rs`
- Modify: `crates/emukc_battle/src/simulation/mod.rs`
- Modify: `crates/emukc_battle/src/simulation/asw.rs`
- Modify: `crates/emukc_battle/src/simulation/kouku.rs`
- Modify: `crates/emukc_battle/src/simulation/shelling.rs`
- Modify: `crates/emukc_battle/src/simulation/torpedo.rs`
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
1. `BattleState` 所有字段改为私有
2. 提供 setter 方法：
   - `set_kouku(kouku: BattleKouku)`
   - `set_opening_attack(attack: BattleOpeningAttack)`
   - `set_opening_taisen(taisen: BattleHougeki, flag: bool)`
   - `set_hougeki1(hougeki: BattleHougeki)`
   - `set_hougeki2(hougeki: BattleHougeki)`
   - `set_raigeki(raigeki: BattleRaigeki)`
   - `set_stage_flag(index: usize, value: i64)`
   - `set_hourai_flag(index: usize, value: i64)`
3. 提供 getter 方法（如 finalize 需要）或保持 `pub(crate)` readonly access
4. `simulate_night()` 使用构造函数替代手动结构体字面量：`BattleState::for_night(friendly, enemy, ...)`
5. simulation 模块改为调用 setter 方法

**Test expectation:** none — 纯重构，行为不变。

**Verification:**
- `cargo test -p emukc_battle` 全部通过
- `state.rs` 中无 `pub` 字段（仅 `pub(crate)` getter）

---

- U9. **types.rs split into module directory**

**Goal:** types.rs 拆分为领域类型、包类型、运行时类型

**Requirements:** R12

**Dependencies:** U8 (先做 BattleState 封装，types.rs 改动最小化)

**Files:**
- Create: `crates/emukc_battle/src/types/mod.rs` (re-exports)
- Create: `crates/emukc_battle/src/types/domain.rs` (enums, value objects)
- Create: `crates/emukc_battle/src/types/packet.rs` (Serialize structs)
- Create: `crates/emukc_battle/src/types/runtime.rs` (BattleRuntimeShip, BattleShipInput, BattleContext, etc.)
- Delete: `crates/emukc_battle/src/types.rs`
- Modify: `crates/emukc_battle/src/lib.rs` (update module declaration)

**Approach:**
1. `types/domain.rs`: `BattleType`, `EngagementType`, `AirState`, `BattlePhase`, `TargetClass`, `AttackCapability`, `TorpedoAttackerSide`, `ShellingParams`, `NightBattleParams`, `TorpedoHit`
2. `types/packet.rs`: `BattleKouku`, `BattleKoukuStage1/2/3`, `BattleOpeningAttack`, `BattleHougeki`, `BattleNightHougeki`, `BattleRaigeki`, `AirstrikeOutput` — 保持 `#[derive(Serialize)]` 和 `api_` 前缀
3. `types/runtime.rs`: `BattleShipInput`, `BattleRuntimeShip`, `BattleContext`, `NightBattleInput`, `BattlePacket`, `NightBattlePacket`, `BattleOutcome`, `BattleSimulation`, `NightBattleSimulation`
4. `types/mod.rs`: re-export 所有 public types，保持 lib.rs 的 re-export 列表不变
5. 删除 `types.rs`，更新 `lib.rs` 的 `mod types` 声明

**Test expectation:** none — 纯重构，public API 不变。

**Verification:**
- `cargo test -p emukc_battle` 全部通过
- `cargo build -p emukc_gameplay` 通过（依赖方编译正常）
- 原 `types.rs` 已删除

---

- U10. **win_rank enum + minor type improvements**

**Goal:** win_rank 从 String 改为枚举；清理遗留标记

**Requirements:** R13

**Dependencies:** U9 (types 拆分后再改)

**Files:**
- Modify: `crates/emukc_battle/src/types/runtime.rs` (BattleOutcome)
- Modify: `crates/emukc_battle/src/outcome.rs` (calculate_win_rank)
- Modify: `crates/emukc_gameplay/src/game/battle/sortie/response.rs` (adapter)
- Modify: `crates/emukc_gameplay/src/game/battle/practice/response.rs` (adapter)
- Test: `crates/emukc_battle/src/outcome.rs` (inline tests)

**Approach:**
1. `BattleOutcome.win_rank`: `String` → `KcSortieResultRank`（已存在于 emukc_model）
2. `calculate_win_rank()`: 返回 `KcSortieResultRank` 而非 `String`
3. `emukc_gameplay` response builder: 调用 `.to_string()` 或 `.api_id()` 转为字符串
4. 删除 `EngagementType` 上的 `#[allow(dead_code)]`
5. 删除 `BattleOutcome.can_midnight` 的 `#[allow(dead_code)]` 或确认使用

**Test scenarios:**
- Happy path: calculate_win_rank returns correct enum variant for S/A/B/C/D/E conditions
- Edge case: 0 sunk enemies, 0 sunk friendly → S rank
- Edge case: half friendly sunk → D rank

**Verification:**
- `cargo test -p emukc_battle -p emukc_gameplay` 全部通过
- `win_rank` field type is `KcSortieResultRank` not `String`

---

- U11. **NightBattleParams cleanup and documentation**

**Goal:** NightBattleParams 的 formation/engagement 字段标注为明确不使用；统一 double-attack 检测逻辑

**Requirements:** R17

**Dependencies:** U9 (types 拆分后)

**Files:**
- Modify: `crates/emukc_battle/src/types/domain.rs` (NightBattleParams)
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
1. `NightBattleParams`: 对 `friendly_formation_id`/`enemy_formation_id`/`engagement` 添加文档注释：`// Night battle does not use formation/engagement modifiers per KanColle mechanics.`
2. 移除 `let _ = (params.friendly_formation_id, params.enemy_formation_id, params.engagement);`，改为在结构体字段上添加 `#[allow(dead_code)]`
3. 提取 double-attack 检测逻辑为独立函数 `is_double_attack_eligible()`，被 `detect_night_attack_type()` 和 `resolve_night_attack()` 共用

**Test expectation:** none — 纯重构和文档。

**Verification:**
- `let _ = (...)` 行已删除
- double-attack 检测逻辑单一定义

---

## System-Wide Impact

- **Interaction graph:** U10 的 win_rank 类型变更影响 `emukc_gameplay` 的 response builder
- **Error propagation:** U6 的 release 模式验证可能新增 log 输出
- **API surface parity:** hourai_flag 修正（U3）影响客户端接收的阶段标志
- **Integration coverage:** 所有变更需通过 `cargo test --test gameplay_tests` 验证端到端无回归
- **Unchanged invariants:** 击沉保护（gocchin stopper）逻辑不变；practice battle 伤害 capping 不变

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| CI 伤害管线修改可能影响所有夜战结果 | U1 添加有精确数值的回归测试 |
| types.rs 拆分可能破坏外部依赖 | U9 保持 public API 完全不变（re-export） |
| BattleState 封装需要改动所有 simulation 模块 | U8 逐模块修改，每步运行测试 |
| 舰队速度判定规则需精确查证 | U2 实现前确认 wikiwiki 的速度分级规则 |
| win_rank 类型变更需修改 emukc_gameplay | U10 最小化改动范围 |

---

## Execution Order

```
Phase 1 (P0 correctness):  U1 → U2 → U3
Phase 2 (P1 correctness):  U4 → U5 → U6
Phase 3 (architecture):    U7 → U8 → U9 → U10 → U11
```

U1→U2→U3 有同文件依赖（simulation/mod.rs, night.rs），顺序执行。
U4→U5→U6 可并行（不同文件）。
U7→U11 有依赖链（DRY → 封装 → 拆分 → 枚举 → 清理），顺序执行。
