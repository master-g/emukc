# Battle 系统审计报告

> 审计日期: 2026-04-10
> 审计范围: battle core (phases, damage, sinking protection, target selection) + enemy build + payload + rank
> 参考文档: `research.md`, `plan.md`, `rules.md`, `data-fidelity-fix.md`

## 审计结论

Battle 系统的**安全性和协议自洽性**已经过关（沉船保护、torpedo 方向、api_si_list、敌舰 fallback 均与 rules.md 一致）。主要差距在**伤害公式保真度** — 当前公式是大幅简化版，与线上真实公式有显著偏差。此外发现一个明确的 bug: 夜战伤害不应受交战形态影响。

---

## 1. Phase 实现覆盖

`core.rs` 中的 phase 实现:

### 已实现

| Phase | 函数 | 行号 | 状态 |
|-------|------|------|------|
| 航空战 (kouku) | `simulate_kouku` | 1105 | ✅ 含 stage1/2/3, 制空权, 触接 |
| 开幕对潜 (OASW) | `simulate_opening_taisen` | 1393 | ✅ |
| 开幕雷击 | `simulate_opening_torpedo` | 911 | ✅ |
| 昼战炮击 (×2) | `simulate_shelling_side` | 850 | ✅ 两轮 (先友后敌) |
| 闭幕雷击 | `simulate_raigeki` | 973 | ✅ |
| 夜战炮击 | `simulate_night_hougeki` | — | ✅ 含 CI/连击判定 |
| sp_midnight | via `simulate_night_battle_v1` | 794 | ✅ |

### 已实现 BattleType

| 类型 | Phase 开关 | 状态 |
|------|-----------|------|
| `Normal` | kouku + OASW + torpedo + shelling ×2 + closing torpedo | ✅ |
| `AirBattle` | kouku + OASW, no shelling/torpedo | ✅ |
| `LdAirBattle` | kouku only | ✅ |
| `LdShooting` | shelling only | ✅ |

### 未实现

| 拓扑 | 文档定位 | 说明 |
|------|---------|------|
| 联合舰队 (combined) | Track 4 | 空母机动/水上打击/输送护卫 |
| 基地航空队 (LBAS) | Track 4 | 喷式强袭 + 基地航空队 |
| 支援舰队 | Track 4 | 近代化支援/长距支援 |
| 夜间航空战 | — | 夜战特殊航空战 |

---

## 2. 伤害公式审计

对比 `research.md` 公式与 `core.rs` 实现。

### 2.1 昼战炮击 (`calculate_shelling_damage`, line 1221)

**实现:**
```
攻击力 = (火力[0] + 5) × 阵形补正
Cap后 = apply_cap(攻击力 × 交战形态, 220)
伤害 = max(floor(Cap后 - 装甲 × 0.7), 1)
```

**文档公式:**
```
基本攻击力 = 火力 + 装备火力 + 装备加成 + 改修强化值 + 5
Cap前攻击力 = 基本攻击力 × Cap前补正 + 轻巡轻炮补正 + 意大利重巡补正
Cap后攻击力 = Cap + √(Cap前 - Cap)
最终伤害 = floor(floor(攻击力 × 阵形 × 交战 × 联合补正) × Cap前乘算 + Cap后加成) × Cap后乘算 - 防御力
```

| 差异项 | 影响 |
|--------|------|
| **缺少改修强化值** | ★ 装备的火力加成被忽略 |
| **缺少空母特殊公式** (`1.5× + 55`) | 空母炮击伤害偏差大 |
| **缺少 CL 轻炮补正** (`√单装 + 2√连装`) | CL/CT/CLT/AO 带轻炮时伤害偏低 |
| **缺少意大利 CA 补正** | Zara/Pola 伤害偏低 |
| **装甲 × 0.7 而非直接减** | 简化近似，总体偏低 |
| **缺联合舰队补正** | 当前不涉及，但扩展时需补 |
| **Cap 值 220** | ✅ 正确 |

### 2.2 雷击 (`calculate_torpedo_damage`, line 1234)

**实现:**
```
攻击力 = (雷装[0] + 5) × 阵形补正
Cap = 180, 装甲 × 0.55
```

| 差异项 | 影响 |
|--------|------|
| **缺改修强化值** (鱼雷 ★ 的 1.2 系数) | 雷击伤害偏低 |
| **装甲 × 0.55** | 简化 |
| **Cap 值 180** | ✅ 正确 |

