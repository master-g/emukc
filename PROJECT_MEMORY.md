# PROJECT_MEMORY.md

Cross-session persistent state. Each section cites its source. This file is an
**index + session state** — authoritative detail lives in `CLAUDE.md`
(architecture / commands / style) and `docs/solutions/` (detailed lessons).

Last updated: 2026-10-15 · branch `main`

## Verified Facts

Architecture — 分层、`Codex`、`entity::user`/`entity::profile` 分域、`svdata=` 前缀等基础见
CLAUDE.md § Architecture；这里只记从中推不出来的：

- [2026-09-18] Gameplay op 是具体类型 `gameplay::Ctx` 上的固有 `async fn`，没有 `XxxOps`/`HasContext`
  trait、没有 `async-trait`；跨领域写走 `_impl`，绝不调第二个固有方法（会开嵌套事务）。
  见 `docs/solutions/architecture-patterns/gameplay-context.md`。
- [2026-07-30] 跨 crate 的战斗入口只有 `emukc_battle::execute_day` / `execute_night`；原始模拟与
  debug overlay 组装是 crate 内部的。见 `docs/solutions/architecture-patterns/battle-crate-docs.md`。

Current verification baseline:

- [2026-07-30] CLI 战斗模拟强制关掉 `god_mode`、`one_hit_kill`，本地 `game_config.json` 不影响种子可重现性
  （`src/bin/cli/battle.rs::load_codex_without_debug_policy`）。
- [2026-09-19] `make update` = `bootstrap --overwrite --force-update` → decode `--sync-assets
  --sync-battle-assets --sync-resource-manifest` → `cache make-list --overwrite`；实测 6.3.2.1、6.3.5.0。
  `make-list` 默认策略会吸收 `cache_rules.json` 的新显式路径，不需要 `--manifest`。
- [2026-09-19] `obfuscator-io-deobfuscator`(ben-sb 1.0.6) 不能替代 `main-decoder`：0/55 battle 字段、
  304 s / 5.1 GB，对比本项目 55/55 / ~20 s。不要再评估。
- [2026-09-19] 两条版本轴：`kcs_const.js` 的 `scriptVesion`（上游拼写错误）是客户端脚本版本，驱动
  `out/version.txt` 与所有同步资产的 `scriptVersion`；`kcs2/version.json` 是各子系统资产版本，独立变动。
  main.js-only 发布只动前者。
- [2026-08-26] 上游怪癖（均已在代码里处理）：kcwiki 空装备槽是 `null` 不是 `false`；`version.json` 把版本
  套在 `resources` 对象里（`parse_version_info` 展平）；webpack 会发 shorthand `ObjectMethod` 工厂
  （`module-graph.ts` 归一）；活动海域 62 默认解锁是设计如此。
- [2026-09-18] `required_exp(99) == required_exp(100) == 1_000_000` 是设计（结婚在同一经验值解锁 Lv.100），
  所以 `exp_to_ship_level(1_000_000)` 得 100，未结婚的调用方靠 `min(cap)` 兜底。不是 bug。
  见 `kc2/level.rs`、`game/ship/exp.rs` 的测试。
- [2026-09-19] `questlist` 的 `api_tab_id` 是客户端标签栏（0,9,1,2,3,4,5 = 全部/进行中/日/周/月/一次性/
  其他），客户端自己不做过滤；`api_label_type` 决定行标签（1,2,3,6,7,101..=112）。
  见 `main.decoded.js` 的 `DutyDataHolder`、`_createTab`。
- [2026-09-19] `SortieStore::with_profile_lock` 的五个持有者（`start_sortie`、`next_sortie`、
  `sortie_battle_result`、`sortie_sp_midnight_battle`、`sortie_battle_impl`）互不嵌套，新入口可安全取用；
  `sortie_midnight_battle` 故意不加锁（改的是 pending session 不是 `active`）。
- [2026-09-22] 真实账号调查（会话随页面存亡、`api_result:100` 两种成因、questlist 按 tab 过滤等）与伤害值 `.1`
  是かばう标记（`isShield`）：`live-api-investigation.md`；脚本 `scripts/fetch_live_api.py`，快照 `z/snapshot/2026-09-22/`。
