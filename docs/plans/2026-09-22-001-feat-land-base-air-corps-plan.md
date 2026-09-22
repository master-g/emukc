---
title: "Land Base Air Corps - Port Side - Plan"
type: feat
date: 2026-09-22
artifact_contract: ce-unified-plan/v1
artifact_readiness: in-progress
status: draft
product_contract_source: ce-plan-bootstrap
execution: code
---

# Land Base Air Corps - Port Side - Plan

## Goal Capsule

- **Objective:** 让基地航空隊在母港侧完整可用——玩家能在 6-4 / 6-5 看到自己的
  航空隊、配备与撤下中隊、改名、指定行动、补给、扩张飞行场与强化整備Lv，而
  `api_get_member/mapinfo` 与 `api_get_member/base_air_corps` 报出与数据库一致的
  中隊状态。**出击与战斗不在本计划内**（见末节）。
- **Authority order:** 本计划的 U-ID；`docs/apilist.txt` 第 2845–2925 行（本功能
  全部端点的请求/响应字段，逐字段带注释）；`main-decoder/out/main.decoded.js`
  （字段是否存在的唯一真源）；`.data/codex/start2.json` 的 `api_mst_slotitem`
  （`api_distance` 半径、`api_cost` 搭載コスト）；`CLAUDE.md` 的分层与 `_impl` 规则。

  这次 apilist 的地位比联合舰队那次更高：`base_air_corps` 一节把 `api_state`、
  `api_cond`、`api_distance` 的 `api_base`/`api_bonus` 分工都写明了，而客户端只是
  读这些字段。凡 apilist 写明含义的字段，直接照做，不再去客户端反推。
- **Execution profile:** U1 补数据模型（当前最大缺口），U2 让玩家真的拥有基地，
  U3/U4 是两组独立的写操作，U5 是扩张类，U6 收质量门。U3 与 U4 之间无依赖，
  可并行。
- **Stop conditions:** 若 U1 发现 `Airbase` 带上 `plane_info` 会迫使
  `api_get_member/record` 或 `mapinfo` 的既有响应形状改变（两者都已在用
  `get_airbases`），停下重新规划——这两个端点的形状是客户端已验证过的。
  若 U4 的补给消耗系数两条来源都取不到数，停在 U4 并把它单列，不要自己编一个
  没有出处的公式就实现。
- **Tail ownership:** U6 拥有全套质量门、`apilist.md` / `TODO.md` /
  `docs/api_coverage.md` 三处清单同步，以及 `PROJECT_MEMORY.md` 回写。

---

## 已核实的前提（不要再查一遍）

| 前提 | 证据 |
| --- | --- |
| 基地航空隊在本地常规图有落点，**不像 `ec_*` 那样卡活动图数据** | `.data/codex/map_catalog.json` 的 `maps`：`"64"` 的 `airbase_count = 1`，`"65"` 的 `= 2`，其余 35 张为 `null` |
| 数据库两张表已就绪，无需迁移 | `crates/emukc_db/src/entity/profile/airbase/{base.rs,plane.rs}`：`airbase` 表（area_id/rid/action/base_range/bonus_range/name/maintenance_level）与 `plane_info` 表（slot_id/squadron_id/state/condition/count/max_count） |
| 读取路径已接通，缺的是内容 | `api_get_member/mapinfo` 已返回 `api_air_base` 与 `api_air_base_expanded_info`，`api_get_member/record` 也在用 `get_airbases` |
| 装备半径与搭載コスト在本地 | `start2.json` 的 `api_mst_slotitem`：263 件装备有 `api_distance`，全部有 `api_cost` |
| 全部端点的字段语义有权威出处 | `docs/apilist.txt:2845-2925`，含 `set_plane` 的 `api_item_id = -1` 表示撤下、`set_action` 的 `api_base_id` 是逗号分隔数组等细节 |

### 已发现的四个缺陷（各由对应 U 修）

1. **`api_plane_info` 恒为空。** `impl From<Airbase> for KcApiAirBase`
   （`crates/emukc_model/src/profile/airbase.rs:102`）硬编码 `api_plane_info: vec![]`，
   而 `Airbase` 结构里根本没有中隊字段。所以即使 `plane_info` 表有行，
   `mapinfo` 报出的基地也永远是零中隊。→ U1
2. **`Airbase.id` 语义冲突。** 字段注释写 "Profile id"，`impl From<Model> for Airbase`
   填 `value.profile_id`，但 `get_airbases` 手写构造时填 `v.id`（实例 id）。
   两条路径产出不同的值。该字段目前不进任何 API 响应，所以是潜伏的而非已发作的。→ U1
3. **未配属の中隊带了不该有的字段。** `From<PlaneInfo> for KcApiPlaneInfo` 无条件填
   `Some(...)`，但 apilist 对 `api_count` / `api_max_count` / `api_cond` 三个字段都
   标注「未配属なら存在しない」。实施 U1 时发现并一并修掉。