### 2.3 夜战 (`calculate_night_damage`, line 1247)

**实现:**
```
攻击力 = (火力[0] + 雷装[0] + 5) × 交战形态
Cap = 360, 装甲 × 0.7
```

| 差异项 | 影响 |
|--------|------|
| **⚠️ 乘了交战形态补正** | **BUG: 夜战不应受交战形态影响** |
| **缺改修强化值** | 夜战 ★ 装备加成被忽略 |
| **缺夜侦常数** (+5/+7/+9) | 夜间触接加成缺失 |
| **Cap 值 360** | ✅ 正确 |

### 2.4 对潜 (`calculate_asw_damage`, line 1365)

**实现:**
```
攻击力 = (√素对潜 × 2 + √装备对潜 × 1.5 + 类型bonus) × 协同补正
Cap = 170
```

| 项目 | 状态 | 说明 |
|------|------|------|
| 素对潜 / 装备对潜分离 | ✅ | `(ship_asw - equip_asw).sqrt() * 2` |
| 类型 bonus (航空+8 / 爆雷+13) | ✅ | |
| 协同补正 1.4375/1.265/1.15/1.1 | ✅ | |
| **爆雷投射机未与爆雷区分** | ⚠️ | `has_projector = has_depth_charge` 是简化 |
| Cap 值 170 | ✅ | |
| 缺对潜减甲值 | ⚠️ | Hedgehog 等 √(装备对潜-2) 减甲缺失 |

### 2.5 Cap 计算 (`apply_cap`, line 842)

```
Cap后 = Cap + floor(√(Cap前 - Cap))
```

与文档公式完全一致。✅

---

## 3. 🐛 Bug: 夜战不应受交战形态影响

**位置:** `core.rs:1254`

```rust
let capped_power = apply_cap(attack_power * engagement.modifier(), 360.0) as f64;
```

**修复方案:** 移除 `* engagement.modifier()`，改为:

```rust
let capped_power = apply_cap(attack_power, 360.0) as f64;
```

**依据:** `research.md` §15.14 的完整伤害计算顺序中，夜战公式不含阵形/交战形态乘算。wikiwiki.jp 夜戦页也确认"夜战无阵形/交战形态修正"。

---

## 4. 夜战 CI/连击

`NightAttackType` (line 2120) 与文档对照:

| 类型 | 实现倍率 | 文档倍率 | 实现命中数 | 文档命中数 | CI系数 | 状态 |
|------|---------|---------|-----------|-----------|--------|------|
| DoubleAttack | 1.2 | 1.2 | 2 | 2 | — | ✅ |
| MainMainMain | 2.0 | 2.0 | 1 | 1 | 140 | ✅ |
| MainMainSec | 1.75 | 1.75 | 1 | 1 | 130 | ✅ |
| TorpTorpTorp | 1.3 | 1.3 | 2 | 2 | 122 | ✅ |
| MainTorpRadar | 1.625 | 1.625 | 1 | 1 | 115 | ✅ |

CI 发动率使用 `ci_coefficient` 与文档 `種別係数` 一致。

**未实现的 CI 类型:**
- 主AP CI (1.3×)
- 主雷达 CI (1.2×)
- 瑞云立体 (1.35×)
- 海空立体 (1.3×)
- 战爆联合 CI (FBA/BBA/BA)

---

## 5. 沉船保护 (轟沈ストッパー)

`BattleRuntimeShip::apply_damage` (line 180-217):

| 规则 | 实现 | rules.md ID | 状态 |
|------|------|-------------|------|
| 非大破入场的友军不会被击沉 | `was_taiha_at_entry = entry_hp * 4 <= maxhp` | `survival.sortie_non_taiha_sinking_protection` | ✅ |
| 旗舰始终受保护 | `is_flagship = ship_index == 0` | 同上 | ✅ |
| 保护公式 `floor(0.5*H + 0.3*rand(0..H))` | 精确实现 | — | ✅ |
| 仅对 sortie + friendly 生效 | `is_friendly && is_sortie` | `survival.practice_and_enemy_do_not_use_sinking_protection` | ✅ |
| 演习和敌方不触发 | 默认 `is_friendly=false` / `is_sortie=false` | 同上 | ✅ |
| Post-condition assertion | `verify_protected_ships_alive` (line 2449) | — | ✅ |

