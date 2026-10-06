---
title: "Route Rules from the Compass Simulator Source - Plan"
type: feat
date: 2026-10-06
status: draft
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Route Rules from the Compass Simulator Source - Plan

## Goal Capsule

把路由规则的来源从「LLM 抽取 wikiwiki 页面」换成「确定性地解析一份 MIT 许可的 TypeScript 源码」。
来源是 X-20A 的羅針盤シミュ。转换器读它每张图一个文件的分歧函数，产出我们的路由规则资产；
同一份源码在 bun 里直接执行，作为转换结果的对拍基准。完成后 wikiwiki 数据链整体退役。

## Product Contract

### Problem Frame

`wikiwiki_map_catalog.json` 的路由规则现在不可再生：产生它的 agent JSON 全是 `Unknown` 谓词，
重跑会倒退（`docs/map/data-dependencies.md` §2）。已知的缺陷都是抽取阶段带进来的——
5-6 有 6 条 `Unknown`、7-5 的 I 格阈值错了一位、1533 条规则里只有 193 条带概率、
所有索敌条件没有分岐点系数。这些在来源里都是对的。

### 已核实的前提（不要再查一遍）

来源（2026-10-06 核对，当时 `compass_dev` 的 HEAD 是 `4f32c40e`）：

- 仓库 `X-20A/X-20A.github.io`，分支 **`compass_dev`**（默认分支 `main` 只有压缩后的部署产物，
  不要去那里找）。根目录有 `LICENSE`：MIT，`Copyright (c) 2025 X-20A`。
- 规则在 `src/core/branch/world{N}/{N-M}.ts`，每张图导出一个 `calc_N_M(node, sim_fleet[, option])`，
  注册表在 `src/core/branch/index.ts` 的 `CALC_TABLE`。常规图 37 张齐全，另有活动图 26 张。
- 返回值是下一格的 label，或 `[{ node, rate }]`。入口用 `case null` 返回起点 `'1'` 或 `'2'`。
- **label 对得上，只有起点例外。** 逐图比对了来源里出现的全部 label 与我们 `map_catalog.json` 的
  `node_label`：37 张图里除起点外零差异（5-6 的 `A1`/`C2`/`K1`/`Q2` 这类也一致）。起点在来源里叫
  `'1'`、`'2'`，在我们这里都叫 `Start`；双起点的图（5-6、6-4、6-5）有两个同名的 `Start` 格
  （5-6 是 cell 0 与 35，6-4 是 0 与 22，6-5 是 0 与 19）。对应规则：`'1'` → `cell_no` 最小的无入边格，
  `'2'` → 另一个。
- **起点选择现在是随机的。** `sortie/mod.rs::select_start_source_cell` 在多个起点格里等概率取一个；
  来源在 `case null` 里按编成决定（5-6 四个条件加阶段，6-4 看揚陸艦、空母、戦艦数与特定舰）。
  旧资产把这些条件挂在 cell 0 的规则上，而从哪个起点出发早在读规则之前就定了。
- 规则出处：常规海域依据日文 wiki，活动海域依据 NGA。所以它与 wikiwiki 同源，
  是同一份知识的人工编码版本；它抓不出 wikiwiki 本身的错。
- 37 个常规图文件共 4965 行，写法规整：838 个 `if`、600 处 `rate`、6 个 `else`、0 个循环。
  运算符只有 `>= === && <= + || < > -` 与一元的 `!`。除解构外的局部变量只有两处（7-3 的 `phase`、7-4 的 `flag`）。
- 用到的输入：舰种与舰种组计数（`BB`…`Ss`）、`ships_length`、`speed`、`seek.c1..c4`（112 处）、
  `route`（5 张图）、`drum_carrier_count`（5 张）、`craft_carrier_count`（4 张）、`SBB_count`（6 张）、
  `radar_carrier_count`（3-2、7-3）、`ship_names`（6-4、7-3）、`arBulge_carrier_count` 与 `fleet_type`（仅 7-3）。