- [2026-10-09] 基地航空隊（全貌、数值出处、可配属机种与機数、疲劳、基地空袭、「不带航空队的战斗不多抽随机数」）：`docs/solutions/architecture-patterns/air-corps.md`。
- [2026-09-22] `apilist.md` 是唯一端点清单：implemented 是 router 的机械投影（`kcsapi/mod.rs` 的
  `nest` + 子模块 `.route` 双向 diff 重推，不要手工审），missing 是「`docs/apilist.txt` 的 136 减 router」，
  两者不互补。`docs/apilist.txt` 是字段**语义**最全来源，解码客户端才是字段**是否存在**的真源。
- [2026-09-19] 击沉敌舰的任务事件只来自 `settle_sortie_battle_impl` 的 `final_enemy_nowhps`（夜战后的
  session 包，与 `api_dests` 同一份切片）。快照里那份昼战冻结的 `enemy_nowhps` 会吞掉夜战击沉，已删除。
  见 `game/sortie_result.rs`。
- [2026-09-20] SeaORM 只当 struct↔row 映射器 + DDL 生成器用：195 处 `Entity::find()` 里 0 个 join /
  `group_by` / `find_with_related`，`Relation` 只喂 `create_table_from_entity`。换 sqlx/rusqlite 要重写
  ~450 处调用来省 25 个依赖，不值。
- [2026-09-20] 跑完整的 `cache populate` 就是权威的 CDN 存在性探针：它请求每条非 hole 路径，失败清单即
  「上游缺什么」，[2026-09-22] 但它只请求表内路径——`z/cache` 的 336 个
  `btxt_flat` 与 `BTXT_FLAT_IDS` 条数相同，只证明「表内存在」，表外 id 必须单独探测。
- [2026-09-20] hole 表的流向是 Rust → 资产而非 decoder → Rust：`main-decoder/src/path-rules.ts` 从 Rust 源码
  反解 `EVENT_SHIP_HOLES` / `BTXT_FLAT_IDS` / `CHARACTER_HOLES` 写进 `cache_rules.json`。改 hole 要改 Rust
  常量再 `make decode-main`。
- [2026-09-20] `start2` 里没有任何字段能把 5 条补给形态舰（743/744/745/748/749，名字以 補 结尾）与普通
  友军舰区分开——`api_sortno`/`api_backs`/`api_aftershipid`/`ship_picturebook.json` 都查过；友军舰队
  graph id 6299/6301/6303 同理。不要再找规则。
- [2026-09-21] `KC3Kai/kancolle-replay` 的 `js/kcsim.js` 是第二条独立数据链（`COMBINEDCF1-4`
  与 `COMBINEDCONSTS` 复现 wikiwiki 的联合舰队表）。其精度/回避補正对本项目无用。
- [2026-09-22] 地图数据源：开放前置表对真实 `mapinfo` 逐 id 复现；`KC3Kai/edges.json` **不接**（唯一分歧是上游错）；
  `real_map_start_data/` 的 7-4/7-5 是错误页。证据见 `docs/map/data-dependencies.md`。

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
- [2026-09-21] manifest 差集结案，**不要补**：
  `docs/solutions/best-practices/manifest-minus-rules-difference.md`。
- [2026-09-21] `emukc_battle` 里的 `escort` 绝大多数指「旗艦援護/かばう」（旗舰护盾），
  与联合舰队的护卫舰队同名不同义。判断联合舰队相关代码要看 `combined*`，不要 grep `escort`。
- [2026-09-22] 改修配方不需要新数据源，且已对官方验证：codex `slotitem_extra_info.improvement` 的
  `base_consumption` 与官方 `remodel_slotlist` 逐字段相同，官方 `api_req_buildkit`/`api_req_remodelkit`
  就是 codex 的 `dev_mat_min`/`screw_min`（发 min，不发范围）。`secretary` 是**二番舰**不是旗舰，旗舰
  必须明石(182)/明石改(187)；variant 在 ★0–★9 仍是普通改修，只在 ★10 转换。
- [2026-09-22] 官方 `api_start2` 的 `api_mst_mission`（63 条）**没有解锁条件字段**，消耗是比例
  （`api_use_fuel:0.3`）：TODO 的「远征解锁表可靠数据源」不要再去 start2 找。
