---
title: "Align Server APIs with 6.3.x Upstream - Plan"
type: feat
date: 2026-08-26
artifact_contract: ce-unified-plan/v1
artifact_readiness: implemented
status: completed
product_contract_source: ce-plan-bootstrap
execution: code
deepened: 2026-08-26
---

# Align Server APIs with 6.3.x Upstream - Plan

## Goal Capsule

- **Objective:** 补齐官方 6.3.0.0–6.3.4.0 版本间 emukc 缺失的 API 能力：搭载数扩张系统（新端点 + 状态 + 道具消耗）、活动图新字段、表现层字段透传，使当前客户端（6.3.4.1）交互正常。
- **Authority order:** 本计划的 U-ID 与 KTD-ID；`api_info/apilist.txt`（sinsinpub/kcs2-assets，6.2.9.1→6.3.4.0 全量 diff）；现有模型、路由与 gameplay 约定；`AGENTS.md` 分层规则。
- **Execution profile:** 模型与透传先行（U1/U2），玩法与端点随后（U3/U4），独立字段组并行（U5/U6），最后集成验证（U7）。
- **Stop conditions:** 若实现需要改动战斗数值抽取顺序、重启改修工廠功能、或实现完整多段血条攻略逻辑，应停止并回到规划（属范围外）。
- **Tail ownership:** U7 拥有 apilist.md 一致性与全套质量门。

---

## Product Contract

### Summary

上游在 6.3.x 周期引入了搭载数扩张玩法（道具「格納庫増設」useitem 105 + `api_req_kaisou/hangar_expand` 端点 + `api_onslot_max` 响应字段优先于 `api_maxeq`），并为活动图增加了难度限制与多段血条字段。emukc 当前全部缺失：客户端对使用了扩张道具的舰船显示异常，活动图新字段静默丢弃。

### Problem Frame

经 `api_info/apilist.txt` 全量 diff 核查（基线 6.2.9.1），缺失项分三层：

| 层 | 项目 | 上游版本 | emukc 现状 |
|---|---|---|---|
| 玩法 | `api_req_kaisou/hangar_expand` 端点 | 6.3.1.0 | 无路由 |
| 玩法 | port 舰船 `api_onslot_max[5]`（优先于 `api_maxeq`） | 6.3.1.0 | 模型无字段，无每舰扩张状态 |
| 玩法 | `api_mst_stype.api_max_slotplus`（扩张上限） | 6.3.1.0 | 模型无字段；start2 数据源亦暂缺 |
| 活动 | `api_eventmap.api_limit_flag`（难度限制） | 6.3.2.0 | 模型无字段 |
| 活动 | `mapinfo.api_gauge_type_e`（多段血条） | 6.3.3.1 | 模型无字段，MapDefinition 无字段 |
| 表现 | `api_mst_shipgraph.api_sp_flag` | 6.3.2.0 | 模型无字段；start2 数据源亦暂缺 |
| 表现 | port `api_event_object`（`api_c_num`、`api_m_flag` 0~2 语义） | event 62 | port.rs 中整块注释掉 |

已确认事实：`api_start2/getData` 由 `state.codex.manifest` 重新序列化输出，解析期未建模的 start2 字段被静默丢弃；useitem 105 已存在于当前 start2 数据；ship DB 实体有 `onslot_1..5`（当前搭载）但无扩张状态；emukc 允许自由选择活动难度。

### Requirements Trace

| R-ID | 需求 | 来源 |
|---|---|---|
| R1 | 客户端调用 `hangar_expand` 后获得正确的 `api_onslot_max`，道具 105 被消耗，扩张状态跨会话持久 | apilist 6.3.1.0 diff |
| R2 | 已扩张舰在 port 响应中携带 `api_onslot_max`，且容量语义优先于 `api_maxeq` | apilist 6.3.1.0 diff |
| R3 | start2 中 `api_max_slotplus`、`api_sp_flag` 出现时被透传（数据源当前缺失不报错） | apilist 6.3.1.0/6.3.2.0 diff + start2 数据核查 |
| R4 | 活动图 mapinfo 携带 `api_limit_flag`（固定 0=不限制）与 `api_gauge_type_e`（数据存在时） | apilist 6.3.2.0/6.3.3.1 diff |
| R5 | port 恢复输出 `api_event_object`，含新 `api_m_flag` 语义 | apilist event 62 diff |

