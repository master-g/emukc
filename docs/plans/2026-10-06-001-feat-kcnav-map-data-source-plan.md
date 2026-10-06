---
title: "KCNav Map Data Source - Plan"
type: feat
date: 2026-10-06
status: draft
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# KCNav Map Data Source - Plan

## Goal Capsule

给 `emukc_bootstrap` 加一条确定性的 KCNav 数据链：一个手动触发的下载命令把原始 JSON 落到
`.data/temp/kcnav/`，一个纯函数归一化器把它变成仓库资产。先解决两个没有其他来源的缺口——
掉落表不可再生、敌方等级与编成权重是占位值。路由规则的来源与 wikiwiki 数据链的退役在计划 `2026-10-06-003`。

全程没有 LLM 环节：同一份原始 JSON 归一化两次，产出逐字节相同。

## Product Contract

### Summary

KCNav（`tsunkit.net/nav`）是 TsunDB 众包出击记录的查询前端。它的前端调用的
`/api/routing/...` 是公开 JSON 接口，2026-10-06 实测可用。本计划把其中的掉落与敌方编成接进管线。

### Problem Frame

`docs/map/data-dependencies.md` 列出的断点里，本计划解决这三个：

| 缺口 | 现状 | KCNav 提供 |
| --- | --- | --- |
| 掉落 | `map_ship_drops.json` 没有生成器，丢了找不回；候选舰等概率 | 每条边的掉落舰、掉落次数、总样本数 |
| 敌方等级 | `level * 5 + cell_no`，1-1 给出 7 | 每条敌舰的真实 `lvl`（1-1 全是 1） |
| 编成权重与阵形 | wikiwiki 的 LLM 抽取，权重多为 1，阵形按 pattern 轮转分配 | 每组编成的出现次数和实际阵形 |

该文档里「TsunDB / KCNav 探测不到公开 API」的结论是错的，本计划收口时订正。

### 已核实的前提（不要再查一遍）

样本存在 `z/kcnav_samples/`（忽略目录，不入库）。

- `GET /api/routing/maps/{N-M}` 返回 `route`：`{edge_id: [起点 label, 终点 label, ...]}`。
  edge id 与我们的 `cell_no` 同一空间——`data-dependencies.md` §1 已经用 KC3Kai `edges.json`
  证明 edge id == `cell_no`，KCNav 的 1-1、5-6 与之一致（5-6 也是 49 条）。
- `GET /api/routing/maps/{N-M}/edges/{e}/enemycomps` 返回 `entries[]`：`node`、`formation`、
  `count`、`mainFleet[]`（`id`、`lvl`、`hp`、`equips`）、`escortFleet[]`、`masterId`。
- `GET /api/routing/maps/{N-M}/edges/{e}/drops` 返回 `entries[]`：`id`（-1 = 无掉落）、
  `drops`、`total`、`pct`、`min_s`/`min_a`/`min_b`。**不带查询参数会超时**（25 秒无响应），
  带上前端的整套参数 1 秒内返回；参数默认值在 `/api/routing/maps/all/meta` 的 `paramDefaults`。
- `GET /api/routing/maps/{N-M}/edges/{e1,e2}/los` 返回每条边在 `cn1`..`cn4` 下的
  `{得分: 次数}` 直方图。5-6 的 edge 27 有 5791 个样本，edge 26 只有 38 个。
- `robots.txt` 只点名禁止 GPTBot、ClaudeBot、CCBot、Google-Extended、Bytespider、
  PerplexityBot 六个爬虫 UA，没有 `User-agent: *` 规则，也没有公开的限流约定。
- 仓库已经在用同一作者的数据：`res.rs:89` 从 `planetarian/TsunKitQuests` 拉 `quests.json`。

### Key Decisions

