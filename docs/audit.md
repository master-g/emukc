# 代码审计报告：feat/vibe 分支（commits 275f3c8..ca50d40）

> 审计日期：2026-04-14  
> 审计范围：`275f3c8`（fix: battle might sunk ships）至 `ca50d40`（feat: map progress）

## 执行摘要

本次提交序列引入了**地图解锁进度系统**、**多源地图目录流水线**（wikiwiki + kcdata + overlay）、**出击战斗集成**（敌舰队解析与战斗模拟），以及**沉船保护机制**（轟沈ストッパー）。然而， diff 中约 70–80% 的变更是 `.rustfmt.toml` 从 hard tabs 切换到 spaces 导致的全库重格式化，这使得功能性改动被大量淹没，降低了 `git blame` 的可追溯性。

---

## 1. 工程架构与技术选型

### 优点

- **SortieStore 替代进程级全局变量**  
  `crates/emukc_gameplay/src/game/sortie_store.rs` 将原先的 `LazyLock<Mutex<HashMap>>` 静态全局变量重构为实例级存储。`HasContext` 提供默认的全局回退实现，而二进制层的 `State` 可以按实例覆盖。这显著提升了路由测试的隔离性，消除了隐式全局状态。

- **地图目录流水线分层清晰**  
  `crates/emukc_bootstrap/src/map_pipeline/` 中的合并顺序明确：wikiwiki → kcdata → public overlay，并附带来源报告（provenance reporting）。易于理解哪一层数据源最终生效。

- **安全的 schema 迁移策略**  
  `crates/emukc_db/src/entity/profile/map_record.rs:147` 对新增 `unlocked` 列的迁移使用了 `DEFAULT 1`，确保现有账号在升级数据库后不会突然被锁定地图，这是正确的向后兼容做法。

- **战斗上下文正确区分出击与演习**  
  `crates/emukc_gameplay/src/game/battle/core.rs:233` 通过 `is_sortie` 字段控制沉船保护，防止演习中意外触发保护机制。

### 问题

- **格式化与功能改动不应混在同一提交**
  `.rustfmt.toml` 的 `hard_tabs = true → false` 变更产生了大量格式化 diff（ca50d40 改动 500 文件、58515 行），将真正的战斗/地图逻辑淹没其中。这降低了 `git blame` 的有效性，也增加 bisect 和回归排查的难度。项目级 reformat 应当是**独立的零功能变更提交**。

- **`sortie_battle_result` 中内存状态在 DB 事务外更新**
  在 `sortie_battle_result`（`sortie.rs:583`）中，`SortieStore` 的修改（remove/modify active sortie）发生在 `tx.commit().await` **之后**。若服务器在 commit 与 store 更新之间崩溃，数据库状态已持久化，但内存中的出击状态却可能不一致。对于回合制游戏来说风险可控，但这确实是一个耐久性缺口。
  - 注：`sortie_battle_impl`（`sortie.rs:946-949`）的情况相反——store 更新在 `tx.commit()` 之前，不存在此问题。

- **`SortieStore` 使用 `std::sync::Mutex` 且每次操作都 `unwrap()`**  
  如果某线程在持有锁时 panic，该锁会被毒化（poisoned），后续所有针对该 profile 的出击请求都会 panic。在 HTTP 服务场景下，这意味着单个意外错误可导致该玩家永久无法出击。建议换用 `parking_lot::Mutex`（不 poison）或 `tokio::sync::RwLock`。

---

## 2. Idiomatic Rust

### 发现的问题

- **伤害计算使用了浮点数**  
  `crates/emukc_gameplay/src/game/battle/core.rs:208`
  ```rust
  let proportional = (0.5 * h as f64 + 0.3 * rand_part as f64).floor() as i64;
  ```
  若游戏逻辑需要跨平台确定性，应避免 `f64`。上述公式可完全用整数运算替代，例如 `(h / 2) + (rand_part * 3) / 10`，以消除不同目标平台（如 WASM vs x86_64）的舍入差异。

- **以 JSON 序列化结果作为分组键**  
  `crates/emukc_gameplay/src/game/map_route.rs:451`
  ```rust
  fn route_predicate_key(predicate: &RoutePredicate) -> String {
      serde_json::to_string(predicate).unwrap_or_else(|_| format!("{predicate:?}"))
  }
  ```
  该函数用于对匹配的路由规则按优先级去重。依赖 `serde_json` 的稳定排序既昂贵又脆弱。更推荐的做法是手写一个基于判别式的键，或直接在 `BTreeSet` 中利用 `RoutePredicate` 的 `Clone` + 比较语义。

