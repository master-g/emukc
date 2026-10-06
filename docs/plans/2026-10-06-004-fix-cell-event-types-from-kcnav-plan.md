---
title: "Cell Event Types From KCNav - Plan"
type: fix
date: 2026-10-06
status: implemented
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Cell Event Types From KCNav - Plan

## Goal Capsule

codex 里格子的 `event_id` 不是任何真实数据给的：`map_pipeline/kcdata.rs` 用「kcdata 里这格有名字就当战斗格」推出来，
真实起点抓包只带格子颜色、不带事件。KCNav 每张图的 `route` 文档里，每条边带实测的事件 id。
本计划用它订正 `event_id`，消掉不该有的战斗和漏掉的 boss。

## Product Contract

### Problem Frame

`route[edge] = [起点 label, 终点 label, 颜色码, event_id]`。edge id 就是 `cell_no`，label 与 codex 逐格一致
（37 张图、全部变体，0 处不同）。`event_id` 与 codex 不同的格子（2026-10-06，原始响应在 `.data/temp/kcnav/<图>/map.json`）：

| codex | KCNav | 格数 | 后果 | 格子 |
| --- | --- | --- | --- | --- |
| 4 战斗 | 6 気のせい | 28 | **打了一场不存在的战斗**，对手是兜底编成 | 2-2 H、2-5 H / I、3-2 F、3-4 J、4-5 M、5-2 B、5-3 D / E / O、5-5 O、6-2 A / E / G、7-2 D / F、7-3 A / F / G / I |
| 4 战斗 | 5 boss | 4 | boss 战被当成道中战 | 5-6 G / N、7-5 K / Q |
| 5 boss | 2 资源 / 6 | 2 | 非 boss 格被当成 boss | 3-4 O、5-2 N |
| 1 无事件 | 6 | 20 | 无（两者都不触发事件） | 5-6、7-2、7-4、7-5 的若干格 |
| 1 无事件 | 2 资源 | 2 | 少给一次资源 | 7-2 K、7-4 O |
| 1 无事件 | 4 战斗 | 1 | 少一场战斗 | 7-4 G |
| 1 无事件 | 8 / 9 | 2 | 少一个事件 | 1-6 N、5-6 E |
| 4 战斗 | 0 起点 | 1 | 起点格被标成战斗 | 6-5 Start |

按 boss 节点比：3-4 是 P（我们多了 O），5-2 是 O（多了 N），5-6 是 G / N / Z（只有 Z），7-5 是 K / Q / T（只有 T），
7-3 的 `pre_p_unlock` 变体是 E（KCNav 不分变体，另列 P）。

计划 `2026-10-06-001` 实施记录里「31 个战斗格两个来源都没有编成」的原因写错了：不是 label 对不上，是这些格子本来就不是战斗格。

### 已核实的前提

- `route` 的第三列不是 `api_color_no`：気のせい 是 90、能動分岐 是 91，是 KCNav 自己的码。**只取第四列**，颜色不动。
- 真实起点抓包（`public_map_catalog_overlays.json`）里的 `event_id` / `event_kind` 也是由颜色合成的：
  599 格里没有一个 `event_id` 6，空襲和夜战格写成 `(10, 10)` / `(11, 11)`。它不能当事件的凭据。
- codex 的 `event_id` 9 / 10 / 11 是本项目对航空偵察、空襲、夜战的内部编码，KCNav 分别给 7 / 4 / 4。这些不改。

### Key Decisions（待定）

- **KD1 新资产还是并进现有资产。** 倾向新资产 `kcnav_cell_events.json`（图名 → cell_no → event_id），由 `kcnav normalize`
  从已下载的 `map.json` 生成；不需要新请求。
- **KD2 订正在组装的哪一步。** 倾向在公开叠加层之后、路由规则之前：只改 `event_id` 落在 {1, 4, 5, 6} 且与 KCNav 不同的格子，
  `color_no` 与 `event_kind` 按目标事件取既有常量（`BATTLE_CELL`、`BOSS_CELL`、`EMPTY_CELL` 等）。