- **KD1 联网下载不进 `bootstrap` 和 `make update`，刷新走独立入口 `make kcnav-update`。**
  归一化后的资产入库并被嵌入，`bootstrap` 组装 codex 时照常消费，不需要 KCNav 在线。理由按分量排：
  KCNav 是持续增长的众包统计，现拉现用会让每次生成的 codex 都不同，掉落测试和 golden 没有稳定基线；
  约 900 次请求、半小时、无 SLA，放进 `bootstrap` 会让它更容易中途断（参见 08-26 的 stale cache-list 事故）；
  `make update` 由客户端发版触发，而掉落与编成不随客户端版本变。
  `make kcnav-update` = `kcnav sync` → `kcnav normalize` → `drift-check`，三步任一失败即停。
- **KD2 礼貌抓取是硬约束。** 并发 1、请求间固定间隔（默认 2 秒，可调大不可调到 0）、
  已存在的原始文件跳过（断点续传）、UA 写明 `emukc-bootstrap/<版本>` 加仓库地址，不伪装浏览器。
  全量约 37 张图 × 每条战斗边 2 个请求，量级 800～900 次，按默认间隔半小时左右跑完一次。
- **KD3 下载与归一化分两步，中间落盘原始 JSON。** 归一化不联网、可重放，测试只喂夹具。
- **KD4 归一化产出按节点 label 建键**，与 `map_ship_drops.json`、wikiwiki 资产同一约定
  （`data-dependencies.md` §2「资产只存 label」）。多条边进同一节点时把样本相加。
- **KD5 KCNav 只覆盖它实测得到的东西。** 路由规则来自羅針盤シミュ源码（计划 `2026-10-06-003`，已落地）；KCNav 的敌方编成
  覆盖 wikiwiki 的编成，但某格在 KCNav 没有数据时保留 wikiwiki 的。这条回退只活到 U5。

### Requirements

- R1 `kcnav sync` 能把指定或全部常规图的原始响应落盘，中断后重跑只补缺的。
- R2 `kcnav normalize` 是纯函数：相同输入目录产出逐字节相同的资产。
- R3 掉落资产带权重（掉落次数），运行时按权重抽取，「无掉落」也按实测比例出现。
- R4 敌舰等级取编成自带的 `lvl`，没有时才退回现公式并 warn。
- R5 编成权重取 `count`，阵形取该编成实测的阵形。
- R6 噪声过滤有明确阈值并写进资产的 `note`：样本过少的编成和掉落被丢弃。
- R7 新资产登记进 `REPO_ASSETS`，受 `drift-check` 跟踪。

### Scope Boundaries

- 只做 37 张常规图。活动海域 KCNav 也有，但本地 codex 没有活动图的其余数据，不在此列。
- 不碰路由规则：它已由计划 `2026-10-06-003` 换成 `map_route_rules.json`。
- 不把 KCNav 的路由统计接成 `probability_pct`——那是独立的一项，另起计划。
- 敌方联合舰队（`escortFleet` 非空）只落盘不消费，等 `ec_*` 端点有计划时再用。
- 不取 `los` 接口：5-6 的索敌阈值已由计划 `2026-10-06-003` 的规则来源给出。

#### Deferred to Follow-Up Work

- 7-3 两阶段、5-6 双血条在 KCNav 里靠 `minGauge`/`maxGauge` 区分，与我们的变体键如何对应
  要在 U1 拿到真实响应后定；定不下来就先只取默认变体，在资产 `note` 里写明。
- 向维护者（planetarian / Chami）打招呼或索要导出：不阻塞本计划，但值得做。

## Implementation Units

### U1. 下载器 `kcnav sync`

- **Files:** `crates/emukc_bootstrap/src/kcnav_download.rs`（新）、`src/bin/cli/kcnav.rs`（新）、
  `src/bin/cli/mod.rs`、`Makefile`（加 `kcnav-sync`；`kcnav-update` 在 U2 有了 normalize 之后补上）
