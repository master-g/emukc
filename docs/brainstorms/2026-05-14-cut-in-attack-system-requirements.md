---
title: "Cut-In 攻撃系统完整实现"
date: 2026-05-14
status: active
scope: deep-feature
---

# Cut-In 攻撃系统完整实现

## Problem Frame

当前 battle 系统的攻击类型严重不完整：
- 昼战砲撃只输出 `api_at_type = 0`（通常）或 `7`（对潜），缺少弾着観測射撃（連撃/CI）
- 夜战 CI 只实现了基础 6 种（Normal/DoubleAttack/MainMainMain/MainMainSec/TorpTorpTorp/MainTorpRadar），缺少驱逐专用 CI（sp_list 7-14）和空母夜戦 CI
- 旗舰特殊攻撃（Nelson Touch 等 100 系列）完全缺失

这导致客户端无法播放 CI 动画，战斗伤害计算不准确（缺少 CI 倍率），玩家体验与官服差距大。

## Requirements

- R1. 昼战砲撃实现弾着観測射撃系统（api_at_type 2-6），包含制空权判定、水上偵察機条件、触发率计算、伤害倍率
- R2. 昼战实现空母 Cut-In（api_at_type 7），包含装备条件检测和伤害倍率
- R3. 夜战扩展驱逐 Cut-In（api_sp_list 7-14），包含主鱼電、鱼見電、鱼水雷鱼、鱼水雷ドラム 4 种及其 2hit 变体
- R4. 夜战实现空母夜戦 Cut-In（api_sp_list 6）���包含夜間作戦航空要員 + 夜間戦闘機条件
- R5. 昼战/夜战实现旗舰特殊攻撃（api_at_type/sp_list 100-106），包含 Nelson Touch、長門/陸奥一斉射、Colorado、Richelieu、Queen Elizabeth
- R6. 所有 CI 类型的触发率公式高保真还原官服机制（含运值、旗舰加成、中破加成、装备改修影响）
- R7. 所有 CI 类型正确输出 api_at_type / api_sp_list / api_si_list / api_df_list / api_cl_list / api_damage，确保客户端能正确播放动画

## Scope Boundaries

- 瑞雲立体攻撃(200)、海空立体攻撃(201)、潜水艦隊攻撃(300-302)、夜間瑞雲夜戦CI(200) 暂不纳入本期
- レーザー攻撃(at_type=1) 为废弃类型，不实现
- 対空 Cut-In（api_air_fire）属于航空战系统，不在本 scope

### Deferred to Follow-Up Work

- 瑞雲/海空立体攻撃：需要瑞雲系装备数据完善后单独规划
- 潜水艦隊攻撃：需要潜水艦编成系统完善后规划
- 夜間瑞雲夜戦CI：依赖瑞雲系夜战机制

## Key Decisions

- **实施顺序**：按实现难度排序——驱逐夜CI（扩展现有系统）→ 弾着観測射撃（新系统但逻辑清晰）→ 空母CI → 旗舰特殊攻撃（最复杂）
- **触发率精度**：高保真还原，参考 wikiwiki/kcwiki 公开的公式数据
- **架构方式**：昼战 CI 在 `shelling.rs` 中增加攻击类型检测层（类似夜战的 `detect_night_attack_type` + `resolve_night_attack` 模式）；旗舰特殊攻撃作为独立模块

## Current Implementation Status

### 已实现 ✅

| 系统 | 位置 | 覆盖 |
|------|------|------|
| 夜战基础 CI | `crates/emukc_battle/src/simulation/night.rs` | Normal, DoubleAttack, MainMainMain, MainMainSec, TorpTorpTorp, MainTorpRadar |
| 夜战 CI 触发率 | 同上 `night_ci_trigger_rate()` | 运值/旗舰/中破加成 |
| 夜战 CI 伤害倍率 | 同上 `damage_multiplier()` | 1.0x ~ 2.0x |
| 夜战 CI 多段攻击 | 同上 `hit_count()` | 1-2 hits |

### 未实现 ❌

| 系统 | 缺失内容 | 影响 |
|------|----------|------|
| 昼战弾着観測 | at_type 2-6 全部缺失 | 昼战只有通常攻击，无 CI 动画和倍率 |
| 昼战空母 CI | at_type 7 缺失 | 空母无法触发戦爆連合 CI |
| 夜战驱逐 CI | sp_list 7-14 缺失 | 驱逐夜战只有基础 CI |
| 夜战空母 CI | sp_list 6 缺失 | 夜戦空母无法 CI |
| 旗舰特殊攻撃 | at_type/sp_list 100-106 全部缺失 | 无 Nelson Touch 等特殊攻击 |

## Phase Plan

### Phase 1: 夜战驱逐 CI 扩展（难度：低）

扩展现有 `NightAttackType` enum 和检测逻辑。

