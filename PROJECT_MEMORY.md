# PROJECT_MEMORY.md

Cross-session persistent state. Each section cites its source. This file is an
**index + session state** — authoritative detail lives in `CLAUDE.md`
(architecture / commands / style) and `docs/solutions/` (detailed lessons).

Last updated: 2026-09-22 · branch `main`

## Verified Facts

Architecture — 分层、`Codex`、`entity::user`/`entity::profile` 分域、`svdata=` 前缀等基础见
CLAUDE.md § Architecture；这里只记从中推不出来的：

- [2026-09-18] Gameplay op 是具体类型 `gameplay::Ctx` 上的固有 `async fn`，没有 `XxxOps`/`HasContext`
  trait、没有 `async-trait`；跨领域写走 `_impl`，绝不调第二个固有方法（会开嵌套事务）。
  见 `docs/solutions/architecture-patterns/gameplay-context.md`。
- [2026-07-30] 跨 crate 的战斗入口只有 `emukc_battle::execute_day` / `execute_night`；原始模拟与
  debug overlay 组装是 crate 内部的。见 `docs/solutions/architecture-patterns/battle-crate-docs.md`。

Current verification baseline:

- [2026-07-30] CLI 战斗模拟测试加载真实 Codex 但强制 `god_mode=false`、`one_hit_kill=false`，所以本地
  `.data/codex/game_config.json` 影响不到种子搜索的可重现性。见
  `src/bin/cli/battle.rs::load_codex_without_debug_policy`。
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
- [2026-09-19] `update_quest_progress_for_action` 只读任务进度行与 codex（`quest/update.rs:174-244`），
  所以在领域事务里的任何位置都能跑；U8 把它移到 `commit` 前，行为零变化。
- [2026-09-19] `questlist` 的 `api_tab_id` 是客户端标签栏（0,9,1,2,3,4,5 = 全部/进行中/日/周/月/一次性/
  其他），客户端自己不做过滤；`api_label_type` 决定行标签（1,2,3,6,7,101..=112）。
  见 `main.decoded.js` 的 `DutyDataHolder`、`_createTab`。
- [2026-09-19] `SortieStore::with_profile_lock` 只有五个持有者（`start_sortie`、`next_sortie`、
  `sortie_battle_result`、`sortie_sp_midnight_battle`、`sortie_battle_impl`），互不嵌套，新入口可安全取用。
  `sortie_midnight_battle` 故意不加锁——它改的是 pending session 不是 `active`。
- [2026-09-22] `api_port/port` 只带 `api_token` 会回 `api_result:100`（伪装成「请重新登录」）：客户端还发
  `api_sort_key=5`、`spi_sort_order=2`（上游 typo）与 `api_port=_createKey(member_id)`。算法与
  `PORT_API_SEED` 已移植到 `emukc_crypto::PortApiKey`，对拍客户端 5 组向量并**已用真实服务器验证通过**。
  会话本身是游戏页面一关就失效。
- [2026-09-22] 基地航空隊配属消耗实测：一式陸攻(169) 空槽配满 18 機扣 216 ボーキ = **每機 12**
  （`set_plane` 前后 material 差值）。是否随机种变化未测。撤下→`api_state:2`→等待→回到 0/0，两阶段。
  `api_plane_info` 是 optional：没有配置転換中的装备时 port 整个字段缺省。
- [2026-09-22] 撤下中隊回 `api_state:2` 并**保留** `api_slotid`、半径不变；port 的
  `api_plane_info.api_base_convert_slot` 列出这些装备，客户端把它们额外塞回装备选择列表所以能直接再
  配属。我们实现的 0/0 即时清空是错的。`api_get_member/base_air_corps` 官方已 404。
- [2026-09-22] `apilist.md` 是唯一端点清单：implemented 是 router 的机械投影（抽 `kcsapi/mod.rs` 的
  `nest("/prefix", ..)` 加子模块 `.route("/leaf"` 双向 diff 重推，不要手工审），missing 是
  「`docs/apilist.txt` 的 136 减 router」；两者不互补——`remodel_slot_recover` 这类不在上游参考里的
  端点只进前者。`docs/apilist.txt`（4209 行）是字段**语义**的最全来源，解码客户端才是字段**是否存在**
  的真源；`docs/api_coverage.md` 只是路线图。
