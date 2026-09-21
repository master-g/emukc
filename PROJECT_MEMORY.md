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
- [2026-07-30] Cache-list validation accepts nonzero map start-source cells and readable slot-item expressions; final linear commit is `0395121`. Source: `crates/emukc_model/src/codex/map.rs`, `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs`.
- [2026-09-19] Update chain (`make update`) is three steps: `bootstrap --overwrite --force-update` →
  decode with `--sync-assets --sync-battle-assets --sync-resource-manifest` → `cache make-list --overwrite`.
  Verified on 6.3.2.1 and 6.3.5.0 (the latter with main.js fetched by hand instead of bootstrap).
- [2026-09-19] 6.3.4.1 → 6.3.5.0 的实测形态：battle 资产零变化，cache list 只多一行显式路径。
  结论是 `make-list` 默认策略会吸收 `cache_rules.json` 的新显式路径，不需要 `--manifest`。
- [2026-09-19] `obfuscator-io-deobfuscator` (ben-sb, v1.0.6) does NOT replace `main-decoder`: on 6.3.5.0 it left
  263,127 `_0x455a(0x..)` calls unevaluated and 0/55 battle protocol fields visible, in 304 s and 5.1 GB peak,
  vs our 0 calls / 55 fields in ~20 s. Its ControlFlowRecoverer and AntiTamperRemover never fire — KanColle's
  obfuscator.io config has no control-flow flattening or self-defending. Re-evaluating it is wasted work.
- [2026-09-19] Two `main.js` version axes: `kcs_const.js` `scriptVesion` (note the upstream typo) is the client
  script version and drives `out/version.txt` plus every synced asset's `scriptVersion`; `kcs2/version.json`
  holds per-subsystem asset versions and moves independently. A main.js-only release bumps the first, not both.
- [2026-08-26] Upstream drift: kcwiki empty equipment slots are `null`, not `false`; `version.json` nests a `resources` object (flattened in `parse_version_info`); webpack emits shorthand `ObjectMethod` factories (normalized in `module-graph.ts`); event area 62 is unlocked by default by design.
- [2026-09-18] `required_exp(99) == required_exp(100) == 1_000_000` by design (marriage unlocks Lv.100 at the same exp), so `exp_to_ship_level(1_000_000)` is 100 and unmarried callers rely on `min(cap)`. Not a bug. Source: `kc2/level.rs`, `game/ship/exp.rs` tests.
- [2026-09-19] `update_quest_progress_for_action` reads only quest progress rows and the codex (`quest/update.rs:174-244`), so it can run at any point inside a domain transaction; U8 moved it to just before `commit` with zero behavior change. Source: `.farm/deepen-u8-report.md`.
- [2026-09-19] `questlist` `api_tab_id` is the client tab bar (0,9,1,2,3,4,5 = all, activated, daily, weekly, monthly, oneshot, other); the client filters nothing itself. `api_label_type` keys the row label (1,2,3,6,7,101..=112). Source: `main.decoded.js` `DutyDataHolder`, `_createTab`.
- [2026-09-19] `SortieStore::with_profile_lock` is held by exactly five top-level entries: `start_sortie`,
  `next_sortie`, `sortie_battle_result`, `sortie_sp_midnight_battle` and `sortie_battle_impl`. None nests
  another, and nothing they call takes the lock, so a new sortie entry can take it without reentrancy risk.
  `sortie_midnight_battle` stays unlocked on purpose: it mutates an existing pending session, not `active`.
- [2026-09-19] `apilist.md` is mechanically aligned with the router: 113 implemented, 34 missing, no overlap.
  Re-verify by extracting `nest("/prefix", mod::router())` from `kcsapi/mod.rs` plus each submodule's
  `.route("/leaf"` and diffing against the two fenced blocks; do not hand-audit it.
- [2026-09-19] Sunk-enemy quest events come only from `settle_sortie_battle_impl`'s `final_enemy_nowhps`
  (the post-night session packet), the same slice as `api_dests`. The snapshot's own day-frozen
  `enemy_nowhps` copy swallowed night-only sinks and is deleted. Source: `game/sortie_result.rs`.
