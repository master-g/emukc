# Bootstrap / Cache 审计实施计划

由 `/improve` 于 2026-09-20 生成，审计基线 commit `82d2203`。
范围：`emukc_bootstrap`（下载、解析、make-list）、`emukc_cache`、`emukc_network`
的下载层，以及 bootstrap → decode-main → make-list → populate 的编排。

每个执行者：完整读完计划再动手，遵守其 STOP 条件，完成后更新下表自己那一行。

## 执行顺序与状态

| Plan | 标题 | 优先级 | 工作量 | 依赖 | 状态 |
|------|------|--------|--------|------|------|
| 001 | 为 emukc_cache 写入/过期/失败路径建立 mock CDN 测试基线 | P1 | M | — | DONE |
| 002 | 为第三方数据解析器建立 fixture 测试基线 | P1 | M | — | TODO |
| 003 | 恢复 HTTP 连接池复用并去掉每文件多余的 HEAD | P0 | S | 001 | IN PROGRESS |
| 004 | 重写 kccp 任务解析器，消除状态机失步 | P0 | S | 002 | TODO |
| 005 | bootstrap web 资产改为先下后替，失败时硬报错 | P0 | S | — | TODO |
| 006 | 禁止空需求被判定为「任务已完成」 | P0 | M | 002 | TODO |
| 007 | 区分「资源不存在」与「瞬时网络失败」 | P1 | M | 001 | DONE |
| 008 | 修正缓存有效性判定：空文件与 .html 不再无条件有效 | P1 | S | 001 | TODO |
| 009 | populate 失败清单落盘，并把 404 从重试路径里分流出去 | P1 | S | 007（仅步骤 4） | TODO |
| 010 | 删除 Greedy / holes-report 死代码并修正文档 | P1 | S | — | TODO |
| 011 | 修复 label_type 年任务表，未命中改为硬错误 | P1 | S | 002 | TODO |
| 012 | 为 cache-list 增加客户端版本校验 | P1 | S | — | TODO |
| 013 | 设计单一权威的客户端版本记录（spike） | P2 | M | 012 | TODO |

状态取值：TODO | IN PROGRESS | DONE | BLOCKED（附一行原因）| REJECTED（附一行理由）

- 007 DONE：`fetch_from_remote` 的 404 分支改返回 `KacheError::FileNotFound`，
  `exists_on_remote` 改为三态 `RemoteExistence{Present,Absent,Indeterminate}`。
  实际调用方是 4 处而非计划正文里的 1 处（`gauge.rs`、`map.rs` ×2、
  `make_list/mod.rs`）；后 3 处原本就用 `?` 传播，改后 `Indeterminate` 继续传播，
  行为不变。真正的缺陷只在 `gauge.rs` 的 `unwrap_or(false)`，现按计划的方案 (a)
  让整个 make-list 失败。回归测试 `variant_crawl_fails_instead_of_truncating_when_no_cdn_answers`
  经变异验证（把 `Indeterminate` 改回 `break` 即失败）。

- 003 IN PROGRESS：两处配置改动已落地，001 的回归基线也已就位——
  `remote_fetch.rs` 的 404 用例通过，且 `fetch_200_writes_body_and_records_version`
  断言「一次 get 只产生一个请求」，把 `skip_header_check(true)` 钉住了（变异验证：
  改回 `false` 后该文件 3 个测试失败）。仍缺完成标准最后一项：步骤 1/步骤 4 的
  改前改后实测耗时。改前的 ~7 files/s 记在 003 正文，改后的数字尚未实测。

## 依赖说明

- 003、007、008 都改 `crates/emukc_cache/src/kache.rs` 或其下载层，而这些路径
  目前**零测试覆盖**。001 先建立 mock CDN 基线，后三者才有回归保护。
- 004、006、011 都改任务数据解析链，且都会改变 `.data/codex/quest.json` 的产物。
  002 先提交 fixture，后三者才能在不跑整轮网络 bootstrap 的前提下验证。
- 013 依赖 012：012 先把「资产里记录客户端版本 + 与实时版本比对」这条最小链路跑通，
  013 再决定要不要把四份版本记录收敛成一份。
- 003 与 009 都动 populate 体验，但改动点不重叠，可并行。
- 009 的步骤 4 依赖 007：007 只提供「404 与瞬时失败是两个错误值」这个能力，把
  populate 侧怎么用它留给了 009。两者之间原本有个缺口——**404 不进 pass 2 的内存
  重试**谁都没写——已在 2026-09-20 补进 009 步骤 4。

## 本次审计确认的关键事实（执行者可直接引用）

- `--greedy` 目前不做任何网络探测，产出与默认策略逐字节相同。`source/mod.rs:92`
  硬编码 `CacheListMakeStrategy::Rules`，`kcs2/mod.rs:22` 与
  `kcs2/resources/mod.rs:35` 各有一行 `let strategy = CacheListMakeStrategy::Manifest;`
  覆写调用方传入的策略。