- 用到的辅助函数 13 个：四个航速判断、`includes_base_ship`、`includes_ship_name`、
  `count_ships_by_base_names`、`count_Taiyo_class`、`is_flagship_CL`、`includes`、
  `omission_of_conditions`（抛「条件漏れ」，不可达分支）等。
- 选项：`4-5`、`5-3`、`5-5`、`6-3`、`7-4`、`7-5` 的选项是能動分岐（`return option.A`）；
  `5-6` 与 `7-3` 另有 `phase`。
- 它的 33 式实现在 `src/logic/seek/{equip,equipBonus,fleet}.ts`，含装备ボーナス。

我们这边的求值语义（`map_route.rs::evaluate_route_destination`）：

- 每条规则的 `priority` 是它在该格规则列表里的下标（`label_overlay.rs` 赋值）。
- 命中的非 `Always` 规则按谓词分组，只有优先级最小的那一组生效；组内多条规则按权重随机。
- `Always` 规则只在没有任何条件规则命中时生效。
- 某格没有规则时走拓扑上的后继：客户端给了选择就用选择，否则等概率。

**结论：与来源的「按顺序，第一个命中的 `if` 生效」等价，运行时求值不用改。** 对应关系是：
第 k 个 `if` → 一组谓词相同、`priority = k` 的规则（返回带概率的列表时一个候选一条，`rate` 进
`probability_pct`）；末尾的无条件 `return` → `Always`；嵌套的 `if` → 外层条件 `And` 进每条内层规则；
能動分岐格 → 不出规则。`RouteOperator` 只有 `Eq`/`Gte`/`Lte`，`<` 与 `>` 在整数上换成 `Lte N-1` / `Gte N+1`
即可；索敌得分是小数，配合计划 `2026-10-06-002` 的「比较前向下取整」同样成立。

### Key Decisions

- **KD1 钉住提交。** 仓库里记一个来源 commit SHA；同步命令只取那个提交。升级 = 改 SHA、重跑、
  看 `drift-check` 的差异。来源随时会改，不钉住就没有可复现的资产。
- **KD2 两段式：TS 解析在 `main-decoder`，落地在 Rust。** 解析器（Babel，已是 `main-decoder` 的依赖）
  只做语法到中间 JSON 的翻译，保留来源的词汇（舰种组名、舰名、`c4`、label）。Rust 侧的 normalize
  把词汇解析成 id（舰种组 → `api_stype` 列表、基础舰名 → 该舰全部改造形态的 id、装备计数 → 装备类别），
  因为 manifest 在 Rust 侧。
- **KD3 白名单语法，遇到不认识的就失败。** 解析器只接受前提里列出的构造；别的一律报
  `文件:行号` 并退出非零。不产出 `Unknown`、不猜。来源升级后出现新写法时，这是唯一安全的行为。
- **KD4 来源代码不入库，资产入库。** 源码下载到 `.data/temp/x20a_compass/`；入库的是转换出的
  `map_route_rules.json`，其 `note` 写明来源仓库、分支、SHA 与 MIT 版权声明。另在仓库的第三方声明里
  登记一条（没有该文件就新建 `THIRD_PARTY_NOTICES.md`）。
- **KD5 对拍是转换的验收，不是质量门。** 它执行第三方代码、依赖下载的源码，所以不进 `cargo test`；
  但每次升级 SHA 都必须跑，结果写进提交说明。
- **KD6 资产独立成文件，不改造 `wikiwiki_map_catalog.json`。** 新资产只有路由规则，按 label 建键。
  旧资产在敌方编成迁到 KCNav（计划 `2026-10-06-001` 的 U4）之前继续提供 `enemy_nodes`，之后整份删除。