4. **没有任何人调用 `unlock_airbase`。** `airbase::init` 是空函数，
   `unlock_airbase` 在整个仓库零调用方。玩家永远没有基地，上面两条因此从未发作。→ U2

---

## Implementation Units

### U1 — 让 `Airbase` 带上中隊，并钉死 `id` 语义 ✅ 已执行

**做什么**

- `Airbase` 增加 `planes: Vec<PlaneInfo>`，`From<Airbase> for KcApiAirBase` 改为
  由它填 `api_plane_info`，删掉硬编码的 `vec![]`。
- 钉死 `Airbase.id` 为**实例 id**（`airbase` 表主键），改正
  `impl From<Model> for Airbase` 并把注释从 "Profile id" 改对。
  `impl From<Airbase> for ActiveModel` 删掉：`Airbase` 不再能提供 `profile_id`，
  它原本就是把 `t.id` 当 profile_id 写进去的，且全仓库零调用方。
- 实现 `crates/emukc_gameplay/src/game/airbase/plane.rs`（当前是 1 行空壳）：
  `get_planes_impl` 读已配属的槽，`squadrons_of` 合成缺口。
  **不预建空槽行**：`plane_info` 以装备实例 id 为主键，空槽没有 id 可用，
  4 个空槽会全部撞在 `slot_id = 0` 上。空槽因此不落库，读的时候补出来。
- 半径计算 `distance_of(planes, codex) -> (base, bonus)`：`api_base` 取已配属中隊里
  `api_distance` 的最小值（未配属中隊不参与，空基地为 0），`api_bonus` 先恒为 0 并留
  `// ponytail:` 注明偵察機 bonus 属于 U3 之后的独立取数。依据 `apilist.txt:2849-2851`。

**为什么先做它** 四个缺陷里三个在这一层，且后面每个写端点都要返回
`api_plane_info`——不先修，每个 U 都得绕一次。

**完成标志** `get_airbases` 返回的基地带中隊；`mapinfo` 与 `record` 的响应形状
不变（字段只从空数组变为有内容）；新增一个单测断言半径取最小值且空基地为 0。

---

### U2 — 让玩家真的拥有基地，并补 `base_air_corps` ✅ 已执行

**做什么**

- 惰性补齐：`get_airbases` 先调 `ensure_airbases_impl`，对每个**有已解锁的图声明了
  `airbase_count` 的 area** 补齐第一个航空隊。惰性而非攻略时触发，理由是
  `get_airbases` 是 `mapinfo` / `record` / `base_air_corps` 三个读接口的共同入口，
  幂等（`unlock_airbase_impl` 是 find-or-insert），且不需要在 map 攻略流程里插跨域写。

  **草案写错了一处**：原文说「补齐 `rid = 1..=airbase_count`」。`airbase_count`
  （`api_air_base_decks`）是 apilist `:2822` 的「基地航空隊**出撃可能数**」——
  一张图能派出几个航空隊，不是玩家拥有几个。6-4 与 6-5 都在 area 6，按原文会给
  area 6 发 3 个基地。实际是：area 拿到第 1 个航空隊，其余靠 `expand_base` 用設営隊
  买，上限 `AIRUNIT_MAX`。已按后者实现，`areas_entitled_to_air_corps` 是那条判定。
- 新增 `api_get_member/base_air_corps` 端点，返回 `Vec<KcApiAirBase>`。
  apilist 注明它「现在は直接使用されていない」，但数据结构是其余端点的公共形状，
  实现它等于把 U1 的输出接上一条可直接验证的读路径。

**为什么先做它** 没有基地，U3–U5 的每个写操作都无从测试。

**完成标志** 新档没有航空隊；解锁 6-4 后 area 6 拿到第 1 个，且 6-5 一起解锁
时仍只有 1 个。实际覆盖：`areas_entitled_to_air_corps` 的单测（跑真实 codex，
断言 6-4 locked → 无、6-4 → [6]、6-4+6-5 → [6]），加
`base_air_corps` 的两个端点测试（新档为空；一个空航空隊报出 4 个裸槽，
且三个 optional 字段确实缺省）。

---

### U3 — 中隊配备：`set_plane` 与 `change_deployment_base`

**做什么**

- `set_plane`（`apilist.txt:2861-2872`）：`api_item_id = -1` 撤下，否则配属。
  配属消耗ボーキサイト，响应 `api_after_bauxite` 仅在消耗时存在；响应的
  `api_plane_info` 只含被更新的槽（配备时 1 个、交换时 2 个）。
- `change_deployment_base`（`:2882-2888`）：同海域内两个航空隊整组交换，
  响应 `api_base_items` 是两个基地的完整数据。
- 校验：装备必须是玩家所有、未装备在舰上、属于陸上機可配属的类型；
  `squadron_id` 在 `1..=SQUADRON_MAX` 内。草案把 4 写成「U5 之前的暂定值」，
  实际它有出处：客户端 `main.decoded.js:85380` 的 `SQUADRON_MAX = 0x4`，
  同行还有 `AIRUNIT_MAX = 0x3`。客户端画固定行数，从不按响应长度推断，
  所以服务端必须每个槽都发一条 `api_plane_info`。