- [2026-09-21] 联合舰队 friendly 索引有三层空间（模拟连续 / 客户端固定从 6 / 切片本地编号），
  缺一层就把命中记到错误的舰上：`docs/solutions/architecture-patterns/combined-fleet-index-spaces.md`。

- [2026-10-08] 真实 mapinfo：未强化的海域在 `api_air_base_expanded_info` 里没有条目（整備Lv 0 = 不报），航空隊默认名用汉字数字。
- [2026-09-22] 航空隊的客户端硬限制、`airbase_count` 是出撃可能数、`expand_base` 是増开一隊、空槽不落库：
  计划 2026-09-22-001 的前提表与各 U 修正段。
- [2026-09-22] `remodel_slot_recover`：客户端走 `slot.get(id).__updateObject__(api_after_slot)`，
  所以 `api_after_slot` 必须是**同一实例 id**（重置不能删建装备，★10 variant 也不退回原装备）；
  `api_dev_num` 只有 1/2/3，成功率上游不告诉客户端，50/75/100 是本项目定值
  （`codex/remodel_slot.rs::recover_success_rate`）。

- [2026-10-07] 客户端进不进战斗只看 `event_id`（4 / 5），`event_kind` 只选接口（2 夜战、4 航空戦、5 敌联合、6 空襲、8 レーダー）。
  常规图的敌联合只有 6-5 M；`each_*`、`ec_night_to_day` 触发不到。见 `combined-fleet-index-spaces.md` 与计划 2026-10-07-001。
- [2026-10-12] 常规图的开幕夜战格只在 5-3、5-4、5-5、6-5（16 个格子条目），敌编成里没有潜艇：敌潜艇在开幕夜战被击沉触发不到，无头检查也配不出来。
- [2026-09-22] `api_si_list` 只放对应阶段画得出名牌的装备；`btxt_flat` 的存在范围与探测方法见
  `docs/solutions/architecture-patterns/battle-display-name-plates.md`。
- [2026-09-22] 同名 `PhaseHougeki` 昼夜两份编号互斥（昼 2=連撃/7=空母切入，夜 1=連撃/6=空母切入）；
  消歧只能按依赖它的 dispatcher（`PhaseDay*` vs `PhaseNight`/`PhaseAllyAttack`），按 hotspot 深浅挑会静默出错。
- [2026-09-23] 常规图没有联合舰队分歧：37 张图 `sally_flag` 全是 `[x, 0, 0]`，2144 条规则无一提「連合」。
  `FleetRouteContext.escort_ship_entries` 因此暂时没人读；`start_sortie` 也不查 `sally_flag`（计划 1116 Deferred）。
- [2026-10-06] 依赖：`rust-version` 1.96 没压住任何版本，不必抬。`cargo audit` 只剩 fontdue 带入的 `ttf-parser` unmaintained。
  tera 2 的 `render_str` 要传 autoescape=`false` 才等同 tera 1。jemallocator 0.7 只在 BSD 编译，未实测。
  tower-http 0.7 起 `Accept-Encoding: *` 会被压缩（实测选 zstd），`*;q=0` 返回 406。

- [2026-10-06] KCNav 有公开 JSON 接口（掉落带样本数，编成带真实 `lvl`）：计划 2026-10-06-001 前提段与实施记录，样本 `z/kcnav_samples/`。
  `robots.txt` 点名禁 ClaudeBot，对它的请求由用户确认或用户自己执行，agent 不主动发。
- [2026-10-07] 缓存清单：客户端写死的母港 BGM 不在 `api_mst_bgm`；深海 `item_up` 取替换表里 >1500 的目标，其中
  5 个三镜像 404（见 `ABYSSAL_ITEM_UP_HOLES`）；`kcs2/img` 由解码器 `imageLoader`（`UIImageLoader` 链）与手工 `LIST` 取并集。
  服务运行时占着 `z/cache` 的 redb 锁，`make-list` / `populate` 要先停服，或把 `cache_root` 指到别处。