- **KD7 起点选择单列一组规则。** `case null` 的规则不属于任何格，进变体的新字段 `start_rules`，
  目的格是起点格本身。`select_start_source_cell` 在有 `start_rules` 时按与分歧相同的求值方式选起点，
  没有时保持现状。这是本计划唯一的运行时行为变化，只影响 5-6、6-4、6-5 三张双起点图。
- **KD8 只做常规图。** 活动图的规则来源里也有，但本地 codex 没有活动图的其余数据。

### Requirements

- R1 一条命令取到钉住的来源提交；重复执行不重复下载。
- R2 解析器对 37 张常规图全部成功，无一条跳过；对白名单外的语法失败并定位。
- R3 normalize 是纯函数，同一份中间 JSON 产出逐字节相同的资产。
- R4 资产里没有 `Unknown` 谓词，每条 LoS 谓词都带分岐点系数。
- R5 对拍在枚举出的舰队上，转换后的规则与来源代码给出相同的目的格集合与概率。
- R6 切换来源后与旧资产的差异有报告，报告按图列出新旧走法不同的格子。

### Acceptance Examples

- AE1 2-5 的 G 格：得分（系数 1）36 去 K，38 在 K / L 间各 50%，41 去 L。
- AE2 5-6 的 E 格：无戦艦級且得分（系数 4）58 去 G，57 去 F；有戦艦級时门槛是 65。
- AE3 1-1 的 A 格：6 艘时 B 45% / C 55%，1 艘时 B 20% / C 80%。
- AE4 7-5 的 I 格：得分 58 落在随机带里（旧资产在这里一条规则都不匹配）。
- AE5 把某个来源文件里的一个 `if` 改成 `while`，解析器失败并报出该文件与行号。
- AE6 6-4 带揚陸艦的舰队从起点 2 出发；现在是两个起点各 50%。

### Scope Boundaries

- 不改运行时的求值顺序与权重计算（已证明等价）；起点选择按 KD7 改。
- 不把来源的 33 式实现搬进来——那是计划 `2026-10-06-002` 的事，本计划只产出带系数的谓词。
- 不做资源格的获得量表（来源里有，`2-5`、`5-4` 等），另起计划。
- 不做活动图，不做联合舰队分歧（`fleet_type` 只在 7-3 出现，见 U3 的处置）。

#### Deferred to Follow-Up Work

- 来源标成红字的「暂定值」和作者自定的「それっぽい値」（「n 寄りランダム」没写百分比时）在代码里
  与实测值无法区分。资产照搬，`note` 里写明这一点；将来可用 KCNav 的走法统计校准。
- `start_sortie` 不查 `sally_flag`（1116 Deferred）不受本计划影响。

## Implementation Units

### U1. 取源命令与 SHA 钉住

- **Files:** `crates/emukc_bootstrap/src/compass_source.rs`（新）、`src/bin/cli/`（`route-rules sync`）、`Makefile`
- **Approach:** 常量 `COMPASS_SOURCE_COMMIT`。按该 SHA 下载仓库归档并解出 `src/` 与 `LICENSE` 到
  `.data/temp/x20a_compass/<sha>/`；目录已存在即跳过。不用 `git clone`，避免把无关分支带下来。
- **Test scenarios:** 归档 URL 拼装；目录存在时不发请求；解出的 `LICENSE` 首行不是 `MIT License` 时报错
  （许可变了要让人知道）。
- **Verification:** `cargo test -p emukc_bootstrap compass_source`；手动跑一次，`src/core/branch/world7/7-5.ts` 在位。

### U2. TS → 中间 JSON 解析器

