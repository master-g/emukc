---
title: "Combined Fleet Sortie Battles - Plan"
type: feat
date: 2026-09-21
artifact_contract: ce-unified-plan/v1
artifact_readiness: drafted
status: draft
product_contract_source: ce-plan-bootstrap
execution: code
---

# Combined Fleet Sortie Battles - Plan

## Goal Capsule

- **Objective:** 让联合舰队能出击并打完一场完整战斗——从 `api_req_map/start` 到
  `api_req_combined_battle/battleresult` 落库，覆盖味方連合 vs 敌通常艦隊的
  空母機動部隊与水上打撃部隊两种编成。
- **Authority order:** 本计划的 U-ID；`docs/battle/combined-fleet-reference.md`
  （wikiwiki 规则与数值表）；`main-decoder/out/main.decoded.js` 的协议字段；
  现有 `emukc_battle` 单舰队实现与 `CLAUDE.md` 分层规则。
- **Execution profile:** 核心数据模型先行（U1），阶段编排与协议输出并行（U2/U3），
  gameplay 舰队拆分随后（U4），端点最后（U5/U6），U7 收质量门。
- **Stop conditions:** 若实现需要改动单舰队路径的 RNG 消耗顺序（会动摇
  `tests/gameplay_tests/battle_golden.rs` 这份冻结的全出击流水），停止并回到规划。
  若 U1 的索引空间核查结论与本计划的假设冲突，停止并重新规划 U2/U3。
- **Tail ownership:** U7 拥有全套质量门与 `apilist.md` 一致性。

---

## Product Contract

### Summary

`api_req_hensei/combined` 已能设置 `profile.combined_type`，编成层是通的；但
`crates/emukc_gameplay/src/game/sortie/setup.rs:68` 在出击时显式拒绝
`combined_type > 0`，报 `combined sortie battle is not implemented yet`。所以
玩家能组联合舰队，却打不了一仗。活动图普遍强制联合舰队，这是「能玩完整游戏」
最大的单个缺口。

根因不在 handler 层——handler 只有 20 行。根因在 `emukc_battle`（18,347 行）
整体按「一方一支舰队」建模：`BattleContext` 只有
`friend_ships: Vec<BattleShipInput>` 一个字段，`targeting.rs:148` 明确写着
`combined-fleet interception is out of scope until combined sortie`，
`shelling.rs:14` 的 `MAX_FLEET_SIZE` 注释写着
`combined fleets need a wider skip array`。

### Problem Frame

| 层 | 现状 | 缺口 |
|---|---|---|
| 编成 | `api_req_hensei/combined` 已实现，`profile.combined_type` 持久化 | 无 |
| 出击 | `setup.rs:68` 硬拒绝 `combined_type > 0` | 双舰队解析与校验 |
| 战斗核心 | `BattleContext` 单舰队；各阶段参与者集合固定为一支舰队 | 第二舰队建模 |
| 战斗核心 | 陣形倍率表只有通常六阵形 | 警戒航行序列四阵形（11–14） |
| 战斗核心 | 基本攻击力公式无「連合艦隊補正」加项 | 补正表接入 |
| 协议 | `packet.rs` 无 `_combined` 系列字段 | 17 个字段 |
| 端点 | `api_req_combined_battle/*` 14 个全缺 | 本计划覆盖 5 个 |
| 结算 | MVP / 经验 / 损伤回写单舰队 | 跨两舰队 |

规则与数值已在 `docs/battle/combined-fleet-reference.md` 落库（来源
wikiwiki.jp，2026-09-21 取得），此前 `research.md` §8.2 把这张表推迟为
「需参考 wikiwiki.jp 的完整数据」，§15.5 只有范围值，不足以实现。

### Requirements Trace

| R-ID | 需求 | 来源 |
|---|---|---|
| R1 | `combined_type ∈ {1,2}` 时出击不再被拒绝，deck 1 与 deck 2 一并进入战斗 | setup.rs:68 |
| R2 | 警戒航行序列（11–14）的砲撃/雷撃/対潜/対空倍率按表生效，且第三/第四受 deck 2 人数限制 | reference §Formations |
| R3 | 基本攻击力含連合艦隊補正，按「敌我/编成/攻击分类/第几舰队」四维取值 | reference §Corrections |
| R4 | 阶段顺序按编成分流：空母機動先 deck 2 砲撃，水上打撃先 deck 1 砲撃 | reference §Phase order |
| R5 | deck 1 不做开幕对潜与开幕雷击；夜战只有 deck 2 参战 | reference §Phase order |
| R6 | 响应含客户端读取的 `_combined` 字段，两支舰队的 HP/装备/参数分别输出 | main.decoded.js |
| R7 | `battleresult` 的 MVP、经验、损伤跨两支舰队正确结算并落库 | 现有单舰队结算 |