---

## Scope Boundaries

### In Scope

- 上述 R1–R5 的模型、DB、gameplay、路由、测试全链路
- ship 表新增扩张状态列及其迁移

### Out of Scope（非目标）

- 改修工廠 `remodel_slotlist` 及其字段变更——该端点本身未实现，属独立功能，另行规划
- `api_gauge_type_e` 的多段血条攻略逻辑（出击/战斗层）——仅建模与序列化，逻辑深度另立计划
- `api_limit_flag` 的真实司令部等级限制——固定发 0
- decoder 从 main.js 提取扩张上限客户端定义——宽松校验阶段不需要

### Deferred to Follow-Up Work

- 改修工廠功能计划（含 `api_sp_type` 新语义）
- 多段血条（`_e` 资源）攻略逻辑计划
- start2 数据源补齐 `api_max_slotplus` 后的严格校验收紧
- `api_c_num` 友军舰队表的实际数据

---

## Key Technical Decisions

| KTD-ID | 决策 | 理由 |
|---|---|---|
| KTD1 | 扩张状态存 ship 表新列 `onslot_plus_1..5`（可空，**相对增量**：NULL=未扩张，存 +N 而非绝对值）；`api_onslot_max` 为派生只输出口：`From<Model>` 恒置 None，gameplay 读包装（`api_sp_effect_items` 同款 post-fill）按 `manifest_maxeq[i] + plus[i]` 合成，任一槽非零才输出字段 | 相对增量使改造天然正确（remodel 走 `codex.new_ship` 重建 + `NotSet` 列不写回，扩张态自动保留）；`KcApiShip` 无 `api_maxeq` 字段且改造后基础值变化——绝对值存储会在改造后过期，且合成值回写增量列会二次叠加；只输出口 + 包装层合成将回填单点化 |
| KTD2 | start2 透传字段（`api_max_slotplus`、`api_sp_flag`）用 `Option` + `serde(default)` + `skip_serializing_if` | 数据源到位即生效，缺失时输出不变；零破坏 |
| KTD3 | `hangar_expand` 校验从宽：不校验 `api_max_slotplus` 上限（缺失时 warn 日志），仅校验归属、slot_pos 范围、道具余量 | 上限数据当前不可得；威胁模型：单机模拟器，存档即用户自己的，刷道具无跨用户威胁，与 KTD5（自由选难度）立场一致，实现期无需反复纠结；收紧点单点明确（gameplay 校验处） |
| KTD4 | 每次扩张消耗 1× useitem 105，对应槽增量 +1——**待验证假设**（全仓零旁证）：增量偏小可自愈（客户端按服务端返回展示，再扩张收敛向上），偏大不收敛 | 官方增量未知；Risk 表挂真机冒烟验证项；若后续 decoder 证明增量不同，修正点单一（gameplay 单处） |
| KTD5 | `api_limit_flag` 对所有活动图固定 `Some(0)` | emukc 允许自由选难度；0=不限制与现状一致 |
| KTD6 | `gauge_type_e` 在 `MapDefinition` 建模并透传 wikiwiki 目录解析，`build_map_info` 存在即发射 | 客户端显示对齐；攻略逻辑不在此计划 |
| KTD7 | `api_event_object` 恢复输出：`api_m_flag` 固定 **2**（活动图连合编成 UI 能力标志：0=仅機動/水上、1=仅輸送、2=全可——非玩家当前编成，后者是已输出的 `api_combined_flag`），`api_c_num` 缺省省略，`api_m_flag2` 缺省省略 | emukc 支持 0..=3 含輸送部隊，固定 2 与实际能力一致；0 会自相矛盾（可选輸送但 UI 被锁）；省略字段是官方允许形态（"存在しない"分支） |
| KTD8 | 扩张后容量优先级：读取点已审计完毕，波及面＝`compose/supply.rs:92`（补给）、`compose/marriage.rs:54`（婚后重置）、`codex/ship.rs:66`（新舰初始）、`sortie/enemy_ship.rs:169`（敌舰降级）四处，均改为扩张合成值优先；**`emukc_battle` 零 `api_maxeq` 读取**（只读 `api_onslot`，扩张经 gameplay 填充后自然流入） | apilist 明示 "存在するなら api_maxeq より優先的に参照される"；battle 无波及使 Stop condition 收窄为"不改 emukc_battle 内 onslot 抽取逻辑"（预期完全不触 battle crate） |