- **Dependencies:** U1
- **Files:** `main-decoder/src/route-rules/`（新）、`main-decoder/test/route-rules/`、`main-decoder/src/cli.ts`
- **Approach:** Babel 带 TypeScript 插件解析每个 `calc_N_M`。对 `switch(node)` 的每个 `case`，
  把语句序列折成有序的 `{ cond, targets }` 列表：
  1. 条件表达式翻成中性的树：`and` / `or` / `not` / `cmp(lhs, op, 整数)` / `call(名字, 参数)`。
     `lhs` 是输入字段或它们的和（`CL + CLT`）；`seek.cN` 记成 `{ los: N }`。
  2. `return 'X'` → 单目标；`return [{node, rate}, …]` → 多目标带概率；`return option.K` → 标记能動分岐。
     `case null` 单独输出为起点规则；恒返回 `'1'` 的（34 张单起点图）不产出任何东西。
  3. 嵌套 `if` 把外层条件带进内层；内层没有兜底 `return` 时按 JS 的贯穿语义继续到外层的下一条。
  4. `phase` 这类选项：对来源里出现过的每个取值做一次常量折叠，各出一份规则表（见 U3 的变体映射）。
  5. 两处局部变量（7-3 的 `phase`、7-4 的 `flag`）按「单次赋值、内联展开」处理；再出现别的写法就按 KD3 失败。
  舰种组（`BBs`、`CVs`、`Ds`、`Ss`、`CAs`、`CLE`、`BBCVs`、`CVH` …）的成员定义从来源的
  `src/models/fleet/` 里解析出来写进中间 JSON 的头部，不在我们这边手抄一份。
- **Execution note:** 先写白名单与失败路径的测试，再写翻译。
- **Test scenarios:** 用**手写的**最小 TS 片段做夹具（不是来源文件）：单 `if` 单目标、带概率列表、
  末尾无条件 `return`、嵌套 `if` 的贯穿、`a + b >= n`、`<` 与 `===`、`seek.c4`、`route.includes('A')`、
  `return option.A`、`phase` 折叠、`omission_of_conditions` 不产出规则；AE5 的失败路径；
  `else`、三元、循环各报错一次。
- **Verification:** `cd main-decoder && bun test && bun run check`；对 U1 取到的 37 个文件全部解析成功，
  统计与前提一致（838 个条件、600 处概率）。

### U3. 中间 JSON → 资产（normalize）与模型补齐

- **Dependencies:** U2
- **Files:** `crates/emukc_gameplay/src/game/sortie/mod.rs`、`crates/emukc_bootstrap/src/parser/compass_route/`（新）、
  `crates/emukc_bootstrap/assets/map_route_rules.json`（生成）、`assets.rs`、
  `crates/emukc_model/src/codex/map/types.rs`、`crates/emukc_gameplay/src/game/map_route.rs`
- **Approach:**
  1. 词汇解析：先读来源 `src/models/fleet/AdoptFleet.ts` 里 composition 的定义再定映射——
     纯按舰种定义的组 → `ShipTypeCount`；若有按舰 id 增减成员的组 → `ShipSetCount`。不要凭组名假定。
     其余：基础舰名 → 该舰全部改造形态的 id（走 manifest 的改造链）；
     航速辅助函数 → `Speed` / `Not(Speed)`；`SBB_count` → `ShipSetSpeedCount`；`is_flagship_CL` →
     `FlagshipShipType`；`route.includes` → `VisitedNodeLabel`；`drum_carrier_count` → `DrumCanisterCount`；
     `craft_carrier_count`、`radar_carrier_count` → `EquipmentCount`（装备类别按来源里的定义取）。
  2. `seek.cN` → `LoS { coefficient: Some(N), … }`。给 `RoutePredicate::LoS` 加 `coefficient: Option<i64>`
     （`serde(default)`）。求值如何用它由计划 `2026-10-06-002` 定义。
  3. 模型里没有对应物的只有两项，都只在 7-3 出现：`arBulge_carrier_count`（按装备 id 计数，
     现有 `EquipmentCount` 只按类别）与 `fleet_type`。前者给 `EquipmentCount` 加一个可选的
     `slotitem_ids`；后者在常规图恒为通常艦隊，normalize 时按常量折叠掉，并断言折叠后该分支不可达。
  4. 概率 `rate` → `probability_pct`；`priority` 由组装时的下标给出，normalize 只保证顺序。
  5. 变体映射：一张代码内的小表把 `(图, phase)` 对到我们的变体键（7-3 的 `pre_p_unlock` /
     `post_p_unlock`，5-6 的各阶段）。对不上的 phase 报错。
  6. 能動分岐格不出规则；normalize 校验这些格在拓扑上确实有多个后继。
  7. label 落地：除起点外原样使用；`'1'` / `'2'` 按前提里的规则对到起点格。起点规则写进
     `start_rules`（`MapVariantDefinition` 的新字段，`serde(default)`），`select_start_source_cell` 按 KD7 读它。
     一条多目标 `return` 展开出的各条规则必须带**同一个**谓词对象——运行时靠谓词键相同把它们并成一组，
     键不同就会各自按优先级竞争，随机带就没了。