- [2026-08-26] `GetOption::new_remote_only()` now really bypasses local cache: `fetch_from_remote` skips its local dedup check when `enable_local` is false.
- [2026-09-20] The DB layer uses SeaORM as a struct<->row mapper with DDL generation, not as an ORM:
  0 SQL joins, 0 `group_by`, 0 `find_with_related` across 195 `Entity::find()` sites. `Relation` exists
  only so `create_table_from_entity` emits FOREIGN KEY (`entity/mod.rs:9-13`). Swapping in sqlx or
  rusqlite would rewrite ~450 call sites to shed 25 crates; sqlx is 133 of the 159-crate subtree.
- [2026-09-20] A full `cache populate` over the generated list IS the authoritative CDN existence probe: it
  requests every non-hole path, so its failure list is an exhaustive answer for "what is missing upstream".
  Use it to maintain the hole tables; `--greedy` is not a substitute (see Pitfalls).
- [2026-09-20] Hole tables flow Rust -> asset, not decoder -> Rust: `main-decoder/src/path-rules.ts` parses
  `EVENT_SHIP_HOLES` / `BTXT_FLAT_IDS` / `CHARACTER_HOLES` back out of the Rust sources into
  `cache_rules.json`. To change a hole, edit the Rust constant, then `make decode-main` to re-sync.
- [2026-09-20] Nothing in `start2` distinguishes the 5 resupply-form ships (743/744/745/748/749, names
  ending in 補) from normal friendly ships — checked `api_sortno`, `api_backs`, `api_aftershipid` and
  `ship_picturebook.json`. Same for friend-fleet graph ids 6299/6301/6303. Do not re-hunt for a rule.
- [2026-09-21] 在 `919ca9c` 上 `cargo test --workspace` 全绿（0 failed / 0 ignored），
  前提是先 `mkdir -p target/tmp`（`test_font` 仍需要它）。09-20 记的另外两条 baseline 失败
  （`api_alignment_e2e`、`real_manifest_parses_with_passthrough_fields_none`）已不再复现，
  不要再当既有失败引用。

- [2026-09-21] `emukc_network`'s download layer already reads the whole body before opening the
  destination and errors out on any non-2xx, so a failed transfer never wrote a partial or corrupt
  file. The `.part` + rename added by plan 005 only closes the truncate-to-copy window; the real
  data loss came from `bootstrap --force-update` deleting the files before the download ran.
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
- [2026-09-21] drift-check 现在跟踪 13 个资产（4 battle + 2 map-catalog + 7 cache-list 输入），
  基线已 `--accept` 到 6.3.5.0，入口是 `make drift-check` / `make drift-accept`，`make update`
  在 decode 后打一份不阻断的报告。指纹按规范化后的字节算，所以 `_0x` 重命名会算作漂移，
  即使解码知识没变——这是「收敛版本记录」要解决的噪声来源。

## Failed Attempts / Pitfalls

| Pitfall | Source |
| --- | --- |
| populate 基准三坑：`head -200` 清单全是 mp3（带宽受限，把收益掩成 10%，全量实为 49% mp3 + 49% png，要随机抽样）；`Kache::build()` 要求 `cache_root` 已存在，否则 exit 1 且 stdout 无输出；配置里的相对路径按 config 文件所在目录解析。 | 计划 003「实测结果」(2026-09-21) |
| Seeded test RNG left thread-local entropy set → cross-test pollution. Must restore entropy after seeded runs. | git `66f8317` |
| Cache downgraded to an older local file on version rollback instead of serving the newer local copy. | git `185c0b8` |
| `remodel()` dropped fields + faulty boiler query (logic error). | `docs/solutions/logic-errors/remodel-preserve-fields-and-boiler-query-2026-05-14.md` |
| Clippy warning triage across the workspace. | `docs/solutions/best-practices/resolve-clippy-warnings-triage-2026-05-28.md` |
| Grepping `test result:` to verify tests misses failures — a FAILED target prints its own line that is easy to lose among many suites. Check the cargo exit code instead. | plan 004 U5 (2026-06-22): reported "821 passed" while 3 `sortie_battle.rs` tests were failing |
| `cargo clippy` default ≠ `-D warnings`: the default run missed a `match`→`let-else` lint. CLAUDE.md gates on `-W warnings`; use `-D warnings` for final verification. | plan 004 U7 (2026-06-22) |
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
| [2026-09-18] `clippy -- -D warnings` fails on old `result_large_err` at `emukc_network/src/download.rs:236` and (rustc 1.98.1) `src/bin/net/auth.rs:139`, both older than plan 002. Repo gate is `-W warnings`; touched-file checks give `-D` strength for new code. | sessions 2026-09-18, 09-19 |
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