---

## High-Level Technical Design

`hangar_expand` 请求时序（directional guidance，非实现规范）：

```
client POST api_req_kaisou/hangar_expand {api_ship_id, api_slot_pos}
  → handler: session → profile_id，调 gameplay trait
  → gameplay tx:
      find ship (归属校验) →
      slot_pos ∈ [0,5) →
      deduct_use_item_impl(&tx, profile, 105, 1)   // 复用 deduct _impl；
      // ⚠️ 不可用 consume_use_item（公开方法自带 begin/commit，外层事务会
      //    裂成独立事务非原子）也不可用 consume_use_item_impl
      //    （KcUseItemType 枚举只到 68，105 直接 WrongType）
      onslot_plus[pos] += 1 → 持久化
  → response: { api_onslot_max: [i64;5] }
      // 全槽数组 = manifest_maxeq[i] + onslot_plus[i] 动态合成（含未扩张槽基础值）
```

关键点（directional guidance，非实现规范）：`api_onslot_max` 返回**全槽数组**（未扩张槽=基础 `api_maxeq` 值），而 port 响应中该字段**仅在该舰任一槽发生过扩张时出现**——两处语义不同，测试需分别覆盖。

`api_onslot_max` 是**派生的只输出口**（deepening 定案，经代码验证）：DB `onslot_plus_*` 列为唯一事实源；`From<Model> for KcApiShip` 恒置 `None`（db 层无 Codex，无法合成）；合成发生在 gameplay 读包装（`find_ship`/`get_ships`），按 `api_sp_effect_items` 同款 post-fill 模式（impl 有 `self.codex()` 取 `manifest_maxeq`）。写路径（`update_ship_impl` 与 remodel 的 `From<KcApiShip>→ActiveModel`）不对 plus 列取值（`NotSet`）——sea_orm update 不写 `NotSet` 列，**改造天然保留扩张态**，U3 改造用例作回归护栏；同时禁止把 `api_onslot_max`（绝对值）回写 plus 列，防止"合成值回灌增量列"的二次叠加腐坏。

---

## Implementation Units

### U1. start2 透传字段建模

- **Goal:** `ApiMstStype` 与 `ApiMstShipgraph` 增加可选透传字段（R3）。
- **Requirements:** R3
- **Dependencies:** 无
- **Files:**
  - `crates/emukc_model/src/kc2/start2.rs`（两处 struct + 文件内 tests）
- **Approach:** `api_max_slotplus: Option<i64>`（ApiMstStype）、`api_sp_flag: Option<i64>`（ApiMstShipgraph），均 `#[serde(default, skip_serializing_if = "Option::is_none")]`。
- **Patterns to follow:** 同文件既有 `Option` 字段（如 shipgraph 的 `api_pa`）。
- **Test scenarios:**
  - 反序列化含新字段的 fixture → 字段值保留；序列化输出含该字段
  - 反序列化不含新字段的 fixture（当前数据形态）→ None；序列化输出不含该字段
  - 全量真实 `.data/codex/start2.json` 解析 → 不报错，两字段均为 None
- **Verification:** `cargo test -p emukc_model` 通过；start2 响应字节级不变（字段缺失时）。

### U2. 扩张状态 DB 列与模型管道