- **Test scenarios:** 每种词汇解析一条；未知舰名、未知舰种组、未知辅助函数各报错；
  AE1–AE4 对应的资产片段；两目标的 `return` 产出两条 `route_predicate_key` 相同的规则；
  5-6 的 `start_rules` 非空且目的格是 cell 0 与 35，单起点图的 `start_rules` 为空；
  固定种子下 5-6 满足「海防 2 以上」的舰队恒从起点 1 出发；资产里 `Unknown` 计数为 0、LoS 谓词无 `coefficient: None`（R4）；
  输出稳定排序（R3）。
- **Verification:** `cargo test -p emukc_bootstrap compass_route`；`cargo test -p emukc_model`。

### U4. 接入组装并出差异报告

- **Dependencies:** U3
- **Files:** `crates/emukc_bootstrap/src/map_pipeline/{sources,assemble,label_overlay}.rs`、
  `crates/emukc_bootstrap/src/map_route_rules.rs`
- **Approach:** 组装时路由规则改从 `map_route_rules.json` 取，`wikiwiki_map_catalog.json` 的
  `routing_rules` 不再读（文件暂留，仍提供 `enemy_nodes`）。结构校验 `map_route_rules.rs` 原样套在新规则上。
  加一个一次性的报告命令：对同一批枚举舰队，分别用旧规则与新规则求目的格分布，按图列出不同的格子。
- **Test scenarios:** 组装后的 2-5、5-6、7-3 两个变体都有规则；规则的目的格都在拓扑的后继里
  （现有校验）；`tests/gameplay_tests/map/boss_fleet.rs` 不受影响。
- **Verification:** `cargo test -p emukc_bootstrap`；重建 `.data/codex`；差异报告存进提交说明或 PR 描述。
  报告里每一类差异要有一句归因（旧的抽取错 / 旧的没有概率 / 来源更新过）；归不了因的逐条看。

### U5. 对拍

- **Dependencies:** U4，以及计划 `2026-10-06-002`（索敌得分算对之前，14 张图上的对拍全是噪声）
- **Files:** `main-decoder/src/route-oracle/`（新）、`src/bin/cli/`（`map route-dist`）、
  `crates/emukc_gameplay/src/game/map_route.rs`、`Makefile`（`route-oracle`）
- **Approach:**
  1. 我们这边：把 `evaluate_route_destination` 里「算权重」与「掷骰」拆成两个函数（行为不变），
     `map route-dist` 读一份 `(图, 变体, 当前格, 舰队描述, 走过的格子)` 列表，输出按 label 归一化的分布。
     舰队描述直接给各项计数与四个系数下的索敌得分，不经过舰船实体——对拍验证的是规则，不是索敌公式。
  2. 来源那边：bun 直接 `import` U1 目录里的 `src/core/branch/index.ts`，用同一份舰队描述构造它的输入对象。
     给输入套 `Proxy`，读到未提供的字段就抛错并报字段名（它的函数解构缺失字段得到 `undefined`，比较会静默为假）。
  3. 舰队枚举是确定性的：对每个分岐点，取该格规则里每个阈值的两侧（N−1 / N / N+1），做组合，
     再加一批固定的常见编成。
  4. 比较：目的格集合不同记为规则差异；集合相同、概率差超过 0.01 记为概率差异。