- **测试专用的 `From` 实现隐藏了关键行为**  
  `crates/emukc_gameplay/src/game/battle/core.rs:220`
  ```rust
  #[cfg(test)]
  impl From<BattleShipInput> for BattleRuntimeShip {
      fn from(input: BattleShipInput) -> Self {
          Self::new(input, false, false)
      }
  }
  ```
  测试默认将舰船视为敌方、非出击，从而**禁用沉船保护**。如果开发者忘记在测试中显式调用 `BattleRuntimeShip::new(..., true, true)`，就会测不到最需要验证的保护逻辑，是一个明显的测试陷阱。

### 风格建议

- 代码整体较好地使用了 `let...else`、守卫条件以及 `sea_orm` 查询构造器。
- Clippy 报告了一些警告（`quest/update.rs` 存在完全相同的 match 分支、对实现了 `Copy` 的类型调用 `clone` 等），但均非致命问题。

---

## 3. 逻辑正确性

### 确认的 Bug

#### A. 加权路由选择存在溢出偏置

`crates/emukc_gameplay/src/game/map_route.rs:438`
```rust
pub(crate) fn select_route_target_for_roll(
    weights: &BTreeMap<i64, u64>,
    mut roll: u64,
) -> Option<i64> {
    for (cell_no, weight) in weights {
        if roll < *weight { return Some(*cell_no); }
        roll -= *weight;
    }
    weights.keys().next().copied()  // BUG
}
```

当 `roll` 的值等于总权重时（由于浮点转整数的四舍五入 `((pct * 100.0).round() as i64).max(1)`，这是可能发生的），概率质量会被错误地分配给**第一个**目标，而非最后一个。正确的回退应当是最后一个键（可参考 `sortie.rs:1491` 的 `select_enemy_composition_for_roll`，它使用了 `.last()`）。

#### B. EO 地图（Extra Operation）未实现解锁逻辑

`crates/emukc_model/src/codex/map.rs:252` 的 `build_regular_prerequisites()` 仅注册到 `no in 2..=4`：
```rust
// EO maps (N-5, N-6, ...) are not included (future work)  ← 源码已有注释
for area in 1..=7 {
    for no in 2..=4 {
        prereqs.insert(compose_map_id(area, no), compose_map_id(area, no - 1));
    }
}
```

EO 地图（如 1-5、1-6、2-5、3-5、4-5、5-5、6-5、7-4、7-5）**没有任何先决条件条目**。在 `crates/emukc_gameplay/src/game/map.rs:440`：
```rust
None => {
    let (area, _) = split_map_id(map_id);
    if (1..=7).contains(&area) {
        map_id == 11   // 仅 1-1 初始解锁
    } else {
        true
    }
}
```

这些 EO 地图落入 `None` 分支后判断为 `false`（被锁定），而 `check_and_unlock_dependencies_impl` 只解锁有明确 prerequisite 条目的地图。因此，**当前玩家没有任何合法途径解锁 EO 地图**。

> **注意**：源码第 251 行已有 `// EO maps (N-5, N-6, ...) are not included (future work)` 注释，表明这是**已知的未完成功能**，而非意外引入的 bug。

#### C. 沉船保护伤害公式使用 current_hp 而非 entry_hp

`crates/emukc_gameplay/src/game/battle/core.rs:202`
```rust
let h = self.current_hp;
let rand_part = if h > 1 { random.roll_range(0, h) } else { 0 };
let proportional = (0.5 * h as f64 + 0.3 * rand_part as f64).floor() as i64;
```

保护伤害公式的基数使用了 `self.current_hp`（可能在本节点已被更早阶段削减），而舰 C 原版机制应以**进入节点时的 HP**（`entry_hp`）为基准。这会导致同一场战斗中先受损伤的舰船，其保护池被不公平地缩小。

> **补充**：同一函数中，大破判定（`was_taiha_at_entry`）**正确使用了** `self.entry_hp`（core.rs:196），仅保护伤害公式的基数存在此问题。实际影响有限——同一天内单舰多阶段受击时概率伤害差异通常很小（约 0.5 × ΔHP）。

### 潜在问题

- **`apply_sortie_map_result` 在 variant 切换时返回 0（非首通）**  
  `crates/emukc_gameplay/src/game/sortie_result.rs:398`
  ```rust
  if stage_cleared && let Some(next_variant_key) = stage.clear_to_variant_key.clone() {
      // ...
      return Ok(0);
  }
  ```
  即使 `was_cleared == false`，只要触发了阶段切换（variant transition），`api_first_clear` 就返回 0。如果某张地图需要多次击败 Boss 才能通关，玩家可能永远看不到首次通关标志。需要确认这是否符合预期的事件图分层设计。