---

## Scope Boundaries

### In Scope

- 端点 5 个：`battle`（空母機動）、`battle_water`（水上打撃）、
  `midnight_battle`、`battleresult`、`goback_port`
- 编成 2 种：空母機動部隊（1）、水上打撃部隊（2）
- 敌方形态 1 种：通常艦隊

### Out of Scope（非目标）

- **輸送護衛部隊（`combined_type == 3`）**——补正表已备好（deck 1 −5 / deck 2 +10），
  但它带 TP 输送量结算这套独立玩法，与战斗编排正交，另行规划。
- **敌连合舰队**——`ec_battle`、`ec_midnight_battle`、`ec_night_to_day`、
  `each_battle`、`each_battle_water` 这 5 个端点需要敌方也拆两队、还要实现
  夜战对手判定（reference §Night battle opponent selection），是独立的一大块。
- **昼战变体**——`airbattle`、`ld_airbattle`、`ld_shooting` 的 combined 版本。
  编排落地后这三个很薄，但归入后续计划以控制本计划的验证面。
- **護衛退避**（`api_escape_idx`）、友军舰队、支援舰队、基地航空队。
- **命中率补正**——wikiwiki 的命中表每格都是 `?`（未验证），不实现，
  沿用单舰队命中逻辑。reference 已写明不要把它们当作已验证数据编码。
- **combined 下的旗艦援護（かばう）**——上游同样未验证，`targeting.rs` 现有实现
  保持「combined 时不触发」。

---

## Implementation Units

### U1 — 战斗核心的联合舰队数据模型

**改动点：** `crates/emukc_battle/src/types/runtime.rs`、`types/mod.rs`、
新增 `crates/emukc_battle/src/combined.rs`

1. `BattleContext` 增加 `friend_escort: Option<Vec<BattleShipInput>>` 与
   `combined_type: Option<CombinedType>`。选 `Option` 而非把结构重构成
   `FleetSide { main, escort }`：单舰队路径传 `None`，现有 18k 行一行不动，
   `battle_golden.rs` 的 RNG 流水不受影响（见 Stop conditions）。
2. 新增 `CombinedType { CarrierTaskForce, SurfaceTaskForce }`（预留
   `TransportEscort` 的判别式但不实现其编排，范围外）。
3. 新增 `combined.rs`，承载两张纯数据表 + 取值函数：
   - `combined_formation_modifier(formation_id, attack_kind) -> f64`
     —— 11–14 四阵形，表见 reference §Formations，三种编成共用同一张表。
   - `combined_correction(context) -> i64` —— 四维取值（我方是否连合 / 编成 /
     攻击分类 / 第几舰队 / 攻守方），表见 reference §Corrections。
4. 两个函数都做表驱动单测，数值逐格对照 reference。

**索引空间（2026-09-21 已核查，结论如下）：跨队连续索引 0–11，deck 2 偏移固定
+6。** 证据在 `main-decoder/out/main.decoded.js` 的 `BattleCommonModel`：

```js
// _getNum(index, baseKey, combinedKey) — 行 123951
if (base.length > index)        return base[index];
if (index >= 6 && combined.length > index - 6) return combined[index - 6];
return default;

// getTaihiShipIndexes — 行 123908
api_escape_idx          → push(v - 1)       // deck 1
api_escape_idx_combined → push(v - 1 + 6)   // deck 2
```

`_getNumArray` / `_getParams` 用同一条判据。所以：

- deck 1 占 0–5，deck 2 占 6–11，**deck 2 的起点恒为 6，与 deck 1 的实际舰数无关**。
  deck 1 只有 4 艘时索引 4、5 是空洞（客户端按 default 处理），deck 2 第一艘仍是 6。
- `api_f_nowhps` 等基础数组按 deck 1 实际舰数输出，`_combined` 数组按 deck 2
  实际舰数输出，不要补齐到 6。
- 因此 `api_df_list` / `api_at_list` 在 combined 响应里发的是 0–11 的连续索引，
  U2/U3 无需舰队标识位来消歧。

**完成标志：** 两张表的单测通过；`cargo test -p emukc_battle` 全绿且单舰队
测试一条未改。

### U2 — 阶段编排

**改动点：** `crates/emukc_battle/src/simulation/mod.rs`

1. `simulate_day` 在 `friend_escort.is_some()` 时走联合舰队编排，否则走现有路径。
2. 两种顺序（reference §Phase order）：
   - 空母機動：航空戦 → deck2 开幕对潜 → deck2+敌 开幕雷击 → deck2 砲撃(1巡)
     → deck2 雷击 → deck1 砲撃(→2巡)
   - 水上打撃：航空戦 → deck2 开幕对潜 → deck2+敌 开幕雷击 → deck1 砲撃(→2巡)
     → deck2 砲撃(1巡) → deck2 雷击
