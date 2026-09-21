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
  `docs/apilist.txt` 的字段语义；现有 `emukc_battle` 单舰队实现与 `CLAUDE.md`
  分层规则。

  `main.decoded.js` 与 `docs/apilist.txt` 的分工是明确的，不是冗余：客户端代码是
  字段**是否存在**的唯一真源（`docs/plans/2026-09-19-2016` 已确立这条，不改），
  但它给不出字段**含义**——哪一轮砲撃属于哪支舰队、`api_mvp_combined` 在通常舰队
  时是 null 还是缺省、`ld_shooting` 的阵形是否固定，都只有 apilist 写着。所以
  apilist 排在客户端之后、实现之前：与客户端冲突时以客户端为准，其余情况下它是
  语义来源。仓库里这份 4209 行，比 GitHub 上找得到的 `andanteyk/ElectronicObserver`
  副本（2023 停更）更全。
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
| R8 | 旗艦援護在联合舰队下生效：deck 2 旗舰不受保护，阵形 11–14 按 60% 拦截 | kcsim.js:2309（拟合值，非抓包） |

R2 与 R3 由 **U2b** 落地；计划初稿把两张表的建立（U1）与接线混为一谈，导致中间
没有单元认领接线，实施 U2 时才发现两个函数零调用点。

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

### U2 — 阶段编排 ✅ 已实施

**改动点（实际比计划宽，下面是落地后的真实清单）：**
`crates/emukc_battle/src/` 的 `simulation/mod.rs`、`targeting.rs`、
`types/runtime.rs`、`state.rs`、`simulation/asw.rs`、`simulation/torpedo.rs`、
`simulation/shelling.rs`（仅修正一处已过时的注释）。

计划原本只写了前两个。多出来的三处是实现「谁参战」必需的：`BattleRuntimeShip`
需要带上自己属于哪支 deck（`combined_role`），`BattleState` 需要把两支 deck 放进
一条连续 vec 并记下边界（`CombinedLayout`）外加第三轮砲撃的槽位，开幕对潜与
雷击的「deck 1 不参战」则是在各自的攻击者循环里按船过滤——**不能**靠切片，因为
敌方在这些阶段仍然打两支 deck。

**与单舰队模型的一处明确分歧（新增，计划原先没有）：** 单舰队路径里
`hougeki1`/`hougeki2` 各只装一方的攻击，两轮交替。联合舰队用不了这套：三轮砲撃
且每轮的 deck 由协议固定，交替会让某支 deck 整场不开火。所以联合舰队的每一轮里
敌我各打一次、合并进同一个 `BattleHougeki`，靠 `api_at_eflag` 区分——这本来就是
客户端的读法。单舰队路径一行未动，`battle_golden.rs` 与 `golden_transcript.rs`
均未变。

1. `simulate_day` 在 `friend_escort.is_some()` 时走联合舰队编排，否则走现有路径。
2. 两种顺序（reference §Phase order）：
   - 空母機動：航空戦 → deck2 开幕对潜 → deck2+敌 开幕雷击 → deck2 砲撃(1巡)
     → deck2 雷击 → deck1 砲撃(→2巡)
   - 水上打撃：航空戦 → deck2 开幕对潜 → deck2+敌 开幕雷击 → deck1 砲撃(→2巡)
     → deck2 砲撃(1巡) → deck2 雷击
3. deck 1 不参与开幕对潜与开幕雷击（R5）。
4. 第二巡砲撃的触发条件不变（敌我任一方有战舰系），沿用现有判定。
5. 航空战由 deck 1 + deck 2 的舰载机共同参与，制空权在此决定。
6. 旗艦援護（かばう）接通联合舰队（R8），改 `crates/emukc_battle/src/targeting.rs` 的
   `escort_shield_rate`（现在对 11–14 返回 `None`，即永不拦截）：
   - **deck 2 的旗舰不受保护**——这是结构规则，不是拟合数值，无条件实现。
   - 阵形 11–14 的拦截率取 **60%**。这是 `kcsim.js:2309-2319` 的兜底值
     （`[0,.45,.6,.75,.6,.6,.75][id]` 对 11–14 落空后 `rate = .6`），不是抓包
     验证过的游戏数据；代码注释必须写明来源与性质，不要写成已验证事实。
   详见 reference §Other mechanics。

**完成标志：** 两种编成各一个阶段顺序测试，断言各阶段的参与者集合与出现次序；
かばう 的两条规则各一个测试（deck 2 旗舰被直击不转移；11–14 有拦截）；
单舰队编排的既有测试一条未改。

**实际交付：** `simulation/mod.rs` 的 `combined_tests` 四个测试 + `targeting.rs`
三个 かばう 测试。参战集合用「deck 1 放潜艇、deck 2 放驱逐」来观测——
`can_shell_day_ship` 拒绝潜艇，所以「某一轮里有我方攻击」等价于「这一轮是
deck 2 的」。`main_deck_does_not_open_with_torpedoes` 配了一个反向对照测试
（同样的潜艇编成单舰队出击必须开幕雷击），否则开幕雷击整个坏掉也能让它通过。

**留给后续单元的（U2 内确认存在，不在 U2 范围）：**

- **索引翻译**：deck 2 的攻击者/被攻击者下标现在是 deck 切片内的 0..n，不是协议
  要求的 6..11。U3 负责。
- **双 MVP**：`finalize_day` 仍对整条 12 槽 vec 取一个 `calculate_mvp`，联合舰队
  应当每支 deck 各一个。U5 负责。