- **胜利率判定公式是简化版**  
  `battle/core.rs:2074` 的 `calculate_win_rank` 使用绝对敌损伤率（`>= 0.7`）和半数沉没规则判定 D 级。原版舰 C 使用**相对**损伤率以及旗舰击沉等覆盖规则。若项目追求数据保真度，此处与原版存在偏差。

---

## 4. 可测试性

### 优点

- **地图解锁有集成测试**  
  `tests/gameplay_tests/map/unlock.rs` 覆盖了新建账号仅可见 1-1、以及锁定地图出击失败等场景，且测试通过。

- **战斗系统拥有大量单元测试**  
  `battle/core.rs` 中测试了开幕雷、航空战、炮击战、航母无法炮击等边界行为。

- **地图目录存在单元测试**  
  验证了 prerequisite 链、stage 回退逻辑和默认阶段选择。

### 缺陷

- **`clearing_1_1_unlocks_1_2` 测试了错误的东西**  
  该测试**直接修改数据库**来解锁后续地图，然后断言 `get_map_infos` 能返回它们。它并没有真正跑通游戏流程：从出击 → Boss 战胜利 → `apply_sortie_map_result` → `check_and_unlock_dependencies_impl` 的完整链路。一个真正的端到端测试应该模拟 Boss 战胜并验证级联解锁。

- **缺少对 `select_route_target_for_roll` 溢出 bug 的测试**  
  只需一个单元测试：总权重 100、roll = 99，即可暴露首个目标被偏置的问题。

- **缺少 EO 地图解锁逻辑的测试**  
  若添加对 `prerequisite_for(15)` 等的断言，就能在开发阶段发现 EO 地图缺失 prerequisite 的问题。

- **沉船保护缺少直接单元测试**  
  `BattleRuntimeShip::apply_damage` 是 `pub(crate)` 方法，但没有任何测试验证：
  - 旗舰是否永远受保护；
  - 入场大破阈值（`entry_hp * 4 <= maxhp`）是否正确生效；
  - 非旗舰、非大破舰船是否会被保护。

- **`git blame` 受到格式化提交影响**
  由于全库重格式化与功能代码混在 ca50d40 提交中，追溯该提交中引入的业务逻辑较为困难。

---

## 优先修复建议

1. **修复路由选择溢出偏置** —— 将 `select_route_target_for_roll` 的回退从 `keys().next()` 改为 `keys().last()`。
2. **实现 EO 地图的先决条件** —— 在 `build_regular_prerequisites()` 中为 EO 地图补充正确的解锁链（例如 1-5 依赖 1-4、1-6 依赖 1-5 等）。源码已标记为 future work，需排期实现。
3. **将沉船保护的浮点公式改为纯整数运算** —— 保证跨平台确定性。
4. **补充真正的端到端集成测试** —— 模拟 Boss 战胜并验证 `api_next_map_ids` 包含被解锁的地图。
5. **未来严格区分格式化提交与功能提交** —— 建议通过 pre-commit 或 CI 策略强制执行。

---

# 附录 A：Map 系统审计

> 审计日期: 2026-04-10
> 代码位置: `crates/emukc_gameplay/src/game/map_route.rs`, `sortie.rs`, `crates/emukc_model/src/codex/map/`, `crates/emukc_bootstrap/src/map_pipeline/`

## A.1 架构验证

| 层 | 代码位置 | 状态 |
|----|---------|------|
| Bootstrap 解析 | `crates/emukc_bootstrap/` (wikiwiki parser, map pipeline) | ✅ |
| 数据模型 | `crates/emukc_model/src/codex/map/` (types, merge, map) | ✅ |
| Runtime | `crates/emukc_gameplay/src/game/sortie.rs` + `map_route.rs` | ✅ |
| API | `src/bin/net/router/kcsapi/api_req_map/` | ✅ |

数据流: wikiwiki route/enemy/drop → `wikiwiki_map_catalog.json` → runtime 加载 → `kc_data` + public overlay 补齐 → sortie 消费

## A.2 起点路由 (cell_0)

`select_route_from_cells` 对 `cell_no == 0` 的处理（`map_route.rs:198-236`）:

1. **显式 routing rules 存在** → `evaluate_route_destination` 走规则优先级分组
2. **无规则 + inferred_multi_root_start** → **直接报错**，不偷偷 fallback
3. **无规则 + structural_start_fallback (多出口)** → 随机选取
4. **有 selected_cell_id** → 校验合法性后采用

起点 "飞到 A" 问题已彻底修复，歧义起点有正确拒绝行为。

## A.3 Route Predicate 覆盖

