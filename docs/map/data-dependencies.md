---
title: "Map data dependencies"
date: 2026-09-22
category: reference
module: emukc_bootstrap
component: map
tags: [map, data-source, provenance, ssot]
---

# 地图数据依赖

按**数据种类**梳理，而不是按文件。每一类都回答三个问题：谁提供、生成器还在不在、断了会怎样。

术语沿用 CONTEXT.md；这里额外用到两个词：**拓扑**指格子与连线，**路由规则**指从一个格子分岔到哪个
格子的判定条件。

## 一览

| 数据种类 | 唯一来源 | 生成器 | 可再生 |
| --- | --- | --- | --- |
| 拓扑（格子、连线、格子类型） | `kc_data` + `stat.json` | 下载 | ✅ 已被 `edges.json` 独立确认 |
| 真实起点抓包 | `assets/real_map_start_data/*.json` | 人工抓包 | ⚠️ 需真实账号 |
| 路由规则 | `assets/wikiwiki_map_catalog.json` | agent skill | ⚠️ 见下 |
| 敌方编成（哪些格子出什么舰队） | 同上，一个文件 | 同上 | ⚠️ 同上 |
| 敌舰属性（HP/火力/装备） | `enemy_ship_extra.json` | 下载 | ✅ |
| 掉落 | `assets/map_ship_drops.json` | **无** | ❌ |
| 地图开放条件 | `build_regular_prerequisites()` | 代码里的公式 | ⚠️ 推断，已对一份真实抓包验证 |

## 1. 拓扑

**kcwikizh/kcdata**（`gh-pages.zip` → `.data/temp/kc_data/_map/<map_id>.json`）是基底。管线以它为
`MapCatalog` 的底座，其余源都是往上贴。它给的是 `routes`（edge id → 起止 label）和 `cells`（格子元数据），
**没有任何分歧条件**——`condition`/`branch` 这类字段一个都没有。

**KagamiChan/kcs2-mapdata 的 `stat.json`** 补格子的 `event_id` / `event_kind`，按 label 键。

**真实抓包**：`assets/real_map_start_data/*.json` 共 36 份，是 `api_req_map/start` 的真实响应，经
`wikiwiki-map build-overlays` 生成 `assets/public_map_catalog_overlays.json`。它是唯一能证明
「官方实际发了什么」的源，7-3 的两阶段变体就靠它。

其中**只有 34 份是有效响应**：`map_7-4.json` 和 `map_7-5.json` 是 `api_result: 100` 的错误页
（抓包用的账号没解锁这两张图，和 `api_get_member/mapinfo` 的 33 条互相印证）。
`source_crosscheck.rs` 已有 `CaptureUnparseable` 分支跳过它们，不是缺陷，但这两张图的起点没有真实凭据。

**`edges.json` 查过了，不接**。`KC3Kai/KC3Kai` 的 `src/data/edges.json` 覆盖 193 张图（含活动图），
键是 edge id、值是 `[起点 label, 终点 label]`；`kcwiki/kancolle-data` 的 `map/edge.json` 是它的镜像，
`build/edge.sh` 只有一行 curl + sed 把 `World 1-1` 改写成 `11`——两边 193 键、0 处取值差异，已实测。

2026-09-22 拿它对我们的常规图做了一次全量交叉校验。它的 edge id 正好等于我们的 `cell_no`，所以
「edge k 的终点 label」必须等于我们 `cell_no == k` 那格的 `node_label`，「起点 label」必须在该格的
前驱里。落到我们 37 张图上共 675 条边：

| 结果 | 条数 | 说明 |
| --- | --- | --- |
| 与我们的拓扑一致 | 674 | |
| 上游有误 | 1 | 7-4 的 edge 3，见下 |

其中 7-3 的 17 条属于 `post_p_unlock` 变体（默认变体是 `pre_p_unlock`，只到 cell 8），5-6 有一条
`["Start 2","Start 2"]` 自指边是它自己的命名产物，两者都在对应变体里对上了。

