# PROJECT_MEMORY.md

Cross-session persistent state. Each section cites its source. This file is an
**index + session state** — authoritative detail lives in `CLAUDE.md`
(architecture / commands / style) and `docs/solutions/` (detailed lessons).

**Maintenance:**

- Update `Last Session` and `Next Session` at the end of every working session.
- `Verified Facts` and `Failed Attempts` are cumulative; append with a date and a source link.
- Do not duplicate `docs/solutions/` content — link to it.

Last updated: 2026-09-21 · branch `main`

## Verified Facts

Architecture — 分层、`Codex`、`entity::user`/`entity::profile` 分域、`svdata=` 前缀等基础见
CLAUDE.md § Architecture；这里只记从中推不出来的：

- [2026-09-18] Gameplay ops are inherent `async fn`s on the concrete `gameplay::Ctx`; owners (`State`, `TestContext`, `SimContext`) embed it and `Deref`, which keeps `state.foo(..)` call sites unchanged.
- [2026-09-18] No `XxxOps`/`GameOps`/`Gameplay`/`HasContext` trait and no `async-trait` in `emukc_gameplay`. Source: `docs/solutions/architecture-patterns/gameplay-context.md`.
- Internal helpers are suffixed `_impl` and take `C: ConnectionTrait`, so they can join transactions started by public `Ctx` methods and be reused across modules. Cross-domain writes go through `_impl`, not through a second inherent method (that would open a nested transaction).
- [2026-07-30] Cross-crate battle callers use `emukc_battle::execute_day` / `execute_night`; raw simulation and debug-overlay composition remain crate-internal. Source: `crates/emukc_battle/src/execution.rs`, `docs/solutions/architecture-patterns/battle-crate-docs.md`.

代码风格与禁改文件：完全按 CLAUDE.md § Code Style / § Do-Not-Modify Files，无额外约定。

Current verification baseline:

- [2026-07-30] CLI battle simulation tests load the real Codex but force `god_mode=false` and `one_hit_kill=false`, so local `.data/codex/game_config.json` cannot make seed-search tests non-hermetic. Source: `src/bin/cli/battle.rs::load_codex_without_debug_policy`.
- [2026-07-30] Cache-list validation accepts nonzero map start-source cells and readable slot-item expressions. Source: `crates/emukc_model/src/codex/map.rs`, `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs`.
- [2026-09-19] `make update` = `bootstrap --overwrite --force-update` → decode `--sync-assets
  --sync-battle-assets --sync-resource-manifest` → `cache make-list --overwrite`. Verified on 6.3.2.1 and
  6.3.5.0 (the latter with main.js fetched by hand).
- [2026-09-19] `make-list` 默认策略会吸收 `cache_rules.json` 的新显式路径，不需要 `--manifest`
  （6.3.4.1 → 6.3.5.0 实测：battle 资产零变化，清单只多一行显式路径）。
- [2026-09-19] `obfuscator-io-deobfuscator` (ben-sb v1.0.6) does NOT replace `main-decoder`: 0/55 battle
  fields and 304 s / 5.1 GB on 6.3.5.0, vs our 55/55 in ~20 s. Re-evaluating it is waste.
- [2026-09-19] Two `main.js` version axes: `kcs_const.js` `scriptVesion` (note the upstream typo) is the client
  script version and drives `out/version.txt` plus every synced asset's `scriptVersion`; `kcs2/version.json`
  holds per-subsystem asset versions and moves independently. A main.js-only release bumps the first, not both.
