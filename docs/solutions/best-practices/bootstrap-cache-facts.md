---
title: "Bootstrap, cache and upstream data: settled facts"
date: 2026-10-18
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Changing cache make-list, populate, holes or the drift check"
  - "Debugging a download, a cached file or an upstream data gap"
tags: [bootstrap, cache, kache, holes, drift-check, upstream-data]
related_components: [emukc_bootstrap, emukc_cache, emukc_network]
---

# Bootstrap, cache and upstream data: settled facts

Moved verbatim from `PROJECT_MEMORY.md` (Verified Facts) on 2026-10-18 to keep that file under its
size limit. Each entry keeps the date it was established.

- [2026-09-20] 跑完整的 `cache populate` 就是权威的 CDN 存在性探针：它请求每条非 hole 路径，失败清单即
  「上游缺什么」，[2026-09-22] 但它只请求表内路径——`z/cache` 的 336 个
  `btxt_flat` 与 `BTXT_FLAT_IDS` 条数相同，只证明「表内存在」，表外 id 必须单独探测。
- [2026-09-20] hole 表的流向是 Rust → 资产而非 decoder → Rust：`main-decoder/src/path-rules.ts` 从 Rust 源码
  反解 `EVENT_SHIP_HOLES` / `BTXT_FLAT_IDS` / `CHARACTER_HOLES` 写进 `cache_rules.json`。改 hole 要改 Rust
  常量再 `make decode-main`。
- [2026-09-21] `emukc_network` 下载先读完 body 再开目标文件，失败的传输不写半截文件；真正的数据丢失来自
  `bootstrap --force-update` 在下载前就删文件。
- [2026-09-21] kccp 源里 11 条任务缺 name（256/615/616/622/627/628/630/632/633/648/652），游戏内显示
  `n/a`。上游数据缺失，不是解析器问题，补标题需要另一个数据源。

- [2026-09-21] 保序读 JSON 对象不要开 `serde_json/preserve_order`（feature unification 会改全工作区 `Map` 的顺序）；
  `visit_map` 本就按文档顺序交付，用自定义 `Visitor`，见 `parser/kccp/quest.rs` 的 `OrderedEntries`。
- [2026-09-21] `make_list` 的 "holes" 有两义：已删的 holes **report**（`HOLES_COLLECTOR` /
  `holes_report.rs`，无写入方）与活着的 `ShipPathHoles` 跳过表（manifest 规则在用）。按词清理会误删后者。
- [2026-09-21] `.html` 缓存无法内容嗅探：真实 `kcs2/hc.html` 仅 54 字节且与错误页同形，
  所以 008 只给它留非空检查。
- [2026-09-21] 需求解析失败：丢一个 `or` 分支只是更严格，丢一个 `and` 条件则白送；统一 `?` 传播会误杀
  api_no 1019。以上三条的依据见 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 的 006/008/010 段。
- [2026-09-21] `kcs/sound/kcwjcrloeyiyxw/158288.mp3` 在全部镜像上都是 200 + 空 body（4 个 w0* 主机实测）。
  `Kache` 现在把「每个镜像都应答且都不可用」返回成 `InvalidFile` 而非 `FailedOnAllCdn`，populate 归入
  missing 不再重试；注意 `exists_on_remote` 走 HEAD，仍判 `Present`。
- [2026-09-23] drift-check 跟踪 `emukc_bootstrap::assets::REPO_ASSETS` 的全部 15 项（含不可再生的
  `map_ship_drops`），基线 6.3.5.0，入口 `make drift-check` / `make drift-accept`。指纹按规范化后的
  字节算，`_0x` 重命名即使解码知识没变也算漂移。
