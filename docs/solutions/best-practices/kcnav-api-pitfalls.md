---
title: "KCNav routing API: what makes a request fail or mislead"
date: 2026-10-06
category: best-practices
module: emukc_bootstrap
problem_type: best_practice
component: tooling
severity: medium
applies_when:
  - "Changing the query `kcnav sync` sends"
  - "A KCNav sync times out or returns fewer entries than expected"
tags: [kcnav, tsundb, drops, enemy-fleets, download]
---

# KCNav 接口的几个坑

`crates/emukc_bootstrap/src/kcnav.rs` 从 `tsunkit.net/api/routing/...` 取掉落与敌方编成。以下都是 2026-10-06 实测。

- **不带 `start` 时，样本多的边会超时。** 1-1 的 A、C 两条边的 `drops` 与 `enemycomps` 查询超过 60 秒，nginx 返回 504；
  同一张图的 B 边却能返回，所以单看一条边会误以为查询串没问题。限定为 `end` 的前一年后，最重的 1-1 A 17 秒返回
  （78 万样本），三个月 7 秒。`kcnav_query` 因此总是补上 `start`。
- **失败可能是 200。** 有的端点（如 `nodesummary`）用 `200` 加 `{"error": ...}` 表示失败。下载器落盘后会检查响应里有没有
  `result`，没有就删掉文件并计为失败；不这样做，续传会把错误文档当成已下载。
- **`rank` 参数对 `drops` 无效。** 带 `rank=S`、`rank=B` 与不带的响应逐条相同。次数是不分评价档的合计；
  只有 `min_s` / `min_a` / `min_b` 是否为空能说明某档见没见过。
- **`page` / `perPage` 不要传。** `drops` 不带分页时一次返回全部条目，归一化用 `result.count == 条目数` 防截断。
- **同一组敌舰会返回两条**，装备编号分别是 500 起和 1500 起的两套；归一化按「舰 id 序列 + 阵形」合并，次数相加。
- **`无掉落 + Σ各舰掉落 == total`**，可以当校验用。
- **`robots.txt` 点名禁了几个 AI 爬虫的 UA**（含 ClaudeBot）。下载器用自己的 UA、单线程、每次请求后至少等 1 秒；
  全量约 1000 次请求，跑之前先与用户确认。
- 下载层没有请求超时，服务端挂住时靠对方的 504 结束；真挂死就中断重跑，已落盘的会跳过。