**完成标志** 配备后 `mapinfo` 的 `api_plane_info` 与 `api_distance` 同步变化；
撤下后该槽 `api_state = 0` 且不再带 `api_count`/`api_cond`（apilist 明确「未配属なら存在しない」）；
交换后两个基地的中隊互换。

---

### U4 — 行动、补给与改名：`set_action`、`supply`、`change_name`

**做什么**

- `set_action`（`:2890-2896`）：`api_base_id` 是逗号分隔数组，一次可改多个基地；
  `api_action_kind` 同样是数组，逐位对应。响应无内容。
- `change_name`（`:2874-2880`）：改名，响应无内容。
- `supply`（`:2907-2915`）：按 `api_squadron_id` 列表补齐不足機数，扣燃料与ボーキサイト，
  响应 `api_after_fuel` / `api_after_bauxite` / 更新后的 `api_plane_info`。

**前置取数（U4 唯一不齐的点）** 补给消耗系数上游不公开：apilist 只写响应字段，
客户端也没有消耗预览（`grep` 过 `getSupplyCost` / `calcSupply` 等，零命中——
它只读响应里的 after 值）。两条候选来源，按序试：
①  wikiwiki 基地航空隊页的補給消費表；②  `KC3Kai/kancolle-replay` 的 `js/kcsim.js`
（`PROJECT_MEMORY` 已确认它是独立数据链）。
两条都取不到就停在 U4 并单列，**不要编一个没有出处的公式**。若最终由本项目定值，
按 `recover_success_rate` 的先例处理：常量放 `emukc_model` 的 codex 模块、
注释写明是本项目选值、加回归测试；若落到 `Default` impl 上则触发
Balance Defaults Policy（独立提交、`feat(balance):`、正文列旧值）。

**完成标志** 三个端点各有端到端测试；`supply` 的消耗有出处或明确标注为项目定值
并带回归测试。

---

### U5 — 扩张类：`expand_base`、`expand_maintenance_level`、`cond_recovery`

**做什么**

- `expand_base`（`:2917-2918`）：**不是扩中隊容量**（草案写错了）。客户端类
  `AirUnitExtendAPI` 的 `_completedEnd` 走 `model.airunit.addData(raw_data[0])`——
  是 add 不是 update——并扣 1 个 useItem **73（設営隊）**。所以它在该 area
  **新增一个航空隊**，上限 `AIRUNIT_MAX = 3`。
- `expand_maintenance_level`（`:2920-2925`）：整備Lv 强化，按 area 生效，
  写入 `airbase.maintenance_level`，`mapinfo` 的 `api_air_base_expanded_info` 已在读它。
- `cond_recovery` 与 `api_port/airCorpsCondRecoveryWithTimer`：疲劳恢复。
  前者是立即恢复，后者是带计时器的查询，`api_port/port` 侧已有类似的 timer 形状可循。

**完成标志** 三处写操作落库且被 `mapinfo` 读出；`expand_base` 后该 area 多出
一个航空隊（各带 `SQUADRON_MAX` 个空槽），到 `AIRUNIT_MAX` 为止拒绝继续。

---

### U6 — 质量门与清单

`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`
（基线 17 条，新代码零告警）、`cargo test --workspace` exit 0。
`apilist.md` 的 implemented 列表机械重推、`TODO.md` 的 Air Corps 段与顶部计数、
`docs/api_coverage.md` 的路线图三处同步。`PROJECT_MEMORY.md` 回写与实现同一提交。

**预期计数** 本计划覆盖 10 个端点（`base_air_corps`、`airCorpsCondRecoveryWithTimer`、
`set_plane`、`set_action`、`supply`、`change_name`、`change_deployment_base`、
`expand_base`、`expand_maintenance_level`、`cond_recovery`），
127 → 137 implemented，22 → 12 missing。

---

## 不在本计划内：出击与战斗

`api_req_map/start_air_base` 与战斗包里的基地航空隊阶段是另一个量级，单列的理由：

- **它牵动 `emukc_battle`，而基地航空隊在那里零命中**（`grep air_base|airbase|AirBase`
  在 `crates/emukc_battle/src/` 无结果）。要新增 `api_air_base_attack`
  （`apilist.txt:2121`，攻击回数数组）、`api_air_base_injection`（`:2075` 噴式強襲）
  与防空侧的 `api_air_raid`，全部要插进既有的航空阶段。
- **它会让 golden 全量重冻结。** 新增战斗阶段必然改动 RNG 消耗顺序，
  `crates/emukc_battle/tests/golden/*.txt` 与 `tests/gameplay_tests/battle_golden.rs`
  都要有意重新冻结并在 PR 说明差异。
- **`start_air_base` 本身几乎是空的**（请求只有三组 `api_strike_point_N`，
  响应「情報なし」），只有战斗侧消费 strike point 之后它才有意义。单独实现是
  写一个没人读的写操作。

先决条件：本计划 U1–U5 完成（战斗侧要读中隊的機数、半径与行动指示）。