- `.data/codex/quest.json` 当前含 8 条 `detail` 为字面量 `"_quest_id_NNN"` 的任务，
  12 条 `name` 为 `"n/a"` 的任务，以及 1 条（api_no 1033）`requirements` 为
  `{"And": []}` 的任务。前两者由 004 修复，后者由 006 修复。
- `crates/emukc_bootstrap/assets/resource_manifest.json` 是 9 个 decoder 资产中
  唯一不带 `scriptVersion` 字段的。
- `.sync-fingerprint.json` 记录的是 `6.3.0.0`，而资产已同步到 `6.3.5.0`。

### 2026-09-20 实测：rules 与 manifest 两种策略的清单差异

在 `82d2203` + 计划 003 的两行改动之上实测：

- `cache make-list --overwrite`（Default/Rules 策略）→ **73,050** 条
- `cache make-list --manifest` → **94,558** 条
- **rules 是 manifest 的严格子集**（rules 独有 0 条）；manifest 多出的 21,508 条
  全部落在 `kcs2/resources/ship`（20,327）和 `kcs2/resources/slot`（1,181）

对这两份清单各做随机抽样、跟随 301 重定向后实测 HTTP 状态：

| 抽样来源 | 样本量 | 200 | 404 |
|---|---|---|---|
| rules 清单 | 25 | 25 | 0 |
| manifest 独有部分 | 85 | 6 | 79 |

结论：**默认用 Rules 策略**。manifest 多出的那两万条里约 93% 是不存在的资源，
下载它们纯属浪费——这正是 `cache-manifest-integration.md` 里
「让 fallback 在 decoder 已覆盖的家族上展开会浪费下载、请求不存在的资源」
所描述的情况。

但反过来也有一个**尚未解决的发现**：manifest 独有部分里约 7%（估算 1,500 条
左右）是**真实存在**的，说明 decoder 规则对 ship/slot 变体家族的覆盖仍有缺口，
Rules 清单漏掉了这些资源。抽样中命中的类别包括 `banner_dmg`。
补齐它的正确做法不是复活 Greedy 的暴力枚举，而是把 manifest 差集当作候选集做
一次性存在性探测，把确实存在的并入规则——候选来自差集而非枚举，量级是两万次
探测而不是无边界搜索。这件事尚未立计划。

## 2026-09-20 计划外已落地的改动

以下改动不属于本计划集任何一份，但触及了它们的范围，执行者跑漂移检查时会看到：

- `populate.rs` 删除了逐项 spinner（每个文件一个 `mp.add()`）。indicatif 0.18 只回收
  `ordering` 头部连续的僵尸条，而头部是常驻的聚合条，所以 73k 个 spinner 一个都不会
  被释放，每次重绘都要遍历全量。全本地命中的一轮从 2m13s / 220s CPU 降到 3s / 1.25s。
  009 的基线已相应推进到 `9bd9f59` + 该未提交改动。
- cache 清单从 73,050 收敛到 73,031：`slot.rs` 的 `card_t` 补上了 `generate.rs` 早就
  在用的 `enemy_slot_border`(1500) 过滤，`EVENT_SHIP_HOLES` 补了 6299/6301/6303，
  新增 `ALBUM_STATUS_HOLES`（743/744/745/748/749，补给形态舰，`start2` 里无字段可判）。
  三者去掉的正是 2026-09-20 那轮 populate 全部 19 条 404。

补充一条给计划 010 的事实：`--greedy` 的 holes 报告不只是「产出与默认策略相同」——
`ship.rs` 的 `HOLES_COLLECTOR` 有读取方和清空方但**没有任何写入方**，所以
`holes_report.txt` 恒为空，`GreedyConfig.concurrent` 也没有消费者。
`z/cache/holes_report.txt` 是 2026-04-20 的遗物，不是当前数据。

## 已考虑并否决

- 拆分 `make_list/mod.rs`（1383 行）与 `manifest/generate.rs`（1872 行）：在 010
  决定 Greedy 去留之前拆分是白拆，拆完还要再拆一次。010 落地后可另行评估。
- 给 `fetch_from_remote` 加 per-CDN 熔断 / 健康度跟踪：观测到的失败是代理侧握手
  掉线，不是单个 CDN 主机故障；003 的两行改动直接消除成因。
- 用 `main-decoder` 的解码产物替代 kccp 作为任务名称/描述来源：
  `grep -c '_quest_id_' main-decoder/out/main.decoded.js` 为 0，解码产物里没有任何
  任务字符串，此路不通。
- 更换第三方数据源：数据本身从未缺失（`kccp_quests.json` 里 771 个 id 全在），
  问题在本仓库的解析器。先做 004，再谈换源。
- 让 populate 在「失败项全为 404」时 exit 0：退出码在本仓库没有自动化消费者，而
  「落盘 + 信任人去看」已有反例（空了五个月无人发现的 `holes_report.txt`）。
  完整理由记在 009 的「维护须知」。
