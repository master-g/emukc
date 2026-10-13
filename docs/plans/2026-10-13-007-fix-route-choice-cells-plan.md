---
title: "Route-Choice Cells - Plan"
type: fix
date: 2026-10-13
status: implemented
execution: code
---

# Route-Choice Cells - Plan

Issue #106。能動分岐格（`event_id` 6、`event_kind` 2）上客户端从不让玩家选路：应答里没有
`api_select_route`。

## Evidence

- 客户端（`main.decoded.js`）：`NextModel.getSelectableRoutes` 读 `api_select_route.api_select_cells`，
  `isJunction` 要它有两项以上；`_preNext` 在 `isJunction` 时起 `TaskBranchRoute`（「艦隊の針路を選択できます。」，
  在 `spotLayer.getSpot(no)` 上画可点的圈），把点中的格号作为 `api_cell_id` 发给 `api_req_map/next`。
- `docs/apilist.txt`：`api_select_route` 「次が能動分岐マスのときのみ存在」，`api_cell_id` 「能動分岐時のみ存在」。
- 真实出撃开始：`real_map_start_data/map_6-3.json` 是 `[2, 3]`，`map_4-5.json` 是 `[2, 4]`，都等于目录里
  到达格的 `next_cells`，顺序也一样。
- 目录里常规图的能動分岐格（4-5 A、C、I；5-3 O；5-5 F；5-6 I、O；6-3 A；7-4 F；7-5 F、H、O）都没有
  路由规则，`evaluate_route_destination` 落到 `select_route_from_cells`：带 `selected_cell_id` 就走它，
  不在 `next_cells` 里就报错。请求一侧不用改。
- 离开这种格时的 `api_rashin_flg`：没有抓包。現行规则（`sortie-compass-rashin-flag.md`）的三条证据都
  来自 2-1，两份真实样本只有到达。官方 2015-04-28 的公告把「能動分岐」与「羅針盤分岐」对举
  （x.com/KanColle_STAFF/status/593007287444213760）。客户端只看应答的 `hasCompass()`，自己不抑制。

## 取舍

- **发什么**：到达格是能動分岐且出边多于一条时，发它的 `next_cells`，原样原序；否则不发这个字段。
- **离开能動分岐格时不转罗针盘**（`rashin_flg` 0、`rashin_id` 0）。这是本项目定的：依据是上面的公告与
  「玩家已经选了」，不是抓包。拿到真实的离开应答后以它为准。
- 没带 `api_cell_id` 的请求照旧由服务器随机选一条，不报错。

## Implementation Units

- **U1 报文。** `KcApiMapStart`、`KcApiMapNext` 加 `api_select_route`；`SortieStartResponse`、
  `SortieNextResponse` 加 `select_route`；`start_sortie`、`next_sortie` 填它；离开时的罗针盘例外。
- **U2 场景。** `recon_6_3` 在 A 点选 C，检查开始应答与客户端发回的 `api_cell_id`。
- **U3 文档。** `sortie-compass-rashin-flag.md` 加例外；`reconnaissance-and-escort-cells.md` 与
  `headless-client-check.md` 改指过去。

## Risks

不抽随机数，带种子的序列不移位。

## 没有做

活动海域；非能動分岐格上带 `api_cell_id` 的请求照旧被接受（既有行为）。