---

## 6. 胜负判定

`calculate_win_rank` (line 2073):

| 条件 | 判定 | rules.md | 状态 |
|------|------|----------|------|
| 己方全沉 | E | — | ✅ |
| 敌全沉 + 己方 0 沉 | S | `S` 需无沉舰 | ✅ |
| 敌全沉 + 有沉 | A | 沉舰降级 | ✅ |
| 己方 ≥ 半数沉 | D | `D` | ✅ |
| 敌损伤 ≥ 70% + 无沉 | A | — | ✅ |
| 敌损伤 ≥ 70% + 有沉 | B | — | ✅ |
| 敌损伤率 > 己方 | B | — | ✅ |
| else | C | — | ✅ |

**已沉舰不获 EXP**: `rules.md` `result.sunk_friendly_ship_gets_no_exp` ✅

---

## 7. 目标选择与分类

### 分类 (`target_class`, line 1791)

| 类别 | 判定方式 |
|------|---------|
| `Submarine` | SS / SSV |
| `PtBoat` | 名字含 "PT小鬼群" / "Schnellboot小鬼群" |
| `Installation` | 名字含 "砲台"/"飛行場"/"港湾"/"離島"/"集積地"/"泊地"/"要塞"/"トーチカ" |
| `SurfaceShip` | 其他所有 |

### 选择逻辑 (`select_random_target_index`, line 1984)

| Phase | 攻击能力 | 合法目标 |
|-------|---------|---------|
| OpeningTorpedo / ClosingTorpedo | `SurfaceOnly` | Surface + Installation + PT (**不含** Submarine) |
| DayShelling (非 ASW 舰) | `SurfaceOnly` | Surface + Installation + PT |
| DayShelling (ASW 舰) | `BothPreferSubmarine` | 优先 Submarine, fallback Surface |
| NightShelling (非 ASW 舰) | `SurfaceOnly` | 同上 |
| NightShelling (ASW 舰) | `BothPreferSubmarine` | 同上 |
| OASW | 单独 `select_submarine_target` | **仅** Submarine |

**已知简化:** Installation 和 PT 当前并入 surface-like bucket。rules.md 已标注为 follow-up。

---

## 8. OASW 发动条件

`can_opening_asw` (line 1290):

| 舰种 | 条件 | 文档 | 状态 |
|------|------|------|------|
| DE | ASW≥60 + 声纳 | ✅ | ✅ |
| DD/CL/CT/CLT/AO | ASW≥100 + 声纳 | ✅ | ✅ |
| CVL | ASW≥65 + ASW 航空机 | ✅ | ✅ |
| CVB | ASW≥100 + ASW 航空机 | ✅ | ✅ |
| BBV | ASW≥100 + 大型声纳 + ASW 航空机 | ✅ | ✅ |
| **Isuzu K2 / Tatsuta K2 / etc.** | **无条件 OASW** | 文档有 | **❌ 未实现** |

**影响**: 少数特殊改二舰无法触发 OASW。

---

## 9. 敌舰构建

`build_sortie_enemy_ship` (`sortie.rs:1226`):

| 层级 | 来源 | 条件 | 状态 |
|------|------|------|------|
| 1 | `codex.new_enemy_ship()` | enemy bootstrap data | ✅ |
| 2 | `codex.new_ship()` | ship_extra fallback | ✅ |
| 3 | `build_manifest_only_sortie_enemy_ship()` | manifest-only | ✅ |

Manifest-only fallback 行为:
- `api_onslot = [0; 5]` ✅
- `api_slot = [-1; 5]` ✅
- 静默丢弃 manifest 中不存在的装备 ✅

---

## 10. 制空权判定

`AirState::from_power` (line 280):

| 状态 | 条件 | 文档 | 状态 |
|------|------|------|------|
| 制空权确保 | friendly ≥ 3×enemy | ✅ | ✅ |
| 航空优势 | 2×friendly ≥ 3×enemy | ✅ | ✅ |
| 航空均衡 | 中间值 | ✅ | ✅ |
| 航空劣势 | 3×friendly ≤ 2×enemy | ✅ | ✅ |
| 制空权丧失 | 3×friendly ≤ enemy | ✅ | ✅ |

---

## 11. Torpedo Payload 方向

`BattleRaigeki` (line 477) 和 `BattleOpeningAttack` (line 442):