- **KD3 一张图多个 boss 怎么表示。** `MapVariantDefinition.boss_cell_no` 是单值，`types.rs` 还校验
  「与 boss 同 label 的格子必须是 `event_id` 5」。5-6 和 7-5 各有三个 boss（按血条阶段）。可选：
  (a) `boss_cell_no` 保持为最后阶段的 boss，其余两个只把 `event_id` 改成 5；(b) 改成集合。
  这牵涉血条阶段如何推进，本项目目前没有按阶段切换的机制（计划 003 也因此只取了 5-6 的最后阶段）。倾向 (a)。
- **KD4 7-3 的变体。** KCNav 不分变体；`pre_p_unlock` 的 boss 是 E，P 只在 `post_p_unlock` 存在。按格子是否存在于该变体处理即可。

### Requirements

- R1 上表前三行（34 格）订正后，这些格子的行为与真实游戏一致：気のせい 格不触发战斗，5-6 / 7-5 的中间 boss 按 boss 战处理。
- R2 订正由资产驱动、可再生，不在代码里写死格子。
- R3 KCNav 没有记录的边（`event_id` 为 −1，如 3-4 I、7-4 P）不动。
- R4 订正的格子数量写进组装报告；KCNav 与 codex 的 label 不一致时组装失败（现在是 0 处）。

### Scope Boundaries

- 不改 `color_no` 的来源，不动 9 / 10 / 11 的内部编码。
- 不做血条阶段切换。
- 不处理 `event_kind` 的真实取值（KCNav 的 `route` 没有这一列）。

## Implementation Units

### U1. 事件资产

`kcnav.rs` 的归一化读 `map.json` 时顺带收集 `cell_no → event_id`；`kcnav normalize` 写 `kcnav_cell_events.json`；登记进 `REPO_ASSETS`。
测试：夹具 1-1 得到 `{1: 4, 2: 4, 3: 5}`（起点边不收）。

### U2. 组装时订正

`map_pipeline/assemble.rs` 加一步，按 KD2 改格子；`boss_cell_no` 按 KD3。测试：2-2 H 订正后 `event_id` 为 6 且没有敌方编成需求；
5-6 G 为 5；3-4 O 不再是 boss；未在资产里的格子不变。

### U3. 行为验证

重建 codex 后：`cargo test --workspace`；「战斗格没有编成」的覆盖统计从 31 降到只剩真正缺数据的格子；
`route-rules dist` 不受影响（路由不读 `event_id`）。golden 走 1-1，预期不变。

### U4. 收口

订正计划 001 的那句错误归因（已随本计划提交）、`data-dependencies.md` 一览表的「拓扑」行（格子类型此前并未被独立确认）、
`PROJECT_MEMORY.md`。

## 实施记录（2026-10-06）

U1–U3 已实施。订正后 codex 与 KCNav 在 `event_id` 上只剩有意不改的差异：本项目的内部编码（9 / 10 / 11，共 13 格）和
没有运行时实现的事件（8、9，各 1 格）。「战斗格没有敌方编成」从 31 格降到 0。

定下来的做法：

- KD1：新资产 `kcnav_cell_events.json`（图名 → `cell_no` → label、颜色码、`event_id`），由 `kcnav normalize` 从已下载的
  `map.json` 生成，没有新请求。
- KD2：在组装时最先订正（敌方编成、掉落、路由规则贴上去之前）。只改 `event_id` 在 {1, 4, 5, 6} 且与记录不同的格子，
  目标限于 0 / 2 / 4 / 5 / 6。気のせい 格的 `event_kind` 取 1，能動分岐（KCNav 颜色码 91）取 2；颜色不动。
  起点格也保留原颜色：真实抓包里 6-5 的起点画成 4。
- KD3：取 (a)。`boss_cell_no` 仍是单值；只有它指向的格子被订正成非 boss 时才改指向最后一个 boss 格（3-4 → P，5-2 → O）。
  5-6 的 G / N 与 7-5 的 K / Q 现在是 `event_id` 5 的 boss 战，但不在 `boss_cell_nos()` 里，打赢不推进血条、不算通关——
  血条阶段另立计划。
- 敌方编成资产的 `is_boss` 改由 KCNav 的记录决定，不再读 codex 的格子类型（否则订正前后要生成两遍）。
- label 与记录不一致时组装失败（R4）；现在是 0 处。
- 订正与路由规则一起加载：自带目录的调用方两者都不套用。

## Verification Contract

`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、`cargo test --workspace` 以退出码为准；
连跑两次 `kcnav normalize`，`git diff` 为空。