**唯一的实质分歧是上游错的**：`edges.json` 说 7-4 的 edge 3 是 `A → C`，kcdata 说 `Start → C`。
wikiwiki 是独立的第三方来源，它给 7-4 的 Start 列了 6 条分歧条件（駆逐+海防 ≥3 或 駆逐 ≥2 去 A，
否则去 C），只有 Start 直接分叉到 A/C 才讲得通。所以是 `edges.json` 错，我们是对的。

结论：它不是更好的信源，是一份**独立的校验样本**。接进管线只会把那条错边带进来；它已经完成的工作是
给我们的拓扑背书——674/675 由一条独立数据链确认。

## 2. 路由规则与敌方编成

**这两类共用一个文件，是整条链上最脆的一环。**

`assets/wikiwiki_map_catalog.json` 同时提供路由规则（2144 条，扇出后）和敌方编成
（284 个格子 / 1171 组 → 扇出 437 / 1814）。工作流是：

```
wikiwiki.jp 页面
  → cargo run -- wikiwiki-map sync          （下到 .data/temp/wikiwiki_map/pages/）
  → agent skill emukc-scrape-wikiwiki-mapdata  （读 HTML，出 JSON）
  → cargo run -- wikiwiki-map normalize     （label → cell_no，写资产）
```

中间那一步是 **LLM 而不是解析器**——2026-06 有意为之，替掉了 7389 行正则（计划见
`docs/plans/archive/2026-06-22-007-...`）。

**没有更好的信源。** TsunDB / KCNav 是权威众包库但探测不到公开 API；en.kancollewiki.net 被
Cloudflare 挡住。Fandom 的 `{{MapBranchingTable}}` 按边分键、由 TsunDB 推导、API 开放，但
33 张图 381 条条件句只有 59% 能归入有限句式，其余是 `Otherwise, D`、`Routing unknown`、
跨边引用 `Do not meet the requirements to go to C`——**只在边这一层结构化，条件仍是散文**。
覆盖互有长短：它缺 1-1 / 5-6 / 7-4 / 7-5、7-3 只 3 条边，但 6-x、1-5、1-6、2-5、3-5 比我们厚。
**值得按图人工对照，不值得替换数据链。**

**当前资产的状态**：它是 2026-09-22 修分层之前的混合产物，含 kcdata/stat 的格子元数据和某次活动的
maparea 42 地图（5 张，0 条规则，最终 catalog 会按 manifest 过滤掉）。要干净必须重跑一次 agent
pass，且产出不得劣于已提交版本——本地历史 agent JSON 全是 `Unknown` 谓词，直接拿来重生会大幅倒退。

**5-6 之前完全没被抓过**：49 格、23 个分歧点、0 条规则、0 组敌方编成，每个分岐都退化成随机、
每场战斗都走 `fallback_enemy_fleet`。现在两样都补齐了：

| | 规则 | 敌方编成 |
| --- | --- | --- |
| 资产内（label 空间） | 38 条 | 26 个战斗节点 / 110 组 |
| 扇出到 catalog 后 | 53 条 | 39 格 |

规则里有 6 条 `Unknown`：三处「索敵」wikiwiki 只写了两个字没给阈值（页面自称
「新規実装のため情報不足」），一处是「第二ゲージ破壊前はQ2」这种阶段门，我们没有对应的谓词。

敌方编成的两点取舍：wikiwiki 的舰名带 `(A)` `(空襲)` `(艦載機白)` 这类**图形变体**后缀，指的是同一条
船的不同立绘，资产既有做法是丢掉后缀取最小 id（`PT小鬼群` 四选一取 1637，`飛行場姫` 十八选一取
1556），5-6 沿用；陣形 在页面上是**按节点给一组**而不是按 pattern 给一个，所以按 pattern 轮转分配，
运行时「随机选一个 pattern」正好等于「随机选一个该节点可能的陣形」。8 个非战斗节点
（E 揚陸地点、F/M/S/Y 戦闘なし、I/O 能動分岐、R 港マス）不产生编成，识别方法是它们那一列写的是
旁白而不是舰名，一个 token 都解析不出来。33 个敌舰 id 在 `enemy_ship_extra.json` 里全有属性。