- **Approach:** 照 `wikiwiki_map_download.rs` 的结构写。先取 `maps/all/meta` 得到
  `paramDefaults`，每张图取 `maps/{name}`，再对 `route` 里终点是战斗格的每条边取
  `enemycomps` 与 `drops`。战斗格的判定用 codex 的格子类型，不靠 KCNav 自己的事件码。
  落到 `.data/temp/kcnav/{map}/edge_{e}_{kind}.json`。`--map 1-1` 可限定范围。
- **Patterns to follow:** `emukc_network::download::Request`；`PROJECT_MEMORY.md` 的
  「下载层先读完 body 再开目标文件」保证不写半截文件。
- **Test scenarios:** URL 与查询串的拼装（纯函数）；已存在文件被跳过；
  `{"error": ...}` 形态的 200 响应记为失败而不是落盘（`nodesummary` 就是这种）。
- **Verification:** `cargo test -p emukc_bootstrap kcnav`；手动 `kcnav sync --map 1-1` 一次，
  对照 `z/kcnav_samples/`。

### U2. 归一化器与掉落资产

- **Files:** `crates/emukc_bootstrap/src/parser/kcnav/{mod,types}.rs`（新）、
  `crates/emukc_bootstrap/assets/map_ship_drops.json`（再生）、`assets.rs`
- **Approach:** 解析 `drops` 响应，按 `route[e][1]` 落到节点 label，同节点多边相加。
  每条掉落记 `ship_id` 与 `weight`（掉落次数），另记该节点的 `no_drop` 次数。
  过滤：`drops` 低于阈值的丢弃（阈值在 U2 首跑后按分布定，写进 `note`）。
  `rank` 维度：分别取 S、A、B 三档还是用 `min_s`/`min_a`/`min_b` 推，U1 拿到数据后定。
- **回归保护:** 再生前后做一次差集报告——旧资产有而新资产没有的舰逐条列出，
  每条给出结论（样本不足被滤 / 已下架 / 活动限定）。旧资产是唯一的历史凭据，不能无声丢条目。
- **Test scenarios:** 夹具 `drops_1-1_edge2.json` 归一化后 B 格含 敷波、权重 17373；
  `id == -1` 进 `no_drop` 而不进候选；多边合并相加；输出稳定排序。
- **Makefile:** 加 `kcnav-normalize` 与 `kcnav-update`（串起 sync、normalize、`drift-check`，按 KD1），
  并写进 `make help` 的自文档注释。
- **Verification:** `cargo test -p emukc_bootstrap kcnav`；`make -n kcnav-update` 展开为三步且顺序正确；重建 `.data/codex` 后
  `cargo test --test gameplay_tests` 里的出击掉落测试全绿。

### U3. 按权重掉落（行为变化）

- **Files:** `crates/emukc_model/src/codex/map/types.rs`、
  `crates/emukc_gameplay/src/game/sortie_result.rs`
- **Approach:** `ShipDropDefinition` 加 `weight`，`eligible_sortie_ship_drops` 之后的抽取改成
  加权，并把 `no_drop` 计入分母。现有的掉落率配置如何与实测无掉落率叠加，实现时读
  `sortie_result.rs:439-480` 再定——若两者冲突，以 codex 配置为总开关、实测比例为默认值。
- **Test scenarios:** 固定种子下权重 0 的舰永不掉；权重悬殊时高权重舰占多数；
  旧格式资产（无 `weight`）仍能加载并按等权处理。
- **Verification:** `cargo test -p emukc_gameplay`、`--test gameplay_tests`。
  若 `battle_golden.rs` 的 transcript 变了，有意重新冻结并在提交里说明。
- **Balance 政策:** 若动到 `codex/` 下的 `Default`，按 Balance Defaults Policy 单独提交。

### U4. 敌方编成、阵形与等级

- **Files:** `crates/emukc_bootstrap/assets/kcnav_enemy_fleets.json`（新）、
  `crates/emukc_bootstrap/src/map_pipeline/`、`crates/emukc_model/src/codex/map/types.rs`、
  `crates/emukc_gameplay/src/game/sortie/enemy_ship.rs`