- [2026-10-06] 交战形态概率只有 `kcsim.js:3629` 给数值：45/15/30/10（同航/T有利/反航/T不利）。
- [2026-10-06] 路由规则的来源（羅針盤シミュ `compass_dev` 分支）与转换：`docs/solutions/architecture-patterns/route-rules-from-compass-source.md`。
- [2026-10-07] 装備ボーナス 的数据链、订正层，以及与 KC3Kai 分歧处以客户端为准：`docs/solutions/architecture-patterns/equipment-stat-bonus.md`。

- [2026-10-11] 対空カットイン（53 个种别全有，数值照来源模拟器，与 wiki 的 5 处差异列在文档里）：`docs/solutions/architecture-patterns/anti-air-cut-in.md`。航空队熟练度的成长/损耗/清零与集中派遣恢复机数见 `air-corps.md`「Proficiency」。
- [2026-10-14] 任务 `api_voice_id` = 1000+任务号，领奖时播；名单以 KC3Kai `quotes.json` 的 `npc` 为准（含 1005→2005）。`kc9999/1101` 是客户端替 190 换的，不能按 `npc` 键反推。
- [2026-10-14] 领奖文案 `api_message` 无结构化来源；`extra.rs` 是按任务名搜玩家攻略与 X 帖子的领奖截图转写的（用户说明）：两份来源对照，用 `main.js` 写死的第一行校准，X 原图走 `api.fxtwitter.com`。TsunKitQuests 的 1033 缺出击列表，解析跳过。
  `api_c_list` 不发：改装系任务由服务器判持有。
- [2026-10-12] 任务日以日本时间 05:00 换日；限定日任务 211/212 与 `_at` 入口见 `quest.md` 的 Quest lifecycle，测试陷阱见计划 2026-10-12-002 Risks。
  周常在周一 05:00 重置，只看传入时刻；建档的 `quest::init` 仍读时钟，固定时刻的测试用 `quest_add_at` 造记录。
- [2026-10-12] 夜战攻击种别与谁出手（表、抽签先后、照明弾、`canNB`；客户端种别 3 必须发两件展示装备）：`docs/solutions/architecture-patterns/night-attack-types.md`。
- [2026-10-13] 资源点与渦潮：`resource-and-maelstrom-cells.md`；偵察、護衛成功、揚陸格：`reconnaissance-and-escort-cells.md`。
  能動分岐格离开时不转罗针盘是本项目定的，没有抓包：`sortie-compass-rashin-flag.md`。
- [2026-10-12] 敌方空母的夜战：`carrier-night-air-attack.md` 的 Enemy carriers。取值 3 的轻空母看的是**玩家**的探照灯，#55 原文写反了。

- [2026-10-14] 季常/年常重置月（`label_type` 100+N）与 928 以标签为准的取舍：`quest.md` 的 Quest lifecycle。
- [2026-10-14] kcanotify 的 `total-fp` 是对空合计，27 是护卫空母（`cve_ids`），来源见 #116 #117；改修、Bw1、远征列表的计数规则已入 `quest.md`。客户端把 `mission` 列表里没有的远征置 -1（`main.decoded.js:15821`）。

- [2026-10-15] 陆上设施按来源 `installtype` 表判（`installation_types.rs`，脚本再生）；名字匹配只兜底比来源更新的深海舰（id > 2397）。倍率与上限后倍率见 `anti-installation.md`。
- [2026-10-15] 测试里 `BattleRuntimeShip::from` 造的是 `is_friendly: false`（注释写反了）：要测「玩家侧」规则（空槽、燃料）得用 `BattleRuntimeShip::new(input, true, true)`。
- [2026-10-15] #147 用户定：戦果 = 提督经验（1 万 7 点）+ 任务与 1-6 的加成，按月清零，排名列本服务器所有账号。月末规则（读过的日文攻略页）：戦果月在最后一天 22:00 JST 结束，22:00～23:59 破 EO 只给箱不给戦果，EO ゲージ 1 日 00:00 恢复；见 `war-result-ranking.md`。

## Failed Attempts / Pitfalls