- [2026-08-26] Upstream drift: kcwiki empty equipment slots are `null`, not `false`; `version.json` nests a `resources` object (flattened in `parse_version_info`); webpack emits shorthand `ObjectMethod` factories (normalized in `module-graph.ts`); event area 62 is unlocked by default by design.
- [2026-09-18] `required_exp(99) == required_exp(100) == 1_000_000` by design (marriage unlocks Lv.100 at the same exp), so `exp_to_ship_level(1_000_000)` is 100 and unmarried callers rely on `min(cap)`. Not a bug. Source: `kc2/level.rs`, `game/ship/exp.rs` tests.
- [2026-09-19] `update_quest_progress_for_action` reads only quest progress rows and the codex (`quest/update.rs:174-244`), so it can run at any point inside a domain transaction; U8 moved it to just before `commit` with zero behavior change. Source: `.farm/deepen-u8-report.md`.
- [2026-09-19] `questlist` `api_tab_id` is the client tab bar (0,9,1,2,3,4,5 = all, activated, daily, weekly, monthly, oneshot, other); the client filters nothing itself. `api_label_type` keys the row label (1,2,3,6,7,101..=112). Source: `main.decoded.js` `DutyDataHolder`, `_createTab`.
- [2026-09-19] `SortieStore::with_profile_lock` has exactly five holders (`start_sortie`, `next_sortie`,
  `sortie_battle_result`, `sortie_sp_midnight_battle`, `sortie_battle_impl`); none nests another, so a new
  entry can take it safely. `sortie_midnight_battle` stays unlocked on purpose — it mutates a pending
  session, not `active`.
- [2026-09-21] 两份 apilist 各司其职。`apilist.md` 是唯一端点清单（当前计数以该文件为准，missing =
  `docs/apilist.txt` 的 136 减 implemented）；重推方法是抽 `kcsapi/mod.rs` 的 `nest("/prefix", ..)` 加各
  子模块 `.route("/leaf"` 再双向 diff，不要手工审。`docs/api_coverage.md` 只是路线图。
  `docs/apilist.txt`（4209 行）是字段**语义**来源，比 GitHub 上任何副本全，别再去外面找；解码客户端
  仍是字段**是否存在**的唯一真源。
- [2026-09-19] Sunk-enemy quest events come only from `settle_sortie_battle_impl`'s `final_enemy_nowhps`
  (the post-night session packet), the same slice as `api_dests`. The snapshot's own day-frozen
  `enemy_nowhps` copy swallowed night-only sinks and is deleted. Source: `game/sortie_result.rs`.
- [2026-08-26] `GetOption::new_remote_only()` now really bypasses local cache: `fetch_from_remote` skips its local dedup check when `enable_local` is false.
- [2026-09-20] SeaORM is used as a struct<->row mapper + DDL generator, not an ORM: 0 joins / `group_by` /
  `find_with_related` across 195 `Entity::find()` sites; `Relation` only feeds `create_table_from_entity`.
  Swapping to sqlx/rusqlite = ~450 call sites rewritten to shed 25 crates. Not worth it.
- [2026-09-20] A full `cache populate` over the generated list IS the authoritative CDN existence probe — it
  requests every non-hole path, so its failure list answers "what is missing upstream". `--greedy` is not a
  substitute (see Pitfalls).
- [2026-09-20] Hole tables flow Rust -> asset, not decoder -> Rust: `main-decoder/src/path-rules.ts` parses
  `EVENT_SHIP_HOLES` / `BTXT_FLAT_IDS` / `CHARACTER_HOLES` back out of the Rust sources into
  `cache_rules.json`. To change a hole, edit the Rust constant, then `make decode-main` to re-sync.
- [2026-09-20] Nothing in `start2` distinguishes the 5 resupply-form ships (743/744/745/748/749, names
  ending in 補) from normal friendly ships — checked `api_sortno`, `api_backs`, `api_aftershipid` and
  `ship_picturebook.json`. Same for friend-fleet graph ids 6299/6301/6303. Do not re-hunt for a rule.
- [2026-09-21] `KC3Kai/kancolle-replay` 的 `js/kcsim.js`（活跃维护）是第二条独立数据链：`COMBINEDCF1-4`
  与 `COMBINEDCONSTS` 逐格复现 wikiwiki 的联合舰队阵形表与補正表。其精度/回避補正对本项目无用（不建模命中率）。
- [2026-09-21] `cargo test --workspace` 全绿；09-20 记的三条 baseline 失败均已不再复现，
  不要再当既有失败引用。（`mkdir -p target/tmp` 的前提见下面 Pitfalls 的 `test_font` 条。）

