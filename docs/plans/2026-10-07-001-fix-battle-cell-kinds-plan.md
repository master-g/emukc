---
title: "Battle Cells Decided by Event Id - Plan"
type: fix
date: 2026-10-07
status: implemented
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Battle Cells Decided by Event Id - Plan

## Goal Capsule

常规图里的夜战开始格、航空戦格、長距離空襲戦格现在都打不了。修到每个战斗格都能用客户端会调用的那个接口打起来。

## Product Contract

### Problem Frame

客户端对一个格子做两次判断，用的是两个不同的字段（`main.decoded.js` 6.3.5.0）：

- **进不进战斗看 `api_event_id`**：`TaskNextSpot._cellEvent` 里 4 是战斗、5 是 boss 战斗；
  6 是気のせい，7 是航空偵察，9 是揚陸地点，10 是泊地修理，都不进战斗。
- **打哪一种看 `api_event_kind`**：`map_info` 的 `isNightStart`（2，以及夜昼戦的 3 / 7）、`isAirBattle`（4）、
  `isVS12`（5 / 7）、`isAirRaid`（6）、`isLongRangeFires`（8），分别对应 `sp_midnight`、`airbattle`、
  `ec_battle`、`ld_airbattle`、`ld_shooting`；其余是普通的 `battle`。

服务端把两件事混成了一件：`sortie/setup.rs` 与 `select_locked_enemy_composition` 用 `event_kind` 判断是不是战斗格，
只放行 1 和 5。后果有两个，都已用探针实测：

1. **22 个战斗格被拒。** 事件 ID 是 4、种类是 2 / 4 / 6 的格子（5-3、5-4、5-5、6-5 的夜战格 11 个；
   1-6、5-2 的航空戦格 3 个；5-2、6-4、6-5 的空襲格 8 个），四个战斗入口全部返回「不是战斗格」。
2. **106 个気のせい格被当成战斗格。** 它们的种类是 1（「敵影を見ず」），守卫放行。客户端不会在这里请求战斗，
   所以没有可见后果，但出击时会给它们锁定一组敌方编成。

另外 codex 里有 12 个格子的事件 ID 不是协议值：7 个是 10（空襲格的另一条入边）、5 个是 11（夜战格的另一条入边），
种类都是 1。客户端收到 10 会演泊地修理，收到 11 会当成道具格，都不会开战。

### Key Decisions

- **KD1 是不是战斗格只看事件 ID**：4 或 5。事件种类不再参与这个判断。
- **KD2 接口与格子种类不做一一对应的强制**，维持现状：只有敌方联合（种类 5）与单舰队接口互斥，这一条已有。
  客户端自己按种类选接口；服务端多拒一种组合只会让现有测试里「在普通格上调航空戦接口」的用法失效，没有收益。
- **KD3 在源头修颜色推断**（实施时改定，原方案见实施记录）：这些值出自
  `map_overlay/merge.rs::infer_event_from_color`，它把真实起点抓包里的格子颜色直接抄成事件 ID。
  战斗颜色改为推出客户端真正会开战的值：7 → (4, 4)、10 → (4, 6)、11 → (4, 2)、13 → (4, 8)，
  然后用 `map build-overlays` 再生 `public_map_catalog_overlays.json`。

### Requirements

- R1 事件 ID 为 4 / 5 的每个格子，用与其种类对应的入口都能发起战斗。
- R2 気のせい格调用战斗入口被拒，出击经过它时不锁定敌方编成。
- R3 组装出的 codex 里没有事件 ID 为 10 或 11 的格子。

### Scope Boundaries

- 不实现夜昼戦（种类 3 / 7）；常规图没有这种格子。
- 不动 6-3 的航空偵察格（事件 ID 7）与揚陸地点格（事件 ID 9）：它们的种类是 1 而协议里应为 0，
  但这两种格子的行为本身没有实现，另案处理。
- 不改各战斗类型的模拟本身。

## Implementation Units

- **U1 颜色推断**：按 KD3 改 `infer_event_from_color`，再生公共叠加层资产，重建 codex。
- **U2 守卫**：`sortie/setup.rs` 与 `sortie/mod.rs::select_locked_enemy_composition` 改为看事件 ID。
- **U3 测试**：遍历 codex 的全部战斗格，各用对应入口打一场；気のせい格被拒；
  夜战开始、航空戦、空襲各导出一个包过客户端规则校验。
- **U4 收口**：文档与 `PROJECT_MEMORY.md`。

## Verification Contract

三道质量门以退出码为准；U3 的遍历测试失败数为 0；codex 里事件 ID 为 10 / 11 的格子数为 0。

## 实施记录（2026-10-07）

U1–U4 完成。

- **原方案走了一步弯路。** 最初把那 12 个格子当成 kcdata 的内部编码，在组装阶段加了一步「按同节点的格子订正」。
  跑全量测试时一个只用公共叠加层的组装测试失败，才看出源头是颜色推断：叠加层里共有 31 个格子带着抄来的事件 ID
  （空襲 15、夜战 15、航空戦 1），其中 19 个在组装时被 `stat.json` 的正确值盖掉，剩下 12 个漏到了 codex。
  已改为在源头修，组装阶段的那一步撤掉。
- **资产差异**：`public_map_catalog_overlays.json` 里这 31 个格子的 `event_id` / `event_kind` 变了；
  另有 33 行新增的 `"gauge_type_e": null`，是模型在上次生成之后多出的字段，与本次无关。指纹基线已更新。
- **结果**：codex 的战斗格是 (4,1) 337、(4,2) 16、(4,4) 3、(4,6) 15、(5,1) 87、(5,5) 2，共 460 个，
  事件 ID 为 10 / 11 的格子为 0。测试 `every_battle_cell_is_playable_through_its_own_entry` 把 460 个格子各打一场，
  每个包都过客户端规则校验；気のせい格调用战斗入口被拒，也不再锁定敌方编成。
- **顺带**：`kcnav sync` 的战斗边筛选从事件 ID 4 / 5 / 7 / 10 收为 4 / 5。下次同步会多请求 5 条夜战入边
  （原先是 11，不在筛选内）；这些节点的编成已由同节点的另一条边覆盖，不影响现有数据。

### 遗留

- 6-3 的航空偵察格 (7,1) 与揚陸地点格 (9,1) 种类仍是 1，协议里应为 0；颜色 6、8、9、14、15 的推断也未核对。
- 各战斗类型的模拟内容（例如空襲戦只有航空阶段）没有与客户端逐项对拍，本次只保证能进入并通过包结构校验。
- 夜戦、航空戦、空襲的包没有像敌联合那样用 `battle-replay` 回放过。