| Pitfall | Source |
| --- | --- |
| populate 基准三坑：`head -200` 清单全是 mp3（带宽受限，收益被掩成 10%，全量实为 49% mp3 + 49% png，要随机抽样）；`Kache::build()` 要求 `cache_root` 已存在，否则 exit 1 且无输出；配置里的相对路径按 config 文件所在目录解析。 | 计划 003 (2026-09-21) |
| `emukc_bootstrap` 的 `make_list` 两个测试打真实 CDN（`make_kache()` 用 `socks5://127.0.0.1:1086`），网络一抖就 `FailedOnAllCdn`，fail-fast 会让整轮 workspace 测试在该 crate 中断。已加 `skip_if_offline`：只吞这一种错；服务器回空文件（`EOF while parsing`）不吞，单独重跑即过（2026-10-14）。 | session 2026-09-21 |
| 联合舰队「deck 1 不开幕对潜/雷击」不能靠切片 attackers——敌方在这些阶段仍打两支 deck，切片会连带砍掉敌方目标池。须按船过滤。 | session 2026-09-21 |
| 全量测试用 `cargo test --workspace --no-fail-fast` 并看退出码：默认在第一个失败的 crate 停，通过数偏少；只 grep `test result:` 会漏掉 FAILED 行。 | plan 004 U5 (2026-06-22)、session 2026-10-11 |
| Upgrading sea-orm does NOT clear the `proc-macro-error2` future-incompat warning: `sea-orm-macros` 2.0.3 still pulls `sea-bae 0.2.1`, same as 1.1.20. Do not cite it as an upgrade reason. | session 2026-09-20, `cargo tree -i proc-macro-error2` |
| 1-1 sorties often dead-end before the boss (routing, not damage RNG). Retry tests restore HP/fuel/ammo via `update_ship` first; quest tests count boss battles (`event_id` 5). | session 2026-08-26, 2026-10-14 |
| `find_ship_impl` does not filter by `profile_id`, and the deduct+mutate template `open_ship_exslot_impl` lacks an ownership check — cross-profile mutation is one copy-paste away. New find-then-mutate ops must compare `profile_id` (see `expand_hangar_slot_impl`). | session 2026-08-26 |
| SeaORM `update()` skips `NotSet` columns, so remodel's rebuild-via-`codex.new_ship` preserves columns `KcApiShip` cannot carry. Derived output-only fields (e.g. `api_onslot_max`) must never be written back into their source increment columns. | session 2026-08-26 |
| [2026-09-18] `net::router::version::test::test_font` writes to `./target/tmp/`, which does not exist when `CARGO_TARGET_DIR` points outside the repo. `mkdir -p target/tmp` once per clone; `cargo clean` or a new machine breaks it again. | session 2026-09-18 |
| [2026-09-19] `tests/gameplay_tests/mod.rs` is a dead file: the compiled entry is `tests/gameplay_tests.rs` with `#[path]` module decls, so a `mod` added only there registers nothing. | session 2026-09-19 |
| [2026-09-19] `sed -i.bak X && cargo test; mv X.bak X` 给假结果：`.bak` 保留原 mtime，cargo 认为没变，复用按**改动后**源码编出的产物。还原后必须 `touch` 再跑。 | session 2026-09-19 |
| [2026-09-19] Missing `main-decoder/node_modules` makes `bun run decode` fail as `Unexpected HTTP` / `Cannot find module '@babel/generator'`, which reads like a corrupt download. `bun install` first; it also unblocks `bun run check`. | git `688e29c` |
| [2026-09-19] Pinning decoder tests to webpack module ids breaks on every upstream build (`DutyModel_` 56360→82131, `PhaseHougeki` 65622→two modules 1830/74885). Match `readableName`, and for duplicate names take the deepest hotspot cleanup. | git `b1016fc` |
| [2026-09-19] A refactor comment saying "keeps feeding X as before" was preserving a bug: plan 002 froze the day-battle `enemy_nowhps` copy, so night-only sinks fired no quest event. Treat "as before" as unverified. | git `HEAD` |
| [2026-09-21] A plan naming one instance of a defect does not bound the fix to it: 007 cited `unwrap_or(false)` in `gauge.rs`; one line down `make_gauge_by_id` mapped every error to `Ok(false)` — same swallow, 3 call sites. Grep the file for the shape, not the cited line. | git `2497efc` |
| [2026-09-20] indicatif: never `mp.add()` a `ProgressBar` per work item (finished bars leak behind the permanent head bar; 73k spinners = 2m13s vs 3s), and any terminal write bypassing `MultiProgress` strands a duplicate bar block in scrollback. | session 2026-09-20, `populate.rs` |