- [2026-09-21] `emukc_network` 的下载层先读完整个 body 再开目标文件，非 2xx 直接报错，所以失败的传输
  从不写出半截文件。计划 005 的 `.part` + rename 只堵了 truncate-to-copy 的窗口；真正的数据丢失来自
  `bootstrap --force-update` 在下载前就删掉了文件。
- [2026-09-21] kccp 源里 11 条任务缺 name（256/615/616/622/627/628/630/632/633/648/652），
  游戏里标题显示 `n/a`。这是上游数据缺失，不是解析器问题，补标题需要另一个数据源。
- [2026-09-21] `extract_label_type` 有第五种周期字母 `s`（Cs1/2/3/5/6 共 5 个真实 wiki_id），
  `match` 里没有分支，和未命中的年任务一样落到 label_type 1。计划 011 只提了 By/Cy 的 7 个，
  修的时候别漏掉 `s`。

- [2026-09-21] `serde` 的 `visit_map` 按文档顺序交付条目，所以「需要保序地读一个 JSON 对象」
  不必开 `serde_json/preserve_order`——那个 feature 在 feature unification 下全工作区生效，
  会改掉 27 处 `serde_json::Map` 的迭代与序列化顺序。自定义 `Visitor` 收集 `Vec<(K, V)>` 即可，
  范围只在一个函数内。见 `parser/kccp/quest.rs` 的 `OrderedEntries`。
- [2026-09-21] `Cs*`（seasonal）的 `label_type = 1` 是对的，不是缺陷：本仓库把
  `Frequency::Seasonal` 映射成 `Kc3rdQuestPeriod::Oneshot`，而 1 正是客户端的一次性标签页。
  计划 002 的注记说它「同样落到 1」有误导，011 已澄清并写进测试。
- [2026-09-21] `make_list` 的 "holes" 有两义：已删的 holes **report**（`HOLES_COLLECTOR` /
  `holes_report.rs`，无写入方）与活着的 `ShipPathHoles` 跳过表（manifest 规则在用）。按词清理会误删后者。
- [2026-09-21] `.html` 缓存无法内容嗅探：真实 `kcs2/hc.html` 仅 54 字节且与错误页同形，
  所以 008 只给它留非空检查。
- [2026-09-21] 需求解析失败在 `or` 与 `and`/`then` 下处理相反：丢一个 `or` 分支只减少完成路径
  （更严格），丢一个 `and` 条件则白送。统一 `?` 传播会误杀 api_no 1019 这类一坏一好分支的任务。
  三条的完整依据见 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 的 006/008/010 段。
- [2026-09-21] `kcs/sound/kcwjcrloeyiyxw/158288.mp3` 在全部镜像上都是 200 + 空 body
  （curl 实测 4 个 w0* 主机，`content-type: audio/mpeg`，`size=0`）。`Kache` 现在把
  「每个镜像都应答且都不可用」返回成 `InvalidFile` 而非 `FailedOnAllCdn`，populate 归入
  missing 类，不再重试。注意 `exists_on_remote` 走 HEAD，仍会把它判成 `Present`。
- [2026-09-21] drift-check 跟踪 13 个资产（4 battle + 2 map-catalog + 7 cache-list 输入），基线已
  `--accept` 到 6.3.5.0，入口 `make drift-check` / `make drift-accept`。指纹按规范化后的字节算，
  `_0x` 重命名即使解码知识没变也算漂移——这是「收敛版本记录」要解决的噪声来源。
- [2026-09-21] manifest 差集（21,510 条）已全量 HEAD 探测结案，结论是**不要补**：
  `docs/solutions/best-practices/manifest-minus-rules-difference.md`。
- [2026-09-21] `api_alignment_e2e` 的 `600..700` 过滤不是缺陷：活动图是三位数 id，
  非活动期 codex 只有 11..75 的常规图，那个循环本来就该一条不匹配（测试注释已写明）。

- [2026-09-21] `emukc_battle` 里的 `escort` 绝大多数指「旗艦援護/かばう」（旗舰护盾），
  与联合舰队的护卫舰队同名不同义。判断联合舰队相关代码要看 `combined*`，不要 grep `escort`。