- [2026-09-19] 击沉敌舰的任务事件只来自 `settle_sortie_battle_impl` 的 `final_enemy_nowhps`（夜战后的
  session 包，与 `api_dests` 同一份切片）。快照里那份昼战冻结的 `enemy_nowhps` 会吞掉夜战击沉，已删除。
  见 `game/sortie_result.rs`。
- [2026-09-20] SeaORM 只当 struct↔row 映射器 + DDL 生成器用：195 处 `Entity::find()` 里 0 个 join /
  `group_by` / `find_with_related`，`Relation` 只喂 `create_table_from_entity`。换 sqlx/rusqlite 要重写
  ~450 处调用来省 25 个依赖，不值。
- [2026-09-20] 跑完整的 `cache populate` 就是权威的 CDN 存在性探针：它请求每条非 hole 路径，失败清单即
  「上游缺什么」，`--greedy` 不是替代（见 Pitfalls）。[2026-09-22] 但它只请求表内路径——`z/cache` 的 336 个
  `btxt_flat` 与 `BTXT_FLAT_IDS` 条数相同，只证明「表内存在」，表外 id 必须单独探测。
- [2026-09-20] hole 表的流向是 Rust → 资产而非 decoder → Rust：`main-decoder/src/path-rules.ts` 从 Rust 源码
  反解 `EVENT_SHIP_HOLES` / `BTXT_FLAT_IDS` / `CHARACTER_HOLES` 写进 `cache_rules.json`。改 hole 要改 Rust
  常量再 `make decode-main`。
- [2026-09-20] `start2` 里没有任何字段能把 5 条补给形态舰（743/744/745/748/749，名字以 補 结尾）与普通
  友军舰区分开——`api_sortno`/`api_backs`/`api_aftershipid`/`ship_picturebook.json` 都查过；友军舰队
  graph id 6299/6301/6303 同理。不要再找规则。
- [2026-09-21] `KC3Kai/kancolle-replay` 的 `js/kcsim.js` 是第二条独立数据链（`COMBINEDCF1-4`
  与 `COMBINEDCONSTS` 复现 wikiwiki 的联合舰队表）。其精度/回避補正对本项目无用。
- [2026-09-22] `cargo test --workspace` exit 0；09-20 记的三条 baseline 失败不再复现，别当既有失败引用
  （`mkdir -p target/tmp` 的前提见 Pitfalls 的 `test_font` 条）。

- [2026-09-21] `emukc_network` 的下载层先读完 body 再开目标文件，非 2xx 直接报错，失败的传输从不写出
  半截文件。计划 005 的 `.part` + rename 只堵了 truncate-to-copy 窗口；真正的数据丢失来自
  `bootstrap --force-update` 在下载前就删文件。
- [2026-09-21] kccp 源里 11 条任务缺 name（256/615/616/622/627/628/630/632/633/648/652），游戏内显示
  `n/a`。上游数据缺失，不是解析器问题，补标题需要另一个数据源。
- [2026-09-21] `extract_label_type` 的第五种周期字母 `s`（Cs1/2/3/5/6，5 个真实 wiki_id）在 `match` 里
  没有分支，落到 label_type 1——而这个结果是对的：本仓库把 `Frequency::Seasonal` 映射成
  `Kc3rdQuestPeriod::Oneshot`，1 正是客户端的一次性标签页（011 已写进测试）。计划 002 说它「和未命中的
  年任务一样落到 1」有误导；计划 011 的修复只提了 By/Cy 的 7 个，别漏掉 `s`。

- [2026-09-21] `serde` 的 `visit_map` 按文档顺序交付条目，所以「保序地读一个 JSON 对象」不必开
  `serde_json/preserve_order`——该 feature 在 feature unification 下全工作区生效，会改掉 27 处
  `serde_json::Map` 的顺序。自定义 `Visitor` 收 `Vec<(K, V)>` 即可，见 `parser/kccp/quest.rs`
  的 `OrderedEntries`。
- [2026-09-21] `make_list` 的 "holes" 有两义：已删的 holes **report**（`HOLES_COLLECTOR` /
  `holes_report.rs`，无写入方）与活着的 `ShipPathHoles` 跳过表（manifest 规则在用）。按词清理会误删后者。
- [2026-09-21] `.html` 缓存无法内容嗅探：真实 `kcs2/hc.html` 仅 54 字节且与错误页同形，
  所以 008 只给它留非空检查。