| [2026-09-21] 「无 .data 跑测试」时备份必须放到**仓库外**：cache rules 的测试会 `create_dir_all(".data/tmp")`，`mv .data .data.bak` 再移回会把备份塞进新目录。 | session 2026-09-21 |
| [2026-09-21] `crates/emukc_battle/tests/golden/*.txt` 是 `{:#?}` dump，加字段就全量
失配，哪怕值恒为 `None`。不是模拟漂移：`EMUKC_BLESS_GOLDEN=1` 后确认每份 diff 只有那一行。
`battle_golden.rs` 渲染 transcript，加字段不动它——Stop condition 只针对后者。 | session 2026-09-21 |
| [2026-09-21] 改 `resource-categories.ts` 的 `defaultAbyssal` 是 no-op：`ship_semantic_targets_for_id` 先查 `targetSemantics`，命中就 `continue`，生成分组只是未覆盖 target 的兜底。加了 `banner_dmg` 后 `bun test` 62 pass、decode+sync 成功、清单一条不变。 | session 2026-09-21 |
| [2026-10-14] 产物过期会假报错（`pub` fn 「not found」、读 `../../.data` 报 NotFound 而单跑 `-p` 能过）：多个检出共用 `~/.cache/cargo-build` 时指纹冲突、`CARGO_MANIFEST_DIR` 沿用别的检出。`touch` 该 crate 源文件或 `cargo clean -p`；worktree 设独立 `CARGO_TARGET_DIR`。 | session 2026-09-18、09-23、10-14 |
| [2026-10-14] 调试用 `quest_add_at` 把新行 `progress` 写成 `Completed`：测未接取任务要读条件里的剩余计数，不能看 `progress`。 | session 2026-10-14 |
| [2026-10-15] 照计划「没有做」节建 issue 会漏掉后来已修的（#137、#139 建时已修）：建前先 `git log -S`/读当前代码核对。 | triage 2026-10-15 |

## Last Session

- [2026-10-15] 分支 `feat/war-result-month-end` 按计划 `2026-10-15-006` 做完 #147 的月末截止：戦果月以 22:00 JST 为界、22:00 后破 1-6 不记 75、月度地图改在 1 日 00:00 重置、跨月到达 1-6 终点不发箱。fmt、严格 clippy、`cargo test --workspace` 1531 全过、0 ignored；未做无头核对（规则只靠时钟）。已本地提交，未推送。

## Next Session

- [2026-10-15] 推送 `feat/war-result-month-end`、开 PR（`Closes #147`）等用户点头。#144 剩 #55 遗留、演习夜间触接、侦察/护卫格；瑞雲 CI 第三件之后显示什么、发动后是否算照明弹都没有游戏证据。可选跟进（用户定）：己方探照灯命中 +7、空母 `canShellInstallation`、かばう 不涉及陆上设施。#150、#138、#143、#145 缺实机数据。暂不发版（用户定）；`CHANGELOG.md` 只到 2026-05，发版前重写。
  演习规则见 `practice-rivals.md`；演习测试不固定种子时基础经验可能为 0。叠放 PR 合并前先把上层 PR 改指 `main`。
  远征：判定时机、kcanotify 字段语义与大発加成见 `expedition.md`；旧码典缺 `escort_carriers.json` 会加载失败，`--codex-only` 前先放 `kcanotify_ships_filter.json` 到 `.data/temp`。大発取整口径（fandom 三段 vs wikiwiki 两段）有实测反例再改。
  来源副本不在仓库，按 `kancolle-replay` 提交 `69097abc` 重新取 `js/` 下的文件。`en.kancollewiki.net` 有人机验证，抓不到。
  验证"客户端会不会正确演出"先读 `main.decoded.js` 与 `z/cache` 资源，再跑 `make headless-check`，不找用户开浏览器；
  查清单漏项跑 `make cache-list-oracle`。改了 `parser/` 或 `assets/` 后重建 codex：`cargo run -- bootstrap --codex-only`；
  多血条图改规则后 `kcnav normalize` → 重建 codex → 再 normalize。
  带种子的战斗测试落空时换种子、不放宽断言。数字先读到再写。