- **Test scenarios:** `map route-dist` 对 AE1–AE4；拆分前后固定种子下的抽样结果逐条相同；`Proxy` 报出缺失字段名。
- **Verification:** `make route-oracle` 对 37 张图跑完，差异数为 0。不为 0 时每条差异要么修转换器，
  要么登记为已知差异并写明原因（预期只有 U3 第 3 步折叠掉的 `fleet_type` 分支）。

### U6. wikiwiki 数据链退役

- **Dependencies:** U5 通过；计划 `2026-10-06-001` 的 U4 已落地，且其覆盖报告显示每个战斗格都有 KCNav 编成。
  后一条不满足时只做第 1、2 步。
- **Approach:**
  1. 删 agent skill：`.claude/skills/emukc-scrape-wikiwiki-mapdata/` 与内容相同的 `.agents/skills/` 副本。
  2. 删 `wikiwiki-map normalize`、`parser/wikiwiki_map/` 里 agent JSON → 资产的转换与 `lift_predicate_to_labels`。
  3. 删 `wikiwiki_map_catalog.json`、`wikiwiki_map_asset.rs`、`wikiwiki_map_download.rs`、`wikiwiki-map sync`、
     `label_overlay.rs` 里贴 wikiwiki 编成的分支及夹具；`EnemyComposition.raw_ship_names` 没有写入方后一并删。
  4. `wikiwiki-map build-overlays` 处理的是真实抓包，与 wikiwiki 无关：保留功能，把子命令挪到
     `map build-overlays`，旧名字不留别名。
  5. `REPO_ASSETS` 去掉 `WIKIWIKI_MAP_CATALOG`、加上新资产；`make drift-accept`。
- **Test scenarios:** 仓库内 grep `wikiwiki_map`、`emukc-scrape-wikiwiki-mapdata`、`enemy_nodes` 无命中
  （`docs/plans/`、`docs/solutions/` 的历史叙述除外）；重建的 codex 里路由规则与 U4 之后逐字节相同。
- **Verification:** `cargo test --workspace`；`cargo clippy --workspace --all-targets -- -W warnings`。

### U7. 收口

- 重写 `docs/map/data-dependencies.md` 的一览表与 §2；更新
  `docs/solutions/architecture-patterns/map-data-authority.md` 的来源清单；`GLOSSARY.md` 里指向 agent skill 的条目。
- `docs/solutions/` 新增一条：为什么白名单失败优于产出 `Unknown`，以及升级来源 SHA 的步骤。
- `CLAUDE.md`：命令段加 `make route-rules-update`（sync → 解析 → normalize → drift-check）与 `make route-oracle`；
  删掉 skill 列表里的 wikiwiki 抓取项。
- `PROJECT_MEMORY.md` 回写。

## Sequencing

U1 → U2 → U3 → U4 是主线。U5 等计划 002；U6 等 U5 与计划 001 的 U4。
与另两份计划合起来的顺序见计划 002 的 Sequencing。

## Verification Contract

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、`cargo test --workspace`、
  `cd main-decoder && bun run check && bun test`，以退出码为准。
- 确定性：对同一个来源 SHA 连跑两次 `make route-rules-update`，`git diff` 为空。
- `battle_golden.rs` 走 1-1；1-1 的 A 格概率会从等概率变成按艦数的 45/55 等，transcript 若因此改变，
  有意重新冻结并在提交里说明。

## Definition of Done

- 路由规则由一条命令从钉住的来源提交再生，资产里没有 `Unknown`。
- 对拍在 37 张图上无未登记的差异。
- 仓库里不再有 wikiwiki 抓取的 skill、解析器与资产。