- **Goal:** ship 表新增 `onslot_plus_1..5` 可空列（相对增量）并迁移；`KcApiShip.api_onslot_max` 建模；DB↔API 转换双向打通（R1/R2 数据基础）。
- **Requirements:** R1, R2
- **Dependencies:** U1（同 crate 序列化约定先定）
- **Files:**
  - `crates/emukc_db/src/entity/profile/ship/mod.rs`（列 + 迁移函数挂 `ship::bootstrap` 末尾；`From<Model> for KcApiShip` 补映射）
  - `crates/emukc_model/src/kc2/api/mod.rs`（`KcApiShip.api_onslot_max: Option<[i64;5]>`）
  - `crates/emukc_model/src/codex/ship.rs`（`new_ship`/`new_enemy_ship` 两处 struct 字面量构造——加非 Default 字段会编译失败，必须补）
  - `crates/emukc_gameplay/src/game/ship/mod.rs`（KcApiShip→ActiveModel 写回、Model→KcApiShip 读取、`update_ship_impl` 三段转换）
  - `crates/emukc_gameplay/src/game/sortie/enemy_ship.rs`（struct 字面量构造点）
  - `crates/emukc_battle/src/test_utils.rs`（测试构造点）
- **Approach:** 存相对增量（KTD1）：`From<Model> for KcApiShip` 恒置 `api_onslot_max: None`（db 层无 Codex 不能合成）；合成在 gameplay 读包装（`find_ship`/`get_ships`，`api_sp_effect_items` 同款 post-fill，impl 有 `self.codex()`），任一槽非零才输出字段。迁移遵循 map_record 模式（`PRAGMA table_info` 探测 + 条件 `ALTER TABLE`），挂 `ship::bootstrap` 末尾，`profile::bootstrap → entity::bootstrap` 链路自动带上（启动时执行、幂等）；**新列不得带 `NOT NULL DEFAULT`**（NULL 本身即"未扩张"语义）。写路径纪律：`update_ship_impl` 与 `From<KcApiShip>→ActiveModel` 对 plus 列取 `NotSet`（保留库中现值）——既防 find→update 往返丢状态，也防把合成后的绝对值回灌增量列（二次叠加腐坏）。旧数据/新舰兼容已验证：`add_ship_impl` 逐字段构造，None → NotSet → NULL。
- **Patterns to follow:** `crates/emukc_db/src/entity/profile/map_record.rs` 的 `migrate_unlocked_column`（挂载与探测方式，但不要学它的 NOT NULL DEFAULT）。
- **Test scenarios:**
  - 旧库（无新列）迁移后可读写扩张状态
  - 全 NULL 舰 → `api_onslot_max` 为 None → 序列化无字段
  - 部分槽非零 → 合成数组正确（manifest 基础值 + 增量，经 gameplay 读包装）；`update_ship` 后 `find_ship` 往返一致（plus 列保留，合成值不回灌）
  - **往返回归：** 扩张后走任一现有 `update_ship` 路径（如补给），扩张态不被冲掉
  - 新舰 `add_ship` 不携带扩张字段（None 语义保持）
- **Verification:** `cargo test -p emukc_db -p emukc_gameplay -p emukc_battle` 通过（构造点补齐后全 crate 编译）。

### U3. hangar_expand gameplay 操作

- **Goal:** 扩张核心逻辑：校验、道具 105 消耗（同事务）、状态更新、返回全槽数组（R1）。
- **Requirements:** R1, R2（KTD3/KTD4/KTD8）
- **Dependencies:** U2
- **Files:**
  - `crates/emukc_gameplay/src/game/ship/mod.rs` 或新 `ship/hangar.rs`（trait 方法 + `_impl`）
  - `crates/emukc_gameplay/src/game/compose/supply.rs`、`crates/emukc_gameplay/src/game/compose/marriage.rs`（KTD8 读取点改造）
  - `tests/gameplay_tests.rs`（注册新测试文件：`#[path = "gameplay_tests/ship/hangar_expand.rs"] mod hangar_expand;`——未注册的测试文件不会被编译，验证要求新测试名出现在 cargo test 输出）
  - `tests/gameplay_tests/ship/hangar_expand.rs`（新）
  - 注：KTD8 另两处读取点（`codex/ship.rs:66`、`sortie/enemy_ship.rs:169`）随 U2 已列文件顺带完成