已实现（`map_route.rs:238-418`）: `Always`, `FleetSizeWeightedRandom`, `VisitedNode`, `FleetSize`, `EquipmentCount`, `ShipTypeCount`, `FlagshipShipType/Id`, `ContainsShipType/Id`, `ContainsShipSet`, `OnlyShipSet`, `OnlyShipTypes`, `ShipSetCount`, `ShipSetSpeedCount`, `Speed`, `LoS`, `DrumCanisterCount`, `And/Or/Not` — 全部 ✅

未实现:

| Predicate | 影响 |
|-----------|------|
| `VisitedNodeLabel` → `Unsupported` | **无** — 当前 asset 里所有 `VisitedNodeLabel` 已 rewrite 成 `VisitedNode` |
| `Unknown` → `Unsupported` | 残留 4 条，已通过 fallback 兜底 |

## A.4 Route Evaluation 优先级逻辑

`evaluate_route_destination` (`map_route.rs:40-196`):

1. 收集所有匹配 predicate 的规则
2. `Always` predicate 单独放入 fallback 组
3. 非-Always 匹配规则按 predicate key 分组，每组取最低优先级
4. 全局取最低优先级的组作为 executable candidates
5. 若 executable 为空: `SourceUnknown` 全部 → 取最小 cell_no；有 indeterminate + 唯一 unconditional → 直接取；否则报错
6. executable 非空 → 按权重随机选择

**注意**: 步骤 5 的 `SourceUnknown` fallback 取 `BTreeSet::iter().next()` 即最小 cell_no。当前 `SourceUnknown = 0` 无实际影响。

## A.5 非起点歧义路由

`map_route.rs:233`: 非 cell_0 节点有多个 `next_cells` 且无规则时，直接取 `next_cells[0]`。与起点保护逻辑不同 — 非起点没有歧义拒绝机制，依赖 catalog 编译质量。

## A.6 Enemy Fleet 决定

Pipeline: `resolve_sortie_enemy_fleet` → `select_locked_enemy_composition` → `select_random_enemy_composition` → `fallback_enemy_composition(ship_id=412)`

## A.7 遗留项

| 优先级 | 项目 | 说明 |
|--------|------|------|
| 低 | 消化剩余 4 个 `Unknown` predicate | 继续扩展 parser vocabulary |
| 低 | `node_label` → 更稳定的 merge identity | 当前 merge 主键仍是 `cell_no` |
| 低 | Arrival-context routing (`ArrivedFrom`) | 当前只有 sortie-wide `VisitedNode` |
| 中 | 确认 7-3 多阶段回归测试是否已落地 | 存在 `first_gauge_clear_switches_map_variant_without_finishing_map` 测试 |

---

# 附录 B：Battle 系统审计

> 审计日期: 2026-04-10
> 代码位置: `crates/emukc_gameplay/src/game/battle/core.rs`, `sortie.rs`

## B.1 Phase 实现覆盖

已实现:

| Phase | 函数 | 行号 | 状态 |
|-------|------|------|------|
| 航空战 (kouku) | `simulate_kouku` | 1105 | ✅ 含 stage1/2/3, 制空权, 触接 |
| 开幕对潜 (OASW) | `simulate_opening_taisen` | 1393 | ✅ |
| 开幕雷击 | `simulate_opening_torpedo` | 911 | ✅ |
| 昼战炮击 (×2) | `simulate_shelling_side` | 850 | ✅ 两轮 |
| 闭幕雷击 | `simulate_raigeki` | 973 | ✅ |
| 夜战炮击 | `simulate_night_hougeki` | — | ✅ 含 CI/连击判定 |
| sp_midnight | via `simulate_night_battle_v1` | 794 | ✅ |

已实现 BattleType: `Normal`, `AirBattle`, `LdAirBattle`, `LdShooting` — 全部 ✅

未实现: 联合舰队 (combined)、基地航空队 (LBAS)、支援舰队、夜间航空战

## B.2 伤害公式偏差

### 昼战炮击 (`calculate_shelling_damage`, line 1221)

**实现**: `(火力[0] + 5) × 阵形补正`, Cap 220, 装甲 × 0.7

**缺失**: 改修强化值、空母特殊公式 (`1.5× + 55`)、CL 轻炮补正 (`√单装 + 2√连装`)、意大利 CA 补正、联合舰队补正；装甲 × 0.7 是简化近似

### 雷击 (`calculate_torpedo_damage`, line 1234)

**实现**: `(雷装[0] + 5) × 阵形补正`, Cap 180, 装甲 × 0.55

**缺失**: 改修强化值（鱼雷 ★ 的 1.2 系数）；装甲 × 0.55 是简化

