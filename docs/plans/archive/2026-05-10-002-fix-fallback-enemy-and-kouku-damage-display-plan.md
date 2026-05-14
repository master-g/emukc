---
title: "fix: 敌舰后备使用友方舰娘 + 航空战api_fdam显示原始伤害"
type: fix
status: completed
date: 2026-05-10
---

# fix: 敌舰后备使用友方舰娘 + 航空战 api_fdam 显示原始伤害

## 摘要

实际游玩 1-3 时发现两个 bug：(1) F 节点敌人显示为友方舰娘（山城改二），因 `fallback_enemy_composition` 使用舰船 ID 412 而非深海栖舰 ID；(2) 航空战（kouku）阶段 `api_fdam` 报告原始伤害而非实际伤害，导致客户端看到保护的舰船被"击沉"。

---

## 问题背景

### Bug 1：后备敌人 = 友方舰娘

地图 1-3 cell 6（F 节点）在 wikiwiki 数据和运行时目录中均无 enemy_fleet 数据。`resolve_sortie_enemy_fleet` 调用 `fallback_enemy_fleet` → `fallback_enemy_composition`，后者返回 `ship_ids: vec![412]`。

舰船 412 = 山城改二（友方 BB）。`build_sortie_enemy_ship(412)` 调用链：`new_enemy_ship(412)` 失败（412 不在 `enemy_ship_extra`）→ `new_ship(412)` 成功（创建友方 BB）。结果：敌人拥有完整的 BB 装备和属性，客户端显示为友方舰娘。

另外 `build_sortie_enemy_ships` 第 23-27 行也有相同的 `vec![412]` 内联后备。

### Bug 2：航空战 api_fdam 显示原始伤害

`kouku.rs:269-270` 和 `311-312`：
```rust
let (raw_dmg, _dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
output.damage[target_idx] += raw_dmg;
```

`apply_damage` 返回 `(raw, dealt)` — 当友方舰船触发大破保护时，`raw` 是致死伤害，`dealt` 是实际扣除的 HP。航空战阶段累加 `raw_dmg` 到 `api_fdam`，客户端看到伤害大于实际 HP，误判舰船被击沉。

炮击（`shelling.rs:61`）和雷击（`torpedo.rs:49`）都正确使用 `display_damage()` 处理此问题。

---

## 需求

- R1. 后备敌舰使用深海栖舰 ID（如 1501 = 驱逐イ级），不使用友方舰娘 ID
- R2. 航空战阶段 `api_fdam` 和 `api_edam` 使用 `display_damage()` 返回值，与炮击/雷击一致
- R3. 为两个修复分别添加测试覆盖

---

## 范围边界

- 不修改 `apply_damage` 内部逻辑 — 保护机制本身正确
- 不修改炮击/雷击阶段 — 已正确使用 `display_damage()`
- 不修复 1-3 cell 6 缺失 enemy_fleet 数据的数据问题 — 这是 wikiwiki 解析器覆盖范围
- `display_damage()` 函数签名不变 — 仅在航空战阶段调用它

---

## 关键技术决策

- **KD1**: 使用深海驱逐イ级（ID 1501）作为后备 — 最弱深海船，作为 "出错了" 的后备合理。1501 在 `enemy_ship_extra.json` 中存在
- **KD2**: 在航空战的 `execute_airstrike_phase` 中使用 `display_damage()` — 与炮击/雷击保持一致，不引入新函数

---

## 实现单元

### U1. 修复后备敌舰 ID

**目标：** 将所有 `vec![412]` 替换为 `vec![1501]`（深海驱逐イ级）。

**需求：** R1, R3

**依赖：** 无

**文件：**
- 修改：`crates/emukc_gameplay/src/game/sortie/enemy_ship.rs`

**方案：**

1. 定义后备 ID 常量：
```rust
/// Fallback enemy ship: Abyssal DD I-class (驱逐イ级).
/// Used when map data is missing enemy fleet definitions.
const FALLBACK_ENEMY_SHIP_ID: i64 = 1501;
```

2. 将 `fallback_enemy_composition` 第 247 行 `ship_ids: vec![412]` 改为 `ship_ids: vec![FALLBACK_ENEMY_SHIP_ID]`。

3. 将 `build_sortie_enemy_ships` 第 24 行 `vec![412]` 改为 `vec![FALLBACK_ENEMY_SHIP_ID]`。

**测试场景：**
- `fallback_enemy_composition` 返回的 `ship_ids` 包含 1501，不含友方舰娘 ID
- `build_sortie_enemy_ships` 在空 `ship_ids` 时使用 1501
- `build_sortie_enemy_ship(1501, level)` 通过 `new_enemy_ship` 成功创建深海舰船
- 集成：地图无 enemy_fleet 数据时，敌人列表全是深海栖舰，无友方舰娘

**验证：** `cargo test -p emukc_gameplay`

---

### U2. 修复航空战 api_fdam 显示伤害

**目标：** 航空战阶段对友方防守舰使用 `display_damage()` 返回实际伤害，与炮击/雷击一致。

**需求：** R2, R3

**依赖：** 无

**文件：**
- 修改：`crates/emukc_battle/src/simulation/kouku.rs`

**方案：**

在 `execute_airstrike_phase` 中，将两处伤害累加改为使用 `display_damage`：

```rust
// 当前（第 269-270 行）：
let (raw_dmg, _dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
output.damage[target_idx] += raw_dmg;

// 修改为：
let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
output.damage[target_idx] += display;
```

同样修改第 311-312 行（雷击轰炸阶段）。

`display_damage` 行为：
- 敌方防守舰：返回 `raw`（允许过量击杀显示）
- 友方防守舰：返回 `dealt`（保护后的实际伤害）

**遵循模式：**
- `shelling.rs:57-61` — 相同的 `apply_damage` + `display_damage` 模式
- `torpedo.rs:47-49` — 相同模式

**测试场景：**
- 友方舰船大破状态下航空战不致死：`api_fdam < entry_hp`（保护生效时显示伤害 < 原始伤害）
- 敌方舰船 `api_edam` 可以超过 HP（过量击杀显示不变）
- 航空战标记阵位正确：flag 数组长度匹配舰队大小（现有测试 `kouku_flag_arrays_match_fleet_sizes` 不受影响）
- 新测试：构造低 HP 友方舰船 + 高伤害航空战，验证 `api_fdam` 反映实际扣除 HP 而非原始伤害

**验证：** `cargo test -p emukc_battle`

---

## 系统级影响

- **战斗结果：** 航空战显示值变化仅影响客户端动画和 HP 追踪，不影响实际战斗结果（`apply_damage` 正确修改了 HP）
- **客户端兼容：** 修正后的 `api_fdam` 与官方服务器行为一致（友方显示实际伤害）
- **后备敌人：** 使用深海驱逐イ级后，即使 map data 缺失，战斗仍可正常进行（敌人属性合理）

---

## 风险与依赖

| 风险 | 缓解措施 |
|------|----------|
| 深海 ID 1501 未来版本被移除 | 1501 是最基础的深海驱逐舰，自游戏初期存在至今，移除风险极低 |
| 航空战显示值变化导致其他测试失败 | `display_damage` 是现有已测试函数，行为可预测 |
| 后备掩盖 map data 缺失 | 已有 `warn!` 日志记录后备使用，与当前行为一致 |