- **Approach:** trait `expand_hangar_slot(profile_id, ship_id, slot_pos) -> Result<[i64;5]>`；事务内复用 **`deduct_use_item_impl`**（只收 `C: ConnectionTrait`、不自开事务；⚠️ 不可用 `consume_use_item` 公开方法——自带 begin/commit 会把外层事务裂成独立事务非原子；也不可用 `consume_use_item_impl`——`KcUseItemType` 枚举无 105（最大 102），useitem 105 直接 WrongType）。**归属校验为规范性要求**：find 后显式比较 `ship.profile_id != profile_id → EntryNotFound`（参照 `toggle_ship_locked_impl`）；⚠️ `find_ship_impl` 不带 profile 过滤，且最近似模板 `open_ship_exslot_impl`（道具消耗+舰船变更同款形状）恰缺此校验——禁止照抄。响应数组在事务内合成后返回（commit 后无需再查 codex）。KTD8 波及面（已审计）：`compose/supply.rs:92`、`compose/marriage.rs:54`、`codex/ship.rs:66`、`sortie/enemy_ship.rs:169` 四处读取点改为合成值优先；`emukc_battle` 零 `api_maxeq` 读取，预期不触 battle crate。
- **Patterns to follow:** `crates/emukc_gameplay/src/game/ndock.rs`（事务 + `_impl` 复用）与 `use_item.rs` 消耗路径。
- **Test scenarios:**
  - 正常：持有道具 105 → 扩张槽 2 → 道具 -1、`onslot_plus[2]` +1、返回全槽数组（未扩张槽=基础 maxeq）
  - 错误：非本人舰（**构造第二个 profile 实际跨越边界**）/ `slot_pos` 越界 / 道具不足 → 各自 `GameplayError` 变体，状态不变（含道具与扩张列均不变）
  - 集成：扩张后 port 的 `get_ships` 响应携带 `api_onslot_max` 且值正确（R2）
  - 集成：扩张舰的补给容量上限取合成值而非 `api_maxeq`（KTD8）
  - **改造交互：** 扩张后 remodel（走 `codex.new_ship` 整舰重建路径）→ 扩张态自动保留（plus 列 NotSet、update 不写——回归护栏用例，见设计关键点），断言改造后 port 合成值仍含旧增量
  - **婚后重置：** 扩张舰触发 marriage onslot 重置 → 重置到合成值而非裸 `api_maxeq`（不低填）
  - **连续扩张：** 同槽二次扩张 → 增量叠加正确、道具再扣 1
- **Verification:** 新测试文件全绿（**测试名出现在 cargo test 输出**，防空注册）；既有 gameplay 套件不回归。

### U4. hangar_expand HTTP 端点

- **Goal:** 注册 `api_req_kaisou/hangar_expand` 路由与 handler（R1）。
- **Requirements:** R1
- **Dependencies:** U3
- **Files:**
  - `src/bin/net/router/kcsapi/api_req_kaisou/hangar_expand.rs`（新）
  - `src/bin/net/router/kcsapi/api_req_kaisou/mod.rs`（注册）
  - `apilist.md`
- **Approach:** Form 参数 `api_ship_id`/`api_slot_pos`；响应体仅 `api_onslot_max: [i64;5]`。
- **Patterns to follow:** `api_req_kaisou/slot_deprive.rs`（参数 + trait 调用 + `KcApiResponse::success`）。
- **Test scenarios:**
  - Test expectation: none —— HTTP 层为薄封装，行为已由 U3 gameplay 测试覆盖；以构建 + 路由注册冒烟（`cargo build` + apilist 勾选）为验证
- **Verification:** `apilist.md` Implemented 列表新增该端点；`cargo build` 通过。

### U5. 活动图字段（limit_flag / gauge_type_e）

