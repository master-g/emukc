---
title: "Reconnaissance and Escort-Success Cells - Plan"
type: feat
date: 2026-10-13
status: implemented
execution: code
---

# Reconnaissance and Escort-Success Cells - Plan

Issue #100。#96 留下的三类格子：6-3 的航空偵察（`event_id` 7）、1-6 的船団護衛成功（`event_id` 8）、
5-6 的揚陸地点（`event_id` 9）。

## Evidence

- 客户端（`main.decoded.js`）按 `event_id` 分发：7 → `CellTaskAirReconnaissance`，8 → `CellTaskAnchor`，
  9 → `CellTaskLanding`。
  - 7：读 `api_airsearch` 的 `api_plane_type`（1 大型飛行艇、2 水上偵察機，其余是「稼働偵察機がないため、
    偵察を中止します」）与 `api_result`（0 失敗、1 成功、2 大成功），气泡取 `api_itemget` 的第一项，图标只画
    `api_icon_id` 2、4、7（弾薬、ボーキサイト、開発資材）。
  - 8：读 `api_itemget_eo_comment`（`api_id` 1–4 四种资源、`api_getcount`），自己把血条减一格
    （`defeat_required − defeat_count`）；`api_get_eo_rate` 大于 0 时弹戦果对话框；`api_itemget_eo_result` 的
    `api_usemst` 是 5 时走 `TaskBonusUseItem`。演完只给回母港的按钮。
  - 9：只读海域是否已通关、`gauge_type`、格子编号与舰队装备，`api_eventmap` 只有活动海域才有。它不从
    `next` 的报文里拿任何给予物。
- wikiwiki 中部海域（2026-10-13 取）6-3「航空偵察について」：スコア = Σ 索敵 × ⁴√機数，只算水上偵察機、
  水上爆撃機、大型飛行艇，按当前残機；一枠都没有就不发生。偵察マス定数 G 12、H 16；スコア ≥ 定数 ×
  (16–22 的均匀整数) ÷ 10 是大成功，≥ 定数是成功。报酬从 20 个 ID 的表里抽一项：弾薬 50/60/80（大成功
  100/150/200）、ボーキ 30/40/50（80/100/150）、開発資材 1（2/3/4）；開発資材的期望是成功 0.35、大成功 1。
  报酬在格子上只是预约，ボス B 勝利以上才到手。
- wikiwiki 鎮守府海域 1-6：每到 N 一次血条减 1/7，第 7 次通关；到达给 100 为单位的一种资源，「多いものでは
  1000」，与路线、编成无关；破坏血条的那次固定燃料 1000，另给プレゼント箱 ×1 与戦果 75；通关后「大幅に減少」。
- wikiwiki 南方海域 5-6：揚陸地点只是数 TP 的地方，到达时大破的舰不算（`fleet_transport_points_impl` 已经
  这样做）。它不给东西。
- 地图目录里有三处与来源不符：
  - 1-6 的次数是 4，`api_mst_mapinfo` 是 7。出处是 `DEFAULT_MAP_RECORDS` 写死的那条。
  - 1-6 第 17 格（B → N）的 `event_id` 是 1，6-3 第 12 格（G → H）是 9。两格都是节点的第二条入边，
    `apply_cell_events` 不改 8 与 7，所以它们停在别的来源猜的值上。KCNav 的记录是 8 与 7。
  - 现在 1-6 没有 boss 格，`apply_sortie_map_result` 永远不会动它的血条：1-6 通不了关。

## 取舍

- **规则写在 gameplay 代码里，不进 `map_cell_effects.json`。** 常规图里航空偵察只有 6-3 的两个节点，船団護衛
  成功只有 1-6 一格；等第二张图有这类格子再搬进资产。
- **20 个 ID 的表里只有開発資材占 7 个是来源推得出的**（7/20 × 1 = 0.35；7 个 ID 上的 2、2、3、3、3、3、4
  平均 20/7，7/20 × 20/7 = 1）。弾薬 7 个、ボーキ 6 个、各数量间的分法是本项目定的。
- **偵察报酬在 boss 战结算时当场入账**，不走回港入账：boss 是终点，下一步只能回港，客户端也只在母港刷新
  资材。
- **1-6 的到达奖励**：通关前四种资源等概率、100–1000 按 100 的步长等概率；通关后 10–100 按 10 的步长。
  两处的分布都是本项目定的，来源只有「100単位」「多いものでは1000」「大幅に減少」。资源走回港入账。
- **`plane_type`**：带大型飛行艇时发 1，否则发 2。客户端没有水上爆撃機的图。
- **戦果没有模型**：`api_get_eo_rate` 只发 75 给客户端演出，不落库。月末的时限不做。

## Implementation Units

- **U1 地图数据。** `DEFAULT_MAP_RECORDS` 的 1-6 改 7；`apply_cell_events` 把记录为 7、8 的别名格改过来。
  重建 codex。
- **U2 航空偵察。** `next_sortie` 在 `event_id` 7 的格子上算スコア、定成败、抽报酬，报酬记在出撃状态里；
  boss B 勝利以上时入账。
- **U3 船団護衛成功。** `next_sortie` 在 `event_id` 8 的格子上给资源、推进次数；第 7 次固定燃料 1000、
  プレゼント箱、戦果 75 并通关。报文三个字段。
- **U4 文档。** `docs/solutions/architecture-patterns/` 沉淀，`resource-and-maelstrom-cells.md` 的 Known gaps
  改指过去。

## Risks

- 偵察格最多抽两次随机数（スコア过了定数才抽大成功，成功了才抽报酬），护卫成功格通关前后各抽两次（种类与
  数量；破坏血条的那次不抽）。路过这些格子的带种子序列会移位：换种子，不放宽断言。
- U1 之后要 `cargo run -- bootstrap --codex-only`，否则测试读到旧的地图目录。

## 没有做

戦果；1-6 月末 21:59／23:59 的时限；「ゴール地点到達」类任务的计数；活动海域的这三类格子。