- `api_fydam` / `api_fydam_list_items` ← 友军造成的伤害 ✅
- `api_eydam` / `api_eydam_list_items` ← 敌军造成的伤害 ✅
- `record_torpedo_hit` 按 `TorpedoAttackerSide` 正确分流 ✅

rules.md `payload.torpedo_damage_fields_are_directional`: ✅

---

## 12. api_si_list 选择

| 攻击类型 | Display 逻辑 | rules.md | 状态 |
|---------|-------------|----------|------|
| 昼战炮击 | `DAY_SURFACE_DISPLAY_TYPES` (主炮/副炮/鱼雷/舰载机) | `display.day_shelling_excludes_non_attack_equipment` | ✅ |
| ASW 攻击 | `ASW_DISPLAY_TYPES` (声纳/爆雷/对潜机) | `display.day_asw_prefers_asw_equipment` | ✅ |
| 夜战 | 按 `NightAttackType` 选择装备组合 | `display.night_attack_matches_attack_type` | ✅ |

---

## 13. 阵形补正

| 阵形 ID | 炮击/雷击 | ASW | 文档炮击 | 文档 ASW | 状态 |
|---------|----------|-----|---------|---------|------|
| 1 (单纵) | 1.0 | 1.0 | 1.0 / 0.6 | ✅/⚠️ |
| 2 (复纵) | 0.8 | 1.0 | 0.8 / 0.8 | ✅/⚠️ |
| 3 (轮形) | 0.7 | 1.2 | 0.7 / 1.2 | ✅/✅ |
| 4 (梯形) | 0.85 | 1.1 | 0.85 / 1.1 | ✅/✅ |
| 5 (单横) | 0.6 | 1.3 | 0.6 / 1.3 | ✅/✅ |

**注意**: ASW 阵形补正已正确区分，但炮击/雷击使用**同一组**阵形补正系数。文档中炮击和雷击的系数相同所以无差异。

---

## 14. 发现汇总

### 🐛 Bug

| # | 位置 | 描述 | 修复方案 |
|---|------|------|---------|
| B1 | `core.rs:1254` | 夜战伤害乘了 `engagement.modifier()`，文档明确说夜战无此修正 | 移除 `* engagement.modifier()` |

### ⚠️ 简化/缺失 (保真度)

| # | 位置 | 描述 | 影响 | 建议优先级 |
|---|------|------|------|-----------|
| F1 | `core.rs:1221-1232` | 炮击伤害缺改修、空母公式、CL轻炮/意大利CA补正，装甲用 ×0.7 近似 | 常规图数值偏差 | 高 |
| F2 | `core.rs:1234-1245` | 雷击伤害缺改修，装甲用 ×0.55 近似 | 雷击伤害偏差 | 中 |
| F3 | `core.rs:1247-1257` | 夜战缺改修、夜侦常数 | 夜战伤害偏低 | 中 |
| F4 | `core.rs:1340-1362` | ASW synergy 没区分投射机和爆雷 | 部分对潜伤害偏差 | 低 |
| F5 | `core.rs:1290` | 特殊舰无条件 OASW 未实现 | 少数舰种无法先制对潜 | 低 |
| F6 | `core.rs:2120` | 夜战 CI 缺主AP/主雷达/瑞云立体/海空立体/战爆联合 | 部分 CI 不会触发 | 低 |

### ✅ 与文档一致

| 项目 | 状态 |
|------|------|
| 沉船保护 (轟沈ストッパー) | ✅ |
| Torpedo payload 方向 | ✅ |
| api_si_list 选择 | ✅ |
| 制空权判定阈值 | ✅ |
| 夜战 CI 倍率/命中数/系数 (已覆盖的类型) | ✅ |
| 胜负判定 S/A/B/C/D/E | ✅ |
| 敌舰三级 fallback | ✅ |
| 目标分类 Surface/Installation/PT/Submarine | ✅ |
| Cap 计算 | ✅ |
| 阵形补正 | ✅ |

---

## 15. 建议修复顺序

1. **B1** — 夜战交战形态 bug (一行修复)
2. **F1** — 补齐炮击伤害公式 (改修、空母公式、装甲修正)
3. **F2+F3** — 补齐雷击/夜战改修和夜侦
4. **F5** — 特殊舰 OASW
5. **F4** — ASW synergy 细分
6. **F6** — 更多 CI 类型