- **Goal:** mapinfo 响应对齐：`api_limit_flag` 固定 0；`api_gauge_type_e` 数据存在时发射（R4）。
- **Requirements:** R4（KTD5/KTD6）
- **Dependencies:** 无（可与 U1–U4 并行）
- **Files:**
  - `crates/emukc_model/src/kc2/api/mod.rs`（`KcApiEventmap.api_limit_flag`、`KcApiMapInfo.api_gauge_type_e`）
  - `crates/emukc_model/src/codex/map/types.rs`（`MapDefinition.gauge_type_e`；⚠️ struct 字面量构造点须同步补：`types.rs minimal()`、`codex/map.rs` 测试字面量、`gameplay/map.rs`/`map_progress.rs`/`sortie/enemy_ship.rs`/`sortie_result.rs`/`sortie_tests.rs` 测试 helper、`tests/gameplay_tests` 内字面量——加非 Default 字段直接编译失败，编译器会逐个指出）
  - `crates/emukc_model/src/codex/map/merge.rs` 与 wikiwiki 目录解析（透传；`MapCatalog` 为 serde JSON，`Option` 字段随 bootstrap 管道自然流转）
  - `crates/emukc_gameplay/src/game/map.rs`（`build_map_info` 内 eventmap 组装）
- **Approach:** `build_map_info` 对 `definition.is_event` 的图发射 `api_limit_flag: Some(0)`；`gauge_type_e` 从定义透传，`Option` 缺省省略。wikiwiki 目录资产再生成（`emukc-scrape-wikiwiki-mapdata` 流程）不在本单元——字段就绪，数据到位即出。
- **Patterns to follow:** `KcApiEventmap` 既有可选字段（`api_s_no` 等）的发射方式。
- **Test scenarios:**
  - 活动图（fixture map_id 621）→ `api_eventmap.api_limit_flag == Some(0)`
  - 常规图 1-1 → 无 `api_limit_flag`
  - 定义含 `gauge_type_e` → mapinfo 发射同值；不含 → 字段缺失
  - 既有 `tests/gameplay_tests/map/` 套件不回归（event 62 可见性断言）
- **Verification:** `cargo test --test gameplay_tests map` 通过。

### U6. port api_event_object 恢复

- **Goal:** port 响应恢复 `api_event_object` 输出（R5，KTD7）。
- **Requirements:** R5
- **Dependencies:** 无（可并行）
- **Files:**
  - `crates/emukc_model/src/kc2/api/mod.rs`（`KcApiPortEventObject`，若尚无）
  - `src/bin/net/router/kcsapi/api_port/port.rs`（取消注释/组装——port 响应完全在此文件组装，gameplay 无 port 构建函数；KTD7 全固定值，零 gameplay 改动）
- **Approach:** 输出 `api_m_flag` 固定 **2**（活动图连合编成 UI 能力标志，0=仅機動/水上、1=仅輸送、2=全可；非玩家当前编成），`api_c_num`/`api_m_flag2` 省略。客户端对省略字段有"存在しない"分支，最小输出安全。
- **Patterns to follow:** port.rs 相邻字段的组装与注释块；测试跟随 port.rs 内既有 `#[cfg(test)]` 模块的 `new_game_session` 模式（new_mem_db + `.data/codex`）。
- **Test scenarios:**
  - port 响应含 `api_event_object` 且 `api_m_flag == 2`（全可，与 emukc 联合舰队支持 0..=3 一致）
  - 省略字段不出现（`api_c_num`/`api_m_flag2` 缺席）
- **Verification:** port.rs 内 `#[cfg(test)]` 模块新增用例通过（`cargo test -p emukc --bin emukcd port`）；`tests/gameplay_tests` 无法访问 bin crate 私有函数，不在此处建测试。

### U7. 集成验证与文档收尾

- **Goal:** 全链冒烟 + apilist 一致性 + 质量门（R1–R5 汇合）。
- **Requirements:** R1–R5
- **Dependencies:** U1, U2, U3, U4, U5, U6
- **Files:**
  - `tests/gameplay_tests.rs`（注册新端到端测试文件：`#[path] mod` 声明——未注册不编译）
  - `tests/gameplay_tests/api_alignment_e2e.rs`（新；端到端用例：注册 → 加舰 → 加道具 105 → 扩张 → port 携带 `api_onslot_max` 与 `api_event_object`（`api_m_flag == 2`）→ mapinfo 携带 `api_limit_flag`）
  - `apilist.md`