- **Approach:** 归一化 `enemycomps`：按节点 label 聚合，同一组 `ship_ids + formation`
  的多条记录（`masterId` 不同、装备 id 的 500/1500 两套编号）合并，`weight = Σcount`。
  `EnemyComposition` 加 `levels: Vec<i64>`。组装时 KCNav 的格子覆盖 wikiwiki 的（KD5）。
  `build_enemy_encounter` 用编成自带等级，缺失才走旧公式。
- **Test scenarios:** 夹具 1-1 edge 2 归一化出 3 组编成（1501/1502/1503 各两艘），
  权重分别是两条记录之和，等级全 1；KCNav 无数据的格子保留 wikiwiki 编成；
  `tests/gameplay_tests/map/boss_fleet.rs` 仍然成立。
- **Verification:** `cargo test -p emukc_bootstrap`、`cargo test -p emukc_gameplay`、
  `--test gameplay_tests`。敌方等级进了战斗包，golden 必然变，有意重新冻结并说明。

### U5. 敌方编成不再读 wikiwiki 资产

- **Dependencies:** U4 已落地，且覆盖报告显示 37 张图的每个战斗格都有 KCNav 编成。
  有缺口时本单元不做，把缺的格子列进报告。
- **Approach:** 组装时敌方编成只取 KCNav 资产，删掉 `map_pipeline/label_overlay.rs` 里贴 wikiwiki
  `enemy_nodes` 的分支及其夹具；KD5 的回退随之作废。
  `wikiwiki_map_catalog.json` 这个文件本身、agent skill、`wikiwiki-map` 的 sync 与 normalize
  由计划 `2026-10-06-003` 的 U6 统一删除——本单元做完，那个 U6 的前提就齐了。
- **Test scenarios:** `tests/gameplay_tests/map/boss_fleet.rs` 仍成立；重建的 `map_catalog.json` 里
  `routing_rules` 与改动前逐字节相同。
- **Verification:** `cargo test --workspace`；`cargo clippy --workspace --all-targets -- -W warnings`。

### U6. 收口

- 订正 `docs/map/data-dependencies.md`：一览表的掉落行与敌方编成行、§2 里敌方编成的部分改为 KCNav
  （路由规则那部分由计划 `2026-10-06-003` 重写）、§4 整节、断链影响表。
- `docs/solutions/architecture-patterns/map-data-authority.md` 的来源清单同步；`GLOSSARY.md` 里若有
  指向 agent skill 的术语一并更新。
- `docs/solutions/` 记一条：KCNav 接口的查询参数陷阱（不带参数超时、200 里包 error）。
- `CLAUDE.md` 的 Do-Not-Modify 清单已用通配覆盖 `assets/*.json`，只需在命令段加 `make kcnav-update`。
- `make drift-accept` 收新基线；`PROJECT_MEMORY.md` 回写。

## 实施记录（2026-10-06）

U1–U4 已实施并提交；U5、U6 未做。全量同步跑过一次：37 张图、995 个响应，5 个连接错误重跑后补齐。

结果：
- 掉落：`map_ship_drops.json` 由 `kcnav normalize` 再生，11210 条。旧表 10215 条里保留 9724 条，未保留的 491 条逐条列在
  `docs/map/kcnav-drop-diff-2026-10-06.md`（374 条是节点一年样本不足 1000）。
- 敌方编成：新资产 `kcnav_enemy_fleets.json`，1853 组。codex 的 487 个战斗格里 454 个改用实测编成、带真实等级与阵形；
  2 个仍用 wikiwiki（6-5 M，敌方是联合舰队）；31 个两边都没有数据，运行时仍走兜底编成：它们大多本来就不是战斗格（KCNav 记为 気のせい），是 codex 的格子类型错了。
  证据与修法见计划 `2026-10-06-004`。