- [2026-09-21] 需求解析失败在 `or` 与 `and`/`then` 下处理相反：丢一个 `or` 分支只减少完成路径
  （更严格），丢一个 `and` 条件则白送。统一 `?` 传播会误杀 api_no 1019 这类一坏一好分支的任务。
  三条的完整依据见 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 的 006/008/010 段。
- [2026-09-21] `kcs/sound/kcwjcrloeyiyxw/158288.mp3` 在全部镜像上都是 200 + 空 body（4 个 w0* 主机实测）。
  `Kache` 现在把「每个镜像都应答且都不可用」返回成 `InvalidFile` 而非 `FailedOnAllCdn`，populate 归入
  missing 不再重试；注意 `exists_on_remote` 走 HEAD，仍判 `Present`。
- [2026-09-21] drift-check 跟踪 13 个资产（4 battle + 2 map-catalog + 7 cache-list 输入），基线 `--accept`
  到 6.3.5.0，入口 `make drift-check` / `make drift-accept`。指纹按规范化后的字节算，`_0x` 重命名即使
  解码知识没变也算漂移。
- [2026-09-21] manifest 差集结案，**不要补**：
  `docs/solutions/best-practices/manifest-minus-rules-difference.md`。
- [2026-09-21] `api_alignment_e2e` 的 `600..700` 过滤不是缺陷：活动图是三位数 id，
  非活动期 codex 只有 11..75 的常规图，那个循环本来就该一条不匹配（测试注释已写明）。

- [2026-09-21] `emukc_battle` 里的 `escort` 绝大多数指「旗艦援護/かばう」（旗舰护盾），
  与联合舰队的护卫舰队同名不同义。判断联合舰队相关代码要看 `combined*`，不要 grep `escort`。
- [2026-09-22] 改修配方不需要新数据源，且已对官方验证：codex `slotitem_extra_info.improvement` 的
  `base_consumption` 与官方 `remodel_slotlist` 逐字段相同，官方 `api_req_buildkit`/`api_req_remodelkit`
  就是 codex 的 `dev_mat_min`/`screw_min`（发 min，不发范围）。`secretary` 是**二番舰**不是旗舰，旗舰
  必须明石(182)/明石改(187)；variant 在 ★0–★9 仍是普通改修，只在 ★10 转换。
- [2026-09-22] 真实 questlist：服务端**按 `api_tab_id` 过滤**，tab↔`api_type` 1:1（0 全部/9 进行中/
  1 日/2 周/3 月/4 单发/5 其他），`api_label_type` 固定 type1→2、2→3、3→6、4→1、5→7+101..111；
  进行中为空时 `api_list` 是 **null** 不是 `[]`。样本 `z/snapshot/2026-09-22/quest_tab*.json`。
- [2026-09-22] 官方 `api_start2` 的 `api_mst_mission`（63 条）**没有解锁条件字段**，消耗是比例
  （`api_use_fuel:0.3`）：TODO 的「远征解锁表可靠数据源」不要再去 start2 找。
- [2026-09-21] 联合舰队 friendly 索引有三层空间（模拟连续 / 客户端固定从 6 / 切片本地编号），
  缺一层就把命中记到错误的舰上：`docs/solutions/architecture-patterns/combined-fleet-index-spaces.md`。

- [2026-09-22] 基地航空隊的客户端硬限制（`AIRUNIT_MAX`/`SQUADRON_MAX`）、
  `airbase_count` 是出撃可能数而非拥有数、`expand_base` 是増开一隊而非扩槽、
  空槽为何不落库：四条都在
  `docs/plans/2026-09-22-001-feat-land-base-air-corps-plan.md` 的前提表与各 U 修正段。
- [2026-09-22] `remodel_slot_recover` 有两条推不出来的约束：客户端是
  `model.slot.get(slot_id).__updateObject__(api_after_slot)`，所以 `api_after_slot` 必须是
  **同一实例 id**——重置不能删建装备，★10 variant 也不退回原装备；`api_dev_num` 只有 1/2/3
  （`ResetDialog` 的 radio），而成功率上游从不告诉客户端，50/75/100 是本项目定的值
  （`codex/remodel_slot.rs::recover_success_rate`）。