- **Approach:** 一条测试走通三个功能组的主路径，作为回归锚。
- **Test scenarios:**
  - 端到端主路径如上；断言各响应字段存在与值
- **Verification:** 端到端测试名出现在 cargo test 输出且全绿；`cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings`（本次改动文件零警告）、`cargo test` 全绿、无 skipped 未申报。R3 不走端到端（数据源缺字段）——由 U1 fixture 负向覆盖，见 Origin。

---

## System-Wide Impact

- **持久化 schema：** ship 表加列（带迁移，旧库无损；迁移挂 `ship::bootstrap`，启动时执行，幂等）。
- **客户端契约：** `KcApiShip`/`KcApiMapInfo`/`KcApiEventmap`/port 响应新增可选字段——仅增不改，旧客户端不受影响。
- **crate 边界：** model → gameplay → bin 单向依赖不变。已审计：`emukc_battle` 零 `api_maxeq` 读取（只读 `api_onslot`，扩张经 gameplay 填充后自然流入）——预期不触 battle crate；Stop condition 收窄为"不改 emukc_battle 内 onslot 抽取逻辑"。构造点波及面：`emukc_battle/src/test_utils.rs` 仅测试 fixture。
- **事务边界：** `deduct_use_item_impl` 与 `update_ship_impl` 均只收 `ConnectionTrait`、不自开事务，可共处同一 tx（ndock 同款模式）；`consume_use_item` 公开方法自带 begin/commit，禁止在外层事务内调用。
- **运维：** start2 第三方数据源补齐 `api_max_slotplus`/`api_sp_flag` 后自动透传，无需代码变更；严格校验收紧为后续单点。

---

## Risk Analysis & Mitigation

| 风险 | 缓解 |
|---|---|
| ~~KTD8 审量审计波及战斗数值路径~~ 已审计解除：battle crate 零 `api_maxeq` 读取，波及面=gameplay/codex 四处读取点（见 KTD8） | 无需缓解；Stop condition 保留为防御性边界 |
| 扩张增量假设（+1）与官方不符——客户端按钮启用条件可能与服务端状态错位 | KTD4 单点决策；**真机冒烟验证**（需真实客户端交互）；decoder 后续提取可验证，修正点唯一 |
| find→update 往返丢扩张态（`update_ship_impl`/`From<Model>` 未同步映射） | U2 已列为显式步骤 + 专项往返回归用例 |
| `api_event_object` 字段语义文档稀疏 | KTD7 语义已纠偏（能力标志非当前编成，固定 2）；省略字段为官方允许形态 |
| start2 数据源字段形态意外（非整数） | Option 透传宽容；解析失败即 None，不炸 |
| 迁移遗漏旧库路径 | 复用 map_record 迁移模式（挂 `ship::bootstrap`，链路自动）+ 旧库迁移测试场景（U2）；新列禁 NOT NULL DEFAULT |

---

## Deferred Implementation Notes

- `api_onslot_max` 是否也出现在 deck/requireinfo 等其他响应——实现期 grep `api_onslot` 序列化点确认，port 之外按需补
- 扩张后舰的 `api_maxeq`（KcApiShip）保持 manifest 原值不变（`onslot_max` 优先语义，KTD1 动态合成下 `KcApiShip` 本就无该字段冲突）
- ~~`api_m_flag` 推导是否需按海域 `sally_flag` 联动~~ 已解决：KTD7 固定 2（deepening 评审定案），不随海域联动

---

## Origin

- API 变更全景：本仓库会话 2026-08-26 的核查报告（信源 `sinsinpub/kcs2-assets` `api_info/apilist.txt`，6.2.9.1→6.3.4.0 逐版本 diff + emukc 代码逐项 grep 验证）。
- start2 数据现状核查：`.data/codex/start2.json`（stypes 全量无 `api_max_slotplus`，shipgraph 无 `api_sp_flag`，useitem 最大 id=105 格納庫増設）。