- **轟沈ストッパー的旗舰豁免**：`apply_damage` 与 `verify_protected_ships_alive`
  都只认 `index == 0`，连续 vec 下 deck 2 的旗舰拿不到豁免。两处判据一致所以不会
  触发断言，但这是个未定结论——第2艦隊旗艦是否享有轟沈保护，wikiwiki 与
  `kcsim.js` 都没有直接给。需要单独定夺，不要在 U3–U6 里顺手改。
- **`canOpTorpMain` 例外**：少数舰能从 deck 1 发动开幕雷击（`kcsim.js:1811`），
  当前实现一律禁止 deck 1 雷击。影响面小，未建模。

**顺带实现的**：`TransportEscort` 的阶段顺序与空母機動完全相同（reference
§Phase order 把两者并列），所以它跟着一起生效了，尽管 U1 把它列为范围外。

### U2b — 数值接线 ✅ 已实施（计划原先漏了这一单元）

**为什么存在：** U1 造好了 `combined_formation_modifier` 与
`combined_correction_vs_single` 两张表，U7 是质量门，中间没有任何单元认领「把表接进
`damage.rs`」。实施 U2 时核查发现这两个函数**全仓零调用点**（只有 `lib.rs` 的
re-export），`damage.rs` 的 `formation_modifier` 对 11–14 返回 1.0——也就是 R2 与 R3
当时事实上都没实现。本单元补上。

**改动点：** `crates/emukc_battle/src/damage.rs`、`types/runtime.rs`、
`types/mod.rs`、`state.rs`、`targeting.rs`（测试）

1. 新增 `day_formation_modifier(formation_id, class)`：11–14 走联合舰队表，其余
   落回既有的 `formation_modifier` / `asw_formation_modifier`。接在砲撃、雷撃、
   対潜三个伤害公式上（R2）。传入的恒为**攻击方**阵形，所以敌方通常舰队打我方
   连合时仍用自己的常规倍率——只有真正处在警戒航行序列的一方吃这张表。
2. 新增私有 `combined_correction(attacker, defender, class)`，接进砲撃与雷撃的
   基础攻击力加项（R3）。索引键取**我方**那条船的 `combined`，因为敌方是通常
   舰队、自己没有 deck，但它的补正仍随「跟哪支 deck 对射」变化。昼戦対潜不取
   补正，夜戦不经过这条路径——与 reference 一致。
3. `BattleRuntimeShip.combined_role` 改成 `combined: Option<CombinedMembership>`，
   同时带上 `combined_type` 与 `role`。这样补正查表不必改动 `damage.rs` 三个函数
   的签名，单舰队的船 `combined` 为 `None` → 补正恒 0、阵形落回原表，行为逐位不变。

**完成标志（已达成）：** `day_formation_modifier` 的 4 阵形 × 3 攻击分类逐格断言
+ 常规阵形 1–6 的落回断言；`combined_correction` 的双向断言（我方攻击 / 敌方攻击
各取哪一行）与单舰队恒 0 断言。`golden_transcript` 与 `battle_golden` 未变。

**实施中发现、未处理的既有偏差：** reference 的雷撃基础攻击力公式带 `+5`，但
`calculate_torpedo_damage` 的 `basic_power` 只有 `api_raisou + 改修补正`，没有那个
`+5`。这是联合舰队之外的既有偏差，改它会动摇 `battle_golden.rs`，本单元只加补正项、
不碰它。要修需单独立项并有意重新冻结。

**已知未接线：** 联合舰队表的**対空列**没有落点——`simulation/kouku.rs` 整个不处理
阵形，常规阵形 1–6 的対空倍率同样没建模。这是既有缺口，不是联合舰队引入的；要补
应当连同单舰队一起补，别只给联合舰队加一半。

### U3 — 协议输出

**改动点：** `crates/emukc_battle/src/types/packet.rs`、`transcript.rs`

输出 reference §Protocol fields 列出的字段。两支舰队的 HP/装备/参数分别成组：
`api_f_maxhps` / `api_f_maxhps_combined` 等。`api_combined_flag` 与
`api_combined_type` 随响应下发。

**砲撃轮次与舰队的对应（见 reference §Protocol fields 的两张对应表）：**
两个端点是镜像关系，**不能把一边的映射套到另一边**。

- `battle`（空母機動/輸送護衛，`docs/apilist.txt:3008`）：hougeki1 = deck 2，
  raigeki，hougeki2 = deck 1，hougeki3 = deck 1。deck 1 的两巡落在 2 号和 3 号槽，
  且雷击夹在中间——单舰队的 hougeki1/hougeki2 直觉在这里是错的。
- `battle_water`（水上打撃，`docs/apilist.txt:3164`）：hougeki1 = deck 1，
  hougeki2 = deck 1，hougeki3 = deck 2，raigeki 回到最后；并显式标注
  hougeki1/2/3/raigeki 分别受 `api_hourai_flag[0]/[1]/[2]/[3]` 约束。

`api_raigeki` 的 `api_frai`/`api_fcl`/`api_fdam`/`api_fydam` 各为 `[12]`，与 U1
核查出的跨队连续索引一致。

请求体没有 `Request.api_req_combined_battle/*` 条目（`goback_port` 除外，且标注
`(情報なし)`），按 apilist 自身约定继承单舰队的 `docs/apilist.txt:2031`：
`api_formation` / `api_recovery_type` / `api_supply_flag` / `api_ration_flag` /
`api_smoke_flag`。U5 不需要另找来源。

**完成标志：** `battle` 与 `battle_water` 各一个响应快照测试，字段齐全且分组
正确，并分别断言 hougeki1/2/3 的参与者与各自的对应表一致（两张表不同，一个测试
覆盖不了）。

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