### 夜战 (`calculate_night_damage`, line 1247)

**实现**: `(火力[0] + 雷装[0] + 5) × 交战形态`, Cap 360, 装甲 × 0.7

**🐛 BUG**: 乘了 `engagement.modifier()`，文档明确夜战无阵形/交战形态修正。修复: 移除 `* engagement.modifier()`。

**缺失**: 改修强化值、夜侦常数 (+5/+7/+9)

### 对潜 (`calculate_asw_damage`, line 1365)

**实现**: `(√素对潜 × 2 + √装备对潜 × 1.5 + 类型bonus) × 协同补正`, Cap 170

**状态**: 素对潜/装备对潜分离 ✅、类型 bonus ✅、协同补正 1.4375/1.265/1.15/1.1 ✅

**⚠️ 简化**: 爆雷投射机未与爆雷区分、缺 Hedgehog 等 √(装备对潜-2) 减甲

### Cap 计算 (`apply_cap`, line 842)

`Cap后 = Cap + floor(√(Cap前 - Cap))` — 与文档公式完全一致 ✅

## B.3 夜战 CI/连击

| 类型 | 实现倍率 | 文档倍率 | CI系数 | 状态 |
|------|---------|---------|--------|------|
| DoubleAttack | 1.2 | 1.2 | — | ✅ |
| MainMainMain | 2.0 | 2.0 | 140 | ✅ |
| MainMainSec | 1.75 | 1.75 | 130 | ✅ |
| TorpTorpTorp | 1.3 | 1.3 | 122 | ✅ |
| MainTorpRadar | 1.625 | 1.625 | 115 | ✅ |

未实现: 主AP CI (1.3×)、主雷达 CI (1.2×)、瑞云立体 (1.35×)、海空立体 (1.3×)、战爆联合 CI (FBA/BBA/BA)

## B.4 沉船保护 (轟沈ストッパー)

`BattleRuntimeShip::apply_damage` (line 180-217):

| 规则 | 状态 |
|------|------|
| 非大破入场的友军不会被击沉 (`entry_hp * 4 <= maxhp`) | ✅ |
| 旗舰始终受保护 | ✅ |
| 保护公式 `floor(0.5*H + 0.3*rand(0..H))` | ✅ |
| 仅 sortie + friendly 生效 | ✅ |
| 演习和敌方不触发 | ✅ |
| Post-condition assertion `verify_protected_ships_alive` | ✅ |

**⚠️ 已知问题**: 保护公式基数使用 `current_hp` 而非 `entry_hp`（见主报告 §3.C）

## B.5 胜负判定

`calculate_win_rank` (line 2073): S/A/B/C/D/E 六级判定逻辑与文档一致 ✅。已沉舰不获 EXP ✅。

## B.6 目标选择与分类

- 分类 (`target_class`, line 1791): `Submarine` / `PtBoat` / `Installation` / `SurfaceShip`
- OASW: 仅 Submarine ✅
- 昼战/夜战 ASW 舰: 优先 Submarine, fallback Surface ✅
- 鱼雷: Surface + Installation + PT, 不含 Submarine ✅

**已知简化**: Installation 和 PT 并入 surface-like bucket。

## B.7 OASW 发动条件

`can_opening_asw` (line 1290): DE/DD/CL/CT/CLT/AO/CVL/CVB/BBV 的条件判定 ✅

**❌ 未实现**: Isuzu K2 / Tatsuta K2 等特殊改二舰的无条件 OASW

## B.8 制空权判定

`AirState::from_power` (line 280): 确保/优势/均衡/劣势/丧失五级阈值与文档一致 ✅

## B.9 Torpedo Payload 方向 & api_si_list

- `api_fydam` / `api_eydam` 按 `TorpedoAttackerSide` 正确分流 ✅
- 昼战/ASW/夜战装备 display 选择与文档一致 ✅

## B.10 阵形补正

炮击/雷击阵形补正 (1.0/0.8/0.7/0.85/0.6) ✅，ASW 阵形补正 (0.6/0.8/1.2/1.1/1.3) ✅

## B.11 建议修复顺序

1. **B1** — 夜战交战形态 bug: 移除 `core.rs:1254` 的 `* engagement.modifier()`（一行修复）
2. **F1** — 补齐炮击伤害公式: 改修、空母公式、CL 轻炮补正、装甲修正
3. **F2+F3** — 补齐雷击/夜战改修和夜侦常数
4. **F5** — 特殊舰无条件 OASW
5. **F4** — ASW synergy 爆雷投射机细分
6. **F6** — 更多夜战 CI 类型
