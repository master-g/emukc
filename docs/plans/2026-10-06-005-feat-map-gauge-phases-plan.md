---
title: "Map Gauge Phases - Plan"
type: feat
date: 2026-10-06
status: implemented
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Map Gauge Phases - Plan

## Goal Capsule

多血条的常规图（5-6、7-5、7-2、7-3）按阶段推进：每个阶段有自己的 boss、自己的可走范围，打掉一段血条才进入下一段。
现在只有 7-3 做了两段；5-6 和 7-5 的中间 boss 打赢不算数，路由规则也只用了 5-6 最后阶段的。

## Product Contract

### Problem Frame

| 图 | 实际 | codex 现状 |
| --- | --- | --- |
| 5-6 | 4 个阶段，boss 依次是 G、N、Z（KCNav 记录的 boss 格） | 单变体，boss 只有 Z，`required_defeat_count` 是 280（来源不明，不像击破次数） |
| 7-5 | 3 个阶段，boss K、Q、T | 单变体，boss 只有 T，击破 2 次即通关 |
| 7-2 | 2 个阶段 | 单变体，`gauge_count` 1 |
| 7-3 | 2 个阶段，boss E、P | 已有 `pre_p_unlock` / `post_p_unlock` 两个变体，可作样板 |

计划 `2026-10-06-004` 之后，5-6 的 G / N 与 7-5 的 K / Q 已是 boss 战，但不在 `boss_cell_nos()` 里，
`apply_sortie_map_result` 直接跳过。

### 已核实的前提

- **阶段就是变体，机制已经有了。** `MapVariantDefinition` 有 `clear_to_variant_key` 与自己的 `required_defeat_count`；
  `sortie_result.rs` 的 `apply_sortie_map_result` 在阶段击破数达标后 `assign_stage_id` 切到下一个变体、`gauge_index + 1`；
  出击中途切换阶段时 `sortie/mod.rs` 会重定位当前格。7-3 走的就是这条路。
- **每个阶段能走哪些边，KCNav 的 meta 里有。** `maps/all/meta` 的 `stages` 与 `breakpoints`：5-6 是 4 段、`[18, 20, 36]`；
  7-5 是 3 段、`[14, 22]`；7-3 是 2 段、`[9]`；7-2 是 2 段、`[10]`。edge id 就是 `cell_no`。
  读法已用 7-3 验证：现有 `pre_p_unlock` 变体正好是 `cell_no` 0–8 共 9 格，`post_p_unlock` 是 0–25，
  所以 **breakpoint 是下一阶段的第一条边**，阶段 k 的格子是 `cell_no < breakpoints[k]`。
- **每个阶段的路由规则，羅針盤シミュ源码里有。** 5-6 与 7-3 的分歧函数带 `phase` 参数；7-5、7-2 的没有（各阶段规则相同，只是可走范围不同）。
  转换器已支持 7-3 的 phase → 变体映射（`main-decoder/src/route-rules.ts`）。
- **每个阶段的 boss 格**来自 `kcnav_cell_events.json`（`event_id` 5），按 edge id 落进所属阶段。
- 已下载的 `.data/temp/kcnav/meta.json` 里就有上述字段，不需要新请求。

### 未核实、实施前必须查清的

- 各阶段的血条类型与长度：是击破次数（TP / 击破型）还是 HP 型，每段几次。KCNav 不给。候选来源：wikiwiki 对应页、
  `api_get_member/mapinfo` 真实抓包（`api_eventmap` / `api_defeat_count` / `api_required_defeat_count`）。
  5-6 现有的 280 很可能是某一段的 HP 上限被读成了击破数。
- 月初重置后回到第几阶段、各阶段是否都重置（`MapResetPolicy`）。
- 5-6 的 `stages` 是 4 而 boss 只有 3 个：按 breakpoints 切，第 2 段只多出 18、19 两条边且其中没有 boss 格
  （G 是 11、N 是 27、Z 是 43），像是不靠击破 boss 推进的开路阶段。它靠什么条件进入第 3 段要查清。

### U1 查到的结果（2026-10-07）

来源：wikiwiki.jp/kancolle（南方海域/5-6）、zekamashi.net/kancolle-kouryaku（5-6、7-2、7-5），
与 murasame.blog.jp（7-2、7-3）、kyuku9999.livedoor.blog（7-2、7-3、5-6）互相印证。

| 图 | 阶段 | 推进条件 |
| --- | --- | --- |
| 5-6 | 1（G） | 输送血条 280 TP：G 点 A 胜以上才减，A 胜是 S 胜的七成，需经过揚陸点 E，大破舰不计 |
| 5-6 | 2 | 到达 R 格一次（无 boss），之后出现第二出击点与 N |
| 5-6 | 3（N）/ 4（Z） | 击沉旗舰 2 次 / 3 次 |
| 7-5 | 1（K）/ 2（Q）/ 3（T） | 2 / 3 / 3 次；进入第三段另需 M 格 S 胜一次，与 Q 先后不限 |
| 7-2 | 1（G）/ 2（M） | 3 / 4 次 |
| 7-3 | 1（E）/ 2（P） | 3 / 4 次（此前第二段回落到 3，是既有错） |