- `battle_golden.rs` 有意重冻：1-1 第一战的敌舰由 ロ級（1502）变成按实测权重抽到的 ハ級（1503），伤害序列随之变化。

与上文不一致之处，以实现为准：

- 下载与归一化在同一个文件 `crates/emukc_bootstrap/src/kcnav.rs`，没有拆成 `kcnav_download.rs` 与 `parser/kcnav/`。
- 查询串取 `paramDefaults` 里所有非空的标量，去掉 `page` / `perPage`，再补 `start` = `end` 的前一年。
  不带 `start` 时样本多的边（1-1 的 A、C）服务端查询超过 60 秒，返回 504；一年窗口下最重的 1-1 A `drops` 17 秒返回。
  时间窗取一年是用户确认过的（Open Questions 第一条）。
- `rank` 查询参数对 `drops` 无效（带与不带的响应逐条相同）。掉落权重是不分档的合计；`min_s` / `min_a` / `min_b`
  非空视为该档见过掉落，归一化成 `ranks`，运行时只用它缩小候选。
- 「无掉落」是掉落表里 `ship_id` 为 0 的一项，按实测次数参与抽取；此前只要胜利必掉。没有另设掉落率配置。
- 权重缺省（旧格式）按 1 计，不是 U3 测试场景写的「权重 0 永不掉」。
- 不过滤低样本条目（R6 的阈值为 1）：权重已经表达了不确定性，一次观测只占它应有的份额。
- 期间限定掉落继续排除（用户确认）：KCNav 分不出限时与常驻，手工清单 `map_limited_drops.json`（47 条，取自旧表的 `limited` 标记）
  在 `kcnav normalize` 时把对应条目打上 `limited`，运行时照旧不掉。新增的限时掉落要人手加进清单。
  **排除并不完整**：清单只覆盖旧表标过的节点。新表另有约 180 条限时掉落仍按实测比例常驻掉——旧表完全没有的 67 种舰
  （大和、武蔵、Warspite、Johnston、Langley 等，多在 boss 格），以及清单内 11 种舰在清单外节点的条目（如 涼波 在 2-5 O）。
  用户判断不影响游玩体验，不处理；要补的话把清单从按节点列改成按舰列。
- 两份资产由 `kcnav normalize` 借 codex 的地图目录展开到变体与 label，所以它要先有 `.data/codex`。
  舰 id 不在 manifest 里的编成被丢弃（本次为 0 组）。
- 敌方联合舰队的记录（`escortFleet` 非空）归一化时跳过。**这使 U5 的前提不成立**：6-5 M 只有 wikiwiki 有编成，
  `wikiwiki_map_catalog.json` 还不能删，计划 `2026-10-06-003` 的 U6 第 3–5 步因此仍然挂起。
- 请求失败不中断同步：计数、继续，结束时非零即退出码非零，重跑只补缺的。下载层没有请求超时。

## Sequencing

U1 → U2 → U3，这三步合起来就解决掉落；U4 独立于 U2/U3，只依赖 U1；U5 在 U4 之后；U6 最后。
与另两份计划合起来的顺序见计划 `2026-10-06-002` 的 Sequencing。
U3 与 U4 都会动 golden，尽量在同一轮里重冻一次。

## Verification Contract

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、
  `cargo test --workspace` 以退出码为准。
- 归一化的确定性：对同一份 `.data/temp/kcnav/` 跑两次 `kcnav normalize`，`git diff` 为空。
- 测试不联网：`kcnav` 相关测试只读 `crates/emukc_bootstrap/` 下的夹具。

## Open Questions

- 掉落的时间窗：KCNav 的 `start` 默认不限，会混入已经改过的旧掉落表。取最近一两年
  还是全量，U2 首跑后看差集报告再定。
- 是否先向维护者打招呼再跑全量。计划本身不依赖答复。