- [2026-09-21] 敌方联合舰队在本地 codex 里**没有数据**：`map_catalog.json` 只有 37 张常规图
  （1-1~7-5），编成 >6 船 0 个、`battle_kind` 只有 1。敌联合只在活动海域出现，所以 `ec_*`/`each_*`
  五个端点本地无格可触发、做不出端到端测试。不要再去 codex 里找。
- [2026-09-22] `api_si_list` 只放对应阶段画得出名牌的装备；`btxt_flat` 的存在范围与探测方法见
  `docs/solutions/architecture-patterns/battle-display-name-plates.md`。
- [2026-09-22] 同名 `PhaseHougeki` 昼夜两份编号互斥（昼 2=連撃/7=空母切入，夜 1=連撃/6=空母切入）；
  消歧只能按依赖它的 dispatcher（`PhaseDay*` vs `PhaseNight`/`PhaseAllyAttack`），按 hotspot 深浅挑会静默出错。

## Failed Attempts / Pitfalls

| Pitfall | Source |
| --- | --- |
| populate 基准三坑：`head -200` 清单全是 mp3（带宽受限，把收益掩成 10%，全量实为 49% mp3 + 49% png，要随机抽样）；`Kache::build()` 要求 `cache_root` 已存在，否则 exit 1 且 stdout 无输出；配置里的相对路径按 config 文件所在目录解析。 | 计划 003「实测结果」(2026-09-21) |
| `emukc_bootstrap` 的 `make_list` 两个测试打真实 CDN（`make_kache()` 用 `socks5://127.0.0.1:1086`），网络一抖就 `FailedOnAllCdn`，fail-fast 会让整轮 workspace 测试在该 crate 中断。已加 `skip_if_offline`：只吞这一种错。 | session 2026-09-21 |
| 联合舰队「deck 1 不开幕对潜/雷击」不能靠切片 attackers——敌方在这些阶段仍打两支 deck，切片会连带砍掉敌方目标池。须按船过滤。 | session 2026-09-21 |
| Seeded test RNG left thread-local entropy set → cross-test pollution. Must restore entropy after seeded runs. | git `66f8317` |
| Cache downgraded to an older local file on version rollback instead of serving the newer local copy. | git `185c0b8` |
| `remodel()` dropped fields + faulty boiler query (logic error). | `docs/solutions/logic-errors/remodel-preserve-fields-and-boiler-query-2026-05-14.md` |
| Clippy warning triage across the workspace. | `docs/solutions/best-practices/resolve-clippy-warnings-triage-2026-05-28.md` |
| Grepping `test result:` to verify tests misses failures — a FAILED target prints its own line that is easy to lose among many suites. Check the cargo exit code instead. | plan 004 U5 (2026-06-22): reported "821 passed" while 3 `sortie_battle.rs` tests were failing |
| Upgrading sea-orm does NOT clear the `proc-macro-error2` future-incompat warning: `sea-orm-macros` 2.0.3 still pulls `sea-bae 0.2.1`, same as 1.1.20. Do not cite it as an upgrade reason. | session 2026-09-20, `cargo tree -i proc-macro-error2` |
| [2026-07-30] Seed-search tests inherited local `god_mode` / `one_hit_kill`, making the night branch unreachable; normalize debug policy in the test fixture instead of changing production behavior or local data. | git `64a8239` |
| 2026-08-26 stale cache-list incident: `bootstrap` died in Phase 2 (`kcwiki_enemy.json` `BoolOrString` null), so Phase 4 never refreshed main.js; decode and make-list then consumed stale inputs. Diagnose from `version.json` and main.js mtime, not from make-list. | session 2026-08-26 |
| `clearing_1_1_unlocks_1_2` flakiness is compass routing plus damage carrying across retries, not damage RNG (~80% of 1-1 sorties dead-end before the boss). Fix: restore fleet HP/fuel/ammo via `find_ship`/`update_ship` before each attempt. Leveling the fleet does not help. | session 2026-08-26 |
| pi-lens 每次编辑 Makefile 都重跑 shellcheck（解析不了 Make 语法）且无视 ignore glob；已在 `.shellcheckrc` 与 `.pi-lens.json` 修好，但会话缓存会重放到会话结束。 | git `b0284d1`, `83ecebb` |
| `find_ship_impl` does not filter by `profile_id`, and the deduct+mutate template `open_ship_exslot_impl` lacks an ownership check — cross-profile mutation is one copy-paste away. New find-then-mutate ops must compare `profile_id` (see `expand_hangar_slot_impl`). | session 2026-08-26 |
| SeaORM `update()` skips `NotSet` columns, so remodel's rebuild-via-`codex.new_ship` preserves columns `KcApiShip` cannot carry. Derived output-only fields (e.g. `api_onslot_max`) must never be written back into their source increment columns. | session 2026-08-26 |
| [2026-09-18] `crates/emukc_gameplay/tests/practice_battle.rs` asserts an unseeded battle's `api_win_rank`, so 2 of its 11 tests fail at roughly a 1-in-3 rate on any commit — it is not a regression signal. Re-run the single target before blaming a change. | session 2026-09-18 |
| [2026-09-18] `net::router::version::test::test_font` writes to `./target/tmp/`, which does not exist when `CARGO_TARGET_DIR` points outside the repo. `mkdir -p target/tmp` once per clone; `cargo clean` or a new machine breaks it again. | session 2026-09-18 |
| [2026-09-18] `-W warnings` and `cargo test` never fail on warnings; a test-target `dead_code` slipped through U1. Gates must run `clippy --all-targets` and fail on warnings in touched files. | git `0066096`, `.farm/deepen-u3-gate.sh` |
| [2026-09-18] A stale `target/` can fail `cargo test` with `BattleContext::head_on` not found although the fn is `pub`; `cargo clean -p emukc_battle` fixes it. Diagnose before blaming a change. | session 2026-09-18, U1 worker report |
| [2026-09-21] `emukc_time`'s `test_jst_next_28/370_day_of_the_month` failures are DATE-dependent: they overflowed at `lib.rs:355` on 09-20 and passed untouched on 09-21. Note the date before calling them baseline. | sessions 2026-09-18, 09-21 |
| `cargo clippy` 默认档比 `-D warnings` 宽（漏过 `match`→`let-else`），但 `-D` 会被既有的 `result_large_err`（`emukc_network/src/download.rs:236`、`src/bin/net/auth.rs:139`）挡住。仓库门是 `-W warnings`；新代码用 touched-file 的 `-D` 检查。 | plan 004 U7、sessions 2026-09-18/19 |
| [2026-09-19] `tests/gameplay_tests/mod.rs` is a dead file: the compiled entry is `tests/gameplay_tests.rs` with `#[path]` module decls, so a `mod` added only to the dead file registers nothing. Verified with `compile_error!` by the U7 worker. | session 2026-09-19, U7 |
| [2026-09-19] `sed -i.bak X && cargo test; mv X.bak X` 给假结果：`.bak` 保留原 mtime，cargo 认为没变，复用按**改动后**源码编出的产物。还原后必须 `touch` 再跑。 | session 2026-09-19 |
| [2026-09-19] Missing `main-decoder/node_modules` makes `bun run decode` fail as `Unexpected HTTP` / `Cannot find module '@babel/generator'`, which reads like a corrupt download. `bun install` first; it also unblocks `bun run check`. | git `688e29c` |
| [2026-09-19] Pinning decoder tests to webpack module ids breaks on every upstream build (`DutyModel_` 56360→82131, `PhaseHougeki` 65622→two modules 1830/74885). Match `readableName`, and for duplicate names take the deepest hotspot cleanup. | git `b1016fc` |
| [2026-09-19] A refactor comment saying "keeps feeding X as before" was preserving a bug: plan 002 froze the day-battle `enemy_nowhps` copy, so night-only sinks fired no quest event. Treat "as before" as unverified. | git `HEAD` |
| [2026-09-19] Per-file gate checks break when a file holds both `Ctx` methods and `_impl`s (U8: `ndock.rs` had to both call and not call `observe`); rely on the AE grep instead. `tests/gameplay_tests/quest/*.rs` use sync `#[test]`, so `#[tokio::test]` counts are 0. | session 2026-09-19, U8 |
| [2026-09-20] `--greedy`'s holes report is dead code: `HOLES_COLLECTOR` has a reader and a clear but no writer, so it is always empty and `GreedyConfig.concurrent` is unused. `z/cache/holes_report.txt` is an April artifact, not current data. | session 2026-09-20, `ship.rs` |
| [2026-09-20] Duplicate progress bars are NOT error line-wraps: any terminal write bypassing `MultiProgress` strands a copy of the bar block in scrollback and is itself overwritten (invisible). Spinner churn, resizes and `mp.suspend` all tested clean. | session 2026-09-20 |
| [2026-09-21] A plan naming one instance of a defect does not bound the fix to it: 007 cited `unwrap_or(false)` in `gauge.rs`; one line down `make_gauge_by_id` mapped every error to `Ok(false)` — same swallow, 3 call sites. Grep the file for the shape, not the cited line. | git `2497efc` |
| [2026-09-20] Never `mp.add()` a `ProgressBar` per work item: indicatif 0.18 reaps only zombies consecutive from the head of `ordering`, and the head is the permanent aggregate bar, so finished bars leak and every redraw walks them. 73k spinners = 2m13s vs 3s. | session 2026-09-20, `populate.rs` |

