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
| 拓扑（格子、连线、格子类型） | `kc_data` + `stat.json` | 下载 | ✅ |
| 真实起点抓包 | `assets/real_map_start_data/*.json` | 人工抓包 | ⚠️ 需真实账号 |
| 路由规则 | `assets/wikiwiki_map_catalog.json` | agent skill | ⚠️ 见下 |
| 敌方编成（哪些格子出什么舰队） | 同上，一个文件 | 同上 | ⚠️ 同上 |
| 敌舰属性（HP/火力/装备） | `enemy_ship_extra.json` | 下载 | ✅ |
| 掉落 | `assets/map_ship_drops.json` | **无** | ❌ |
| 地图开放条件 | `build_regular_prerequisites()` | 代码里的公式 | ⚠️ 非真实数据 |

## 1. 拓扑

**kcwikizh/kcdata**（`gh-pages.zip` → `.data/temp/kc_data/_map/<map_id>.json`）是基底。管线以它为
`MapCatalog` 的底座，其余源都是往上贴。它给的是 `routes`（edge id → 起止 label）和 `cells`（格子元数据），
**没有任何分歧条件**——`condition`/`branch` 这类字段一个都没有。

**KagamiChan/kcs2-mapdata 的 `stat.json`** 补格子的 `event_id` / `event_kind`，按 label 键。

**真实抓包**：`assets/real_map_start_data/*.json` 共 36 份，是 `api_req_map/start` 的真实响应，经
`wikiwiki-map build-overlays` 生成 `assets/public_map_catalog_overlays.json`。它是唯一能证明
「官方实际发了什么」的源，7-3 的两阶段变体就靠它。

**有更好的没接**：`KC3Kai/KC3Kai` 的 `src/data/edges.json` 覆盖 193 张图（含活动图），
键是 edge id、值是 `[起点 label, 终点 label]`——正对应 API 里 `api_no` 的语义。
`kcwiki/kancolle-data` 的 `map/edge.json` 是它的镜像，生成脚本 `build/edge.sh` 只有一行 curl + sed
把 `World 1-1` 改写成 `11`，内容逐字节一致。镜像更新靠人工触发，比上游晚一天。

## 2. 路由规则与敌方编成

**这两类共用一个文件，是整条链上最脆的一环。**

`assets/wikiwiki_map_catalog.json` 同时提供路由规则（2091 条，扇出后）和敌方编成
（258 个格子 → 扇出 398）。工作流是：

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
maparea 42 地图。要干净必须重跑一次 agent pass，且产出不得劣于已提交版本——本地历史 agent JSON
全是 `Unknown` 谓词，直接拿来重生会大幅倒退。

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

`build_regular_prerequisites()` 按两条结构规则生成，**不是数据**：同区 N-M 需要 N-(M-1)，
跨区 (N+1)-1 需要 N-4。上游无来源——`api_mst_mapinfo` 只有 `api_level` /
`api_required_defeat_count` / `api_sally_flag`，36 份 wikiwiki 抽取文本 0 份含开放条件。
详见该函数的文档注释。

## 断链影响

| 断了什么 | 后果 |
| --- | --- |
| kcdata | 整个 catalog 没有底座，地图不可用 |
| wikiwiki 资产 | 所有分歧退化成随机、敌方编成消失 |
| `map_ship_drops.json` | 出击不掉船（且**无法找回**） |
| `enemy_ship_extra.json` | 敌舰属性降级，出击仍可进行 |
| `stat.json` / public overlay | 格子类型与真实起点语义退化 |