- 四张图都在每月 1 日重置并回到第一阶段。5-6、7-5 此前是 `Never`。
- 5-6 的 `stages` 是 4 而 boss 只有 3 个：第 2 段就是「到达 R」的开路阶段。
- `api_mst_mapinfo` 里 5-6 的 `api_required_defeat_count` 280 是 TP 上限；客户端用
  `defeat_required − defeat_count` 画常规图的血条，输送血条也走这两个字段。

**与用户确认的范围（2026-10-07）**：5-6 的真 TP 另开计划，第一段暂用「G 点胜利 3 次」占位；
7-5 的 M 格条件在本计划内做，标记存在 `map_record.event_state`，不加列。

### Key Decisions

- **KD1 阶段用变体表示**，变体键 `phase1`…`phaseN`，沿用 7-3 的机制；不引入「一张图多个 boss 的集合」。
  7-3 现有的 `pre_p_unlock` / `post_p_unlock` 键保留不改名，避免迁移存档里的 `stage_id`。
- **KD2 阶段拓扑由 breakpoints 机械切出**：阶段 k 的格子是 `cell_no < breakpoints[k]` 的那些（末阶段是全部），
  `next_cells` 裁到本阶段内。生成一份资产，不手写。
- **KD3 血条长度进资产并注明来源**；查不到可靠来源的阶段不做，图保持现状并在计划里登记。
- **KD4 掉落与敌方编成两份资产已按「变体里有这个 label 就给」展开**，阶段变体自动得到自己的那部分，不用改。

### Requirements

- R1 5-6、7-5、7-2 各阶段有独立变体，boss 格、可走范围、路由规则与该阶段一致。
- R2 打掉一段血条后进入下一段；最后一段打掉才算通关。
- R3 既有存档（`stage_id` 为空串的 5-6 / 7-5 / 7-2 记录）在读取时落到第一个未完成的阶段，不丢通关状态。
- R4 `route-oracle` 对 5-6 的每个阶段都对拍。

### Scope Boundaries

- 只做常规图。活动图的多阶段、解谜、难度不在此列。
- 不做 5-6 之外的「出击点随阶段变化」等特殊机制，除非 U1 查清它们存在。

## Implementation Units

- **U1 查清血条数据**（上面「未核实」三条）。产出写回本计划；查不清的图划出范围。
- **U2 阶段资产**：`kcnav normalize` 从 `meta.json` 写出 `map_gauge_phases.json`（图 → 阶段 → 最后一条边、boss label），
  血条长度一栏由 U1 的结果填（手工维护部分与生成部分分两个文件，参照 `map_limited_drops.json` 的做法）。
- **U3 组装出阶段变体**：`map_pipeline` 按资产把单变体图切成阶段变体，串好 `clear_to_variant_key`。
  用 7-3 验证：机械切出的两段与现有两个变体的格子集合一致。
- **U4 路由规则按阶段**：`route-rules.ts` 把 5-6 的 phase 映射到阶段变体；无 phase 的图把同一套规则贴到每个阶段，
  指向阶段外格子的规则在该阶段丢弃（需要与「规则贴不上拓扑即失败」的现有约束区分开）。
- **U5 存档迁移与通关语义**（R3），以及 `api_get_member/mapinfo` 里血条字段的输出。
- **U6 对拍与收口**。

## 实施记录（2026-10-07）

- 与计划的出入：资产是两份——手工的 `map_gauge_rules.json`（推进条件）与 `kcnav normalize` 生成的
  `map_gauge_phases.json`（前者加上 KCNav 的断点）；boss label 不进资产，由组装时「该阶段新增的 boss 格」得出。
- phase → 变体的映射在 Rust 侧的 `compass_route_rules.rs`，不在 `route-rules.ts`；`route-rules.ts` 没有改。
- 7-3 两个变体保留原样，只按断点校验并接线；实测机械切分与现有两个变体的格子集合一致（9 格 / 26 格）。
- 新增两种推进方式：到达某格（`advance_on_reach`）与需要某格 S 胜（`advance_needs_s_rank_at`）。
- `route-oracle` 覆盖 5-6 的 phase1、phase3、phase4；phase2（开路阶段）来源没有对应的 phase，不对拍。
- 未做：5-6 的输送血条（占位，G 点 A 胜以上 3 次）。血条按击沉旗舰计数见计划 `2026-10-07-002`。
- 沉淀：`docs/solutions/architecture-patterns/map-gauge-phases.md`。

## Verification Contract

三道质量门以退出码为准；`make route-oracle` 零差异且覆盖 5-6 各阶段；新增一条集成测试走完 7-5 的三段血条。