`5-6.html` 之前不在页面缓存里——`sync` 的图单来自 manifest，所以它一直会被拉，只是没人跑过。

**两张图的 boss 格没有编成**，与 5-6 无关、修 5-6 之前就存在：1-6 的 `boss_cell_no` 是 0，也就是
Start（有编成的是 C/F/J/K/L）；3-2 的是 12 = L（有编成的是 A/C/H/J）。`sortie_bosscomp` 靠
`enemy_fleets.contains_key(boss_cell_no)` 判断，所以这两张图的 boss 标志一直是 false。尚未排查。

**「经由某格」的编号空间踩过一次**：资产里的 `VisitedNode` 存的是 wikiwiki 自己的 BFS 编号，而
`auto_derive_label_overlay` 曾把谓词原样透传，于是这些编号进了 kcdata 空间、指向了别的格子——
4-5 的「Dマスを経由」在查 B，5-5 的「Nマス」在查 H，7-4 的「Dマス」在查 C，5 条全错。
现在 `lift_predicate_to_labels` 先把它抬回 label，再由 `resolve_predicate_labels` 落到目标空间。

## 3. 敌舰属性

与「哪个格子出什么舰队」完全分开的一条链：`kcwiki/kancolle-data` 的 `wiki/enemy.json` 与
`wiki/enemyEquipment.json` → `enemy_ship_extra.json`（889 条）。

缺条目时 `sortie/enemy_ship.rs` 有三级降级：`ship_extra` 兜底 → manifest-only → 标注缺失字段的
degraded 兜底，每级都 warn。所以敌舰属性缺失不会中断出击，但会静静降保真度。

## 4. 掉落

`assets/map_ship_drops.json`，242 格 / 10384 条（扇出后 367 / 16499）。

**它没有任何可复现来源。** 产出它的 Rust HTML 解析器在 2026-06 被删；agent skill 产不出（所有历史
agent JSON 的 `ship_drops` 都是 0）；缓存的 wikiwiki 页面里也没有——36 份的 `DROP TABLE` 段全是
难度、作战名、BGM，最长 332 字，一个舰名都没有。

`ca027294` 已经栽过一次：按流程重建资产导致掉落全丢、10 个出击测试挂掉，当时选择把资产冻结。
2026-09-22 把它拆成独立文件，资产因此解锁，但**掉落本身仍然不可再生**。接一个真实掉落源是未解决项。

## 5. 地图开放条件

`build_regular_prerequisites()` 按两条结构规则生成：同区 N-M 需要 N-(M-1)，跨区 (N+1)-1
需要 N-4。上游**没有**对应数据——`api_mst_mapinfo` 只有 `api_level` /
`api_required_defeat_count` / `api_sally_flag`，36 份 wikiwiki 抽取文本 0 份含开放条件。

但它有一份真实样本可对：2026-09-22 抓的 `api_get_member/mapinfo` 发回 33 条，官方只发
「已开放」的图，所以那份列表就是某个通关状态下的开放集合。把同一通关状态喂进这张表，
逐个 id 复现全部 33 条，连 1-6 和 5-6 两条都对上（样本里没有它们，公式也判定未开放）。
覆盖了全部 37 张常规图，但只是通关曲线上的一个点：若某张图真正的门是别的条件、而该条件
在这个样本里恰好也满足，这次比对看不出来。回归测试
`prerequisites_reproduce_the_live_mapinfo_sample` 固定了这份样本。

## 断链影响

| 断了什么 | 后果 |
| --- | --- |
| kcdata | 整个 catalog 没有底座，地图不可用 |
| wikiwiki 资产 | 所有分歧退化成随机、敌方编成消失 |
| `map_ship_drops.json` | 出击不掉船（且**无法找回**） |
| `enemy_ship_extra.json` | 敌舰属性降级，出击仍可进行 |
| `stat.json` / public overlay | 格子类型与真实起点语义退化 |