3. deck 1 不参与开幕对潜与开幕雷击（R5）。
4. 第二巡砲撃的触发条件不变（敌我任一方有战舰系），沿用现有判定。
5. 航空战由 deck 1 + deck 2 的舰载机共同参与，制空权在此决定。

**完成标志：** 两种编成各一个阶段顺序测试，断言各阶段的参与者集合与出现次序；
单舰队编排的既有测试一条未改。

### U3 — 协议输出

**改动点：** `crates/emukc_battle/src/types/packet.rs`、`transcript.rs`

输出 reference §Protocol fields 列出的字段。两支舰队的 HP/装备/参数分别成组：
`api_f_maxhps` / `api_f_maxhps_combined` 等。`api_combined_flag` 与
`api_combined_type` 随响应下发。

**完成标志：** 一个 combined 响应的快照测试，字段齐全且分组正确。

### U4 — gameplay 舰队拆分

**改动点：** `crates/emukc_gameplay/src/game/sortie/setup.rs`、`mod.rs`、
`crates/emukc_gameplay/src/game/sortie/route_context.rs`

1. 解除 `setup.rs:68` 的拒绝，改为按 `combined_type` 解析 deck 1 + deck 2。
2. 校验：deck 2 非空；阵形 13 需 deck 2 ≥ 5，阵形 14 需 deck 2 ≥ 4（R2）。
3. sortie session 保存两支舰队的战斗状态，损伤回写覆盖两队。
4. 燃弹消耗按 12 舰计。

**完成标志：** 联合舰队出击到达战斗格不再报错；阵形校验的边界测试通过。

### U5 — 昼战端点与结算

**改动点：** 新增 `src/bin/net/router/kcsapi/api_req_combined_battle/`
（`mod.rs`、`battle.rs`、`battle_water.rs`、`battleresult.rs`、`goback_port.rs`），
在 `kcsapi/mod.rs` 注册；`crates/emukc_gameplay/src/game/sortie_result.rs`

1. 四个 handler 照 `api_req_sortie/` 的形态写，薄层转发。
2. `battleresult` 的 MVP 跨两队：deck 1 与 deck 2 各出一个 MVP，
   分别走 `api_mvp` 与 `api_mvp_combined`（R7）。
3. 经验与损伤对两队分别结算并落库。

**完成标志：** 集成测试跑完一次联合舰队出击 → battle → battleresult →
goback_port，两队的 HP 与经验都已落库。

### U6 — 夜战端点

**改动点：** `api_req_combined_battle/midnight_battle.rs`、
`crates/emukc_battle/src/simulation/night.rs`

夜战只有 deck 2 参战（R5），敌方是通常艦隊所以无对手判定问题。
`NightBattleInput` 传 deck 2 作为 friendly。夜战不吃連合艦隊補正
（reference：夜战除对潜外与通常舰队完全相同）。

**完成标志：** 昼战 → 夜战 → battleresult 链路的集成测试通过。

### U7 — 质量门

1. `cargo fmt --all --check`
2. `cargo clippy --workspace -- -W warnings`（既有 6 条 `result_large_err`
   不算新增）
3. `cargo test --workspace`（先 `mkdir -p target/tmp`，`test_font` 需要）
4. `cargo test --test gameplay_tests`
5. `apilist.md` 按机械方式重新对齐：新增 5 个端点从 missing 移到 implemented，
   数字由 114/34 变 119/29。不要手工审计，按
   `PROJECT_MEMORY.md` 记的提取方式重新 diff。
6. `battle_golden.rs` 必须一字未改——若它变了，说明单舰队路径的 RNG 流水被动了，
   触发 Stop condition。

---

## Verification

| 验收项 | 检查方式 |
|---|---|
| 补正表与陣形表数值正确 | `cargo test -p emukc_battle` 的表驱动单测 |
| 两种编成的阶段顺序正确 | U2 的顺序断言测试 |
| 联合舰队能打完一仗并落库 | `cargo test --test gameplay_tests` 的新集成测试 |
| 单舰队路径零回归 | `battle_golden.rs` 未改且通过 |
| 端点清单一致 | `apilist.md` 重新机械对齐，119/29 |
| 全套门禁 | U7 的 1–4 全绿 |

---

## Known Gaps

以下在 reference 里标为上游未验证，实现时**不要猜值**：

- 联合 vs 联合的航空战补正（要検証）
- 命中率补正的具体数值（每格都是 `?`）
- combined 下的旗艦援護触发率
- 夜战对手判定里旗舰中破/大破的具体分值
- レーダー射撃マス 的无阵形状态

每一条在代码里留 `// ponytail:` 或等价注释说明为何取当前值，不要写成
「已按规则实现」。