- [2026-09-21] 改修配方数据早就在 codex 里，不需要新数据源：`slotitem_extra_info` 的
  `improvement` 覆盖 174 件装备，含分档材料、消耗装备/道具与秘书舰。`secretary` 字段是
  **二番舰**（睦月/如月系），不是旗舰；旗舰必须是明石(182)/明石改(187)，这是两件事。
- [2026-09-21] variant 配方在 ★0–★9 就是普通改修，只在 ★10 才转换成 variant 装备。
  12cm単装砲(1) 没有 `level_consumption`、只有 variant，所以「有 variant」不等于「只能更新」。
- [2026-09-21] 联合舰队有两套 friendly 索引空间（模拟连续 vs 客户端固定从 6），
  外加 `simulate_shelling_side` 的切片本地编号，一共三层，缺一层就把命中记到错误的
  舰上。翻译点、端点与编成的双向契约、夜战测试为什么打不出来：
  `docs/solutions/architecture-patterns/combined-fleet-index-spaces.md`。

- [2026-09-22] `apilist.md` 的 implemented 是 router 的机械投影，missing 是「`docs/apilist.txt`
  减 router」，两者不互补：不在上游参考里的端点（`remodel_slot_recover`）只进前者，missing
  计数不动。`registration_sp` grep 仍零命中，不是缺口。
- [2026-09-22] `remodel_slot_recover` 有两条推不出来的约束：客户端是
  `model.slot.get(slot_id).__updateObject__(api_after_slot)`，所以 `api_after_slot` 必须是
  **同一实例 id**——重置不能删建装备，★10 variant 也不退回原装备；`api_dev_num` 只有 1/2/3
  （`ResetDialog` 的 radio），而成功率上游从不告诉客户端，50/75/100 是本项目定的值
  （`codex/remodel_slot.rs::recover_success_rate`）。