**新增类型：**
- sp_list 7: 主砲/魚雷/電探 (1 hit, 1.3x)
- sp_list 8: 魚雷/見張員/電探 (1 hit, 1.2x)
- sp_list 9: 魚雷/水雷戦隊熟練見張員/魚雷 (1 hit, 1.3x)
- sp_list 10: 魚雷/水雷戦隊熟練見張員/ドラム缶 (1 hit, 1.2x)
- sp_list 11-14: 上述 4 种的 2hit 变体 (2 hits, 倍率略低)

**装备条件检测：**
- 需要新增 `KcSlotItemType3` 匹配：見張員、水雷戦隊熟練見張員、ドラム缶
- 驱逐 CI 仅限 DD 舰种

**触发率：**
- 与基础夜CI共用运值公式，但 coefficient 不同

### Phase 2: 昼战弾着観測射撃（難度：中）

新增昼战攻击类型检测系统。

**前置条件（制空権）：**
- 弾着観測射撃要求制空権確保或航空優勢
- 需要从航空战阶段传递制空状态到砲撃阶段
- 当前 `ShellingParams` 需要扩展 `air_state` 字段

**水上偵察機条件：**
- 攻击舰需装备水上偵察機/水上爆撃機（type2 = 10/11）
- 且该搭载数 > 0（未被击落）

**攻击类型（优先级从高到低）：**
- at_type 6: 主砲/主砲 CI (1.5x, 1 hit)
- at_type 5: 主砲/徹甲弾 CI (1.3x, 1 hit)
- at_type 4: 主砲/電探 CI (1.2x, 1 hit)
- at_type 3: 主砲/副砲 CI (1.1x, 1 hit)
- at_type 2: 連続射撃 (1.2x, 2 hits)

**触发率公式：**
- 基础値 = floor(sqrt(运) + 10)
- 制空加成：確保 +10, 優勢 +0
- 旗舰加成：+15
- 装備改修加成：各装備の改修値 * 係数

### Phase 3: 空母 Cut-In（難度：中）

**昼战空母 CI (at_type 7)：**
- 条件：空母 + 艦爆/艦攻 + 制空権確保/優勢
- 倍率：1.25x (戦爆連合), 1.2x (戦爆攻)
- 需要区分 FBA/BBA/BA 子类型

**夜战空母 CI (sp_list 6)：**
- 条件：夜間作戦航空要員 + 夜間戦闘機/夜攻
- 倍率：根据装备组合不同
- 需要新增夜間装備 type 检测

### Phase 4: 旗舰特殊攻撃（難度：高）

每种特殊攻撃需要：
- 特定旗舰舰船 ID 检测
- 编成条件（2番舰/3番舰要求）
- 多舰协同攻击（多个攻击者对多个目标）
- 独立的伤害计算（各舰分别计算后合算）

**实现列表：**
| sp_list | 名称 | 旗舰条件 | 编成要求 |
|---------|------|----------|----------|
| 100 | Nelson Touch | Nelson | 1/3/5番舰 |
| 101 | 一斉射 | 長門改二 | 2番舰 BB |
| 102 | 長門一斉射 | 長門改二 | 2番舰=陸奥改二 |
| 103 | Colorado | Colorado改 | 2/3番舰 BB |
| 105 | Richelieu | Richelieu改 | 2番舰 |
| 106 | QE | Warspite/Valiant | 姉妹艦 |

**架构：**
- 独立模块 `special_attack.rs`
- 在砲撃/夜戦 phase 开始前检测是否满足条件
- 满足时替换该舰的普通攻击为特殊攻撃

## Dependencies & Assumptions

- 制空権判定已在航空战阶段计算（需确认当前 `BattleState` 是否传递 `api_disp_seiku`）
- 装备 type2/type3 分类在 Codex manifest 中已完整
- 旗舰特殊攻撃依赖舰船 ID 精确匹配，需要 Codex 中有对应舰船数据

## Outstanding Questions

### Resolve Before Planning

- Q1. 当前 `BattleState` 或 `ShellingParams` 是否已传递制空状态？如果没有，需要在 Phase 2 前先补充航空战→砲撃的状态传递
- Q2. 見張員/水雷戦隊熟練見張員 在 `KcSlotItemType3` 中是否已有对应枚举值？

### Deferred to Implementation

- 各 CI 类型的精确 coefficient 值需要从 wikiwiki 查证
- 旗舰特殊攻撃的具体舰船 ID 列表需要从 Codex 数据确认
- 2hit 变体的精确倍率差异需要查证

## Sources & References

- `docs/apilist.txt` lines 2252-2270 (昼战 at_type), lines 2319-2342 (夜战 sp_list)
- `crates/emukc_battle/src/simulation/night.rs` — 现有夜战 CI 实现
- `crates/emukc_battle/src/simulation/shelling.rs` — 当前昼战砲撃（无 CI）
- `crates/emukc_battle/src/damage.rs` — 伤害计算（需扩展 CI 倍率参数）
- wikiwiki 弾着観測射撃页面（触发率公式参考）