| [2026-09-21] 「无 .data 跑测试」时备份必须放到**仓库外**：cache rules 的测试会
`create_dir_all(".data/tmp")`，`mv .data .data.bak` 再移回会把备份塞进新目录。正确做法
`mv .data ../.data-bak` → 跑 → `rm -rf .data` → `mv ../.data-bak .data`。 | session 2026-09-21 |
| [2026-09-21] `crates/emukc_battle/tests/golden/*.txt` 是 `{:#?}` dump，加字段就全量
失配，哪怕值恒为 `None`。不是模拟漂移：`EMUKC_BLESS_GOLDEN=1` 后确认每份 diff 只有那一行。
`battle_golden.rs` 渲染 transcript，加字段不动它——Stop condition 只针对后者。 | session 2026-09-21 |
| [2026-09-21] 改 `resource-categories.ts` 的 `defaultAbyssal` 是 no-op：`ship_semantic_targets_for_id` 先查 `targetSemantics`，命中就 `continue`，生成分组只是未覆盖 target 的兜底。加了 `banner_dmg` 后 `bun test` 62 pass、decode+sync 成功、清单一条不变。 | session 2026-09-21 |

## Last Session

- [2026-09-22] 战斗协议语义闸门收口（计划 `2026-09-22-1435-fix-battle-protocol-semantics-gate-plan.md`，
  U1–U9）：13 个提交已 fast-forward 进 `main`，分支已删；审查 verdict 的发现全部落地
  （`28328e62`、`c888a92f`）；收尾把 `battle_rules.rs` 拆成 `battle_rules/{attack_types,resources}.rs`
  （实现部分 1720 → 830 行，纯移动，测试 1100 行未动）。