- [2026-09-21] 敌方联合舰队在本地 codex 里**没有数据**：`map_catalog.json` 是 37 张常规图
  （1-1~7-5），编成船数分布 1/2/3/4/5/6 = 13/7/75/95/165/1222，**>6 船 0 个**，`battle_kind`
  只有 1 这一个取值。敌联合只在活动海域出现，所以 `ec_*`/`each_*` 五个端点本地无格可触发、
  做不出端到端测试。不要再去 codex 里找敌联合编成。
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
| [2026-07-30] Do not delete a divergent branch merely during cleanup. `codex/fix-cache-list-warnings` contained one valuable commit; it was inspected, rebased onto current `main`, retested, fast-forwarded, then deleted. | git `0395121` |
| 2026-08-26 stale cache-list incident: `bootstrap` died in Phase 2 (`kcwiki_enemy.json` `BoolOrString` null), so Phase 4 never refreshed main.js; decode and make-list then consumed stale inputs. Diagnose from `version.json` and main.js mtime, not from make-list. | session 2026-08-26 |
| `clearing_1_1_unlocks_1_2` flakiness is compass routing plus damage carrying across retries, not damage RNG (~80% of 1-1 sorties dead-end before the boss). Fix: restore fleet HP/fuel/ammo via `find_ship`/`update_ship` before each attempt. Leveling the fleet does not help. | session 2026-08-26 |
| pi-lens edit-time dispatch re-runs shellcheck on every Makefile edit and ignores `.pi-lens.json`'s ignore glob; shellcheck cannot parse Make syntax. Fixed in both `.shellcheckrc` and `.pi-lens.json` rules.disable. Session cache replays persist until the session ends. | git `b0284d1`, `83ecebb` |
| `find_ship_impl` does not filter by `profile_id`, and the deduct+mutate template `open_ship_exslot_impl` lacks an ownership check — cross-profile mutation is one copy-paste away. New find-then-mutate ops must compare `profile_id` (see `expand_hangar_slot_impl`). | session 2026-08-26 |
| SeaORM `update()` skips `NotSet` columns, so remodel's rebuild-via-`codex.new_ship` preserves columns `KcApiShip` cannot carry. Derived output-only fields (e.g. `api_onslot_max`) must never be written back into their source increment columns. | session 2026-08-26 |
| [2026-09-18] `crates/emukc_gameplay/tests/practice_battle.rs` asserts an unseeded battle's `api_win_rank`, so 2 of its 11 tests fail at roughly a 1-in-3 rate on any commit — it is not a regression signal. Re-run the single target before blaming a change. | session 2026-09-18 |
| [2026-09-18] `net::router::version::test::test_font` writes to `./target/tmp/`, which does not exist when `CARGO_TARGET_DIR` points outside the repo. `mkdir -p target/tmp` once per clone; `cargo clean` or a new machine breaks it again. | session 2026-09-18 |
| [2026-09-18] `-W warnings` and `cargo test` never fail on warnings; a test-target `dead_code` slipped through U1. Gates must run `clippy --all-targets` and fail on warnings in touched files. | git `0066096`, `.farm/deepen-u3-gate.sh` |
| [2026-09-18] A grep gate (`api_f_nowhps`) matched a test *read* and the worker rewrote the assertion to pass it. Gate greps must match assignments (`name:`); briefs must forbid changing assertions to satisfy a gate. | session 2026-09-18, U3 |
| [2026-09-18] A stale `target/` can fail `cargo test` with `BattleContext::head_on` not found although the fn is `pub`; `cargo clean -p emukc_battle` fixes it. Diagnose before blaming a change. | session 2026-09-18, U1 worker report |
| [2026-09-21] `emukc_time`'s `test_jst_next_28/370_day_of_the_month` failures are DATE-dependent: they overflowed at `lib.rs:355` on 09-20 and passed untouched on 09-21. Note the date before calling them baseline. | sessions 2026-09-18, 09-21 |
| `cargo clippy` 默认档比 `-D warnings` 宽（漏过 `match`→`let-else`），但 `-D` 会被既有的 `result_large_err`（`emukc_network/src/download.rs:236`、`src/bin/net/auth.rs:139`）挡住。仓库门是 `-W warnings`；新代码用 touched-file 的 `-D` 检查。 | plan 004 U7、sessions 2026-09-18/19 |
| [2026-09-19] `tests/gameplay_tests/mod.rs` is a dead file: the compiled entry is `tests/gameplay_tests.rs` with `#[path]` module decls, so a `mod` added only to the dead file registers nothing. Verified with `compile_error!` by the U7 worker. | session 2026-09-19, U7 |
| [2026-09-19] `sed -i.bak X && cargo test; mv X.bak X` gives FALSE results: `.bak` keeps the ORIGINAL mtime, so cargo sees no change and reuses the artifact built from the EDITED file. `touch` X after restoring and re-run. | session 2026-09-19 |
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

- [2026-09-22] 提交 `75ecde9`（联合舰队 U8 的四个端点，门禁重跑确认），然后实现
  `api_req_kousyou/remodel_slot_recover`：`Ctx::remodel_slot_recover` + 端点 + 两个端到端
  测试，`KcUseItemType::ArsenalResource = 104`（工廠資源）与
  `codex::remodel_slot::recover_success_rate` 为新增。语义全部从解码客户端的
  `RevampSlotLevelResetAPI` 与 `TaskSelectResetSlotitem` 追出来，依据见上面那条事实与
  `docs/api_coverage.md`。无条件扣 1 个工廠資源、成功才扣 `api_dev_num` 个開発資材。
- 门禁：`cargo test --workspace` exit 0（47 个 suite 全 ok）；fmt clean；clippy
  `--all-targets -W warnings` 17 条真实 lint，逐条定位全在未改动文件。清单机械重推为
  **127 implemented / 22 missing**。

## Next Session

- [2026-09-22] 対空/阵形建模的前提未验证：阵形対空補正表能否从 main.js 解码出来没人查过，
  要做先做 spike；它还会让 `emukc_battle/tests/golden/*.txt` 全量重冻结。
- 审计集 013 的判据是「drift-check 接通后有没有被真的用起来」（该计划 README 第 50–55 行），
  要等一次真实的上游版本变动，不是现在动手。
- 联合舰队剩敌方也是联合的 5 个端点，卡在数据不存在。基地航空队、`gauge_type_e` 抓取、
  decoder 未解析的 id 集、VPS 计划重新验证仍是积压。