| [2026-09-21] 「无 .data 跑测试」时备份必须放到**仓库外**，且移回前先删掉重建出来的空壳。
cache rules 的测试（`loader-rules-*`、`kcs-rules-*`）会 `create_dir_all(".data/tmp")`，
`mv .data .data.bak` 之后再移回就把备份塞进了新目录，两轮嵌套两层。正确做法：
`mv .data ../.data-bak` → 跑 → `rm -rf .data`（此时只剩 tmp）→ `mv ../.data-bak .data`。 | session 2026-09-21 |

## Last Session

- [2026-09-21] 五件事全部推送：积压 11 个提交推掉；`bootstrap` 裸跑中止修复
  （`06e57a6` + `919ca9c`）；本文件压到 21.7 KB（`217fe6f`）；空 200 的 40 次重试
  （`44ca6ba`）；drift-check 接通（`617cfca`）。
- `Codex::save` 13 个输出里只有 `start2.json` 是 warn+skip，其余 12 个返回 `AlreadyExist`，
  所以第二次裸跑必死在 Phase 3。全部改成 warn+skip，`CodexError::AlreadyExist` 随之删除。
  顺带修掉 `needs_quiet_stdout` 让失败的 bootstrap exit 1 且两个流全空。
- 空 200：`fetch_from_remote` 把每个镜像的 `InvalidFile` 折叠成 `FailedOnAllCdn`，
  被读作「没有镜像应答」。改成「全部镜像都应答且都不可用 → `InvalidFile`」并归入 missing 类。
- drift-check：013 选的是「先接通，再谈收敛」。跟踪集 6 → 13，基线 6.3.0.0 → 6.3.5.0，
  接进 Makefile 三个入口，两份 solutions 文档同步更新。
- 门禁每次跑满 fmt / clippy（恒 6 条既有 `result_large_err`）/ `cargo test --workspace` 全绿；
  三次真实验证：裸 `bootstrap` exit 0、单条清单 populate 归入 `*.missing.nedb` 零重试、
  `make drift-check` exit 0 报 no drift。

## Next Session

- [2026-09-21] 审计集 013 走的是「先接通，再谈收敛」：drift-check 已接通并刷新基线，
  但**四份版本记录仍未收敛**，013 的设计问题原样留着。下次要判断的是：接通之后
  它是否真的被用起来了（`make update` 的报告有没有人看、基线有没有第四次被绕过），
  再决定收不收敛。原计划文档见 `docs/plans/2026-09-20-bootstrap-cache-audit/013-*.md`。
- 收敛要处理的两条裂缝没变：同一个上游字段 `scriptVesion`（上游拼错）被
  `main-decoder/src/io.ts` 和 `make_list/source/kcs2/plain.rs` 各用一个正则解析，
  两边不一致时 012 的校验会误报；指纹按字节算，`_0x` 重命名会产生噪声漂移。
- manifest 差集里约 7%（估 1,500 条）是真实存在的资源，Rules 清单漏了它们（抽样命中 `banner_dmg`）。
  正确补法是拿差集做一次性存在性探测并入规则，不是复活 Greedy 枚举。尚未立计划。
- 在 `*.missing.nedb` 被用来喂 `EVENT_SHIP_HOLES` / `ALBUM_STATUS_HOLES` 之前，先跨镜像确认。
- 009 里明知而未修：`tokio::fs::write` 非原子；两个并发 populate 对同一清单互相覆盖无锁；
  `FailureKind::Rollback` 自 `185c0b8` 起不可达，但 BOOTSTRAP.md 仍写着它的日志行。
- 更早的积压未变：14 个 `api_req_combined_battle/*` 然后基地航空队（EO74 字段规格见
  sinsinpub/kcs2-assets `api_info/apilist.txt`，已过时）；`api_alignment_e2e` 的 `600..700`
  与 codex 的 61-65 号 id 矛盾；`gauge_type_e` 抓取；decoder 未解析的 id 集；VPS 计划需重新验证。