- 用真实账号 token 打了一轮官方 API（只读 + 两次 `set_plane`）：12 份存档快照在 `z/snapshot/2026-09-22/`，
  量出配属消耗、钉死基地航空隊状态机、移植并真机验证了 `PortApiKey`。结论都在「已验证的事实」。
- 门禁：`cargo test --workspace` exit 0；fmt clean；clippy 改动文件零告警；`make drift-check` no drift。

## Next Session

- [2026-09-22] 回到基地航空隊计划的 U4（`set_action`、`change_name`、`supply`）：补给消耗系数仍是唯一不齐的点，
  按 wikiwiki → `KC3Kai/kcsim.js` 顺序取数，两条都取不到就停下不要编公式；
  取到之后**同时**接上 `set_plane` 的配属消耗。之后 U5、U6 收尾。
- 基地航空隊新积压：按真实响应修 `set_plane` 撤下的状态机（`api_state:2` + 保留 slotid + 半径不变），
  给 `api_port/port` 补 `api_plane_info`（`api_base_convert_slot` / `api_unset_slot`）、`api_event_object`、
  `api_c_flags`、`api_c_flag2`、`api_friendly_setting`、`api_combined_flag`，并改 U3 断言 0/0 的那个测试。
  真实存档快照在 `z/snapshot/2026-09-22/`（13 份，含 port）。
- 战斗侧新积压一条：特殊攻击（`api_at_type = 100`）按每参战舰发一条记录，
  官方是一条记录带三个目标。取值合法所以新闸门抓不到，见 `docs/battle/rules.md` Follow-up。
- 更早积压未变：対空/阵形建模先做「補正表能否解码」spike；审计集 013 等上游版本；敌联合 5 端点卡在数据不存在。
