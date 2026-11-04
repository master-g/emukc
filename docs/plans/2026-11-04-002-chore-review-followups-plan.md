---
title: "Review Follow-ups - Plan"
type: chore
date: 2026-11-04
status: implemented
execution: code
---

# Review Follow-ups - Plan

2026-11-04 审查中，9 个已确认的 bug 由 `2026-11-04-001-fix-review-bugs-plan.md` 修复，另有 13 处所有权检查缺口在 `fix/ownership-checks` 中补上。#307 之前写进别的 profile 船坞的旧行无需迁移，由 `fix/stale-repair-dock` 确认会自行清掉，并修好船坞里舰船已不存在时母港报错的问题。本计划处理审查剩下的发现：先是请求路径上会出错的地方，再是类型与校验，最后是机械清理和 lint 收紧。

**基线**：`fix/ownership-checks` 合并之后的 `main`。
**规则**：每个 P 单元单独一个 PR，标题用 Conventional Commits；能写出失败测试的单元，测试先单独提交；每个 PR 都过 fmt、严格 clippy、`cargo test --workspace`，并在说明里写出 golden 是否变化。
**非目标**：不引入 ID 新类型（所有权检查已在查询层完成）；不拆分 `GameplayError`（需要先统计调用方按哪些变体分支，再决定）；不动 `emukc_internal::prelude` 和 glob 重导出（项目约定）。

## 顺序

P1–P4 修的是运行时会出错的地方，按顺序先做。P5–P7 是类型与校验。P8、P9 是清理，可以随时穿插。P10 需要先拿到运行时证据，再决定改不改。

## 进展

每个单元都已开 PR，都还没合并。每个 PR 都过了 fmt、严格 clippy 和完整测试，0 失败 0 忽略；P5、P7 的 golden 不变。

| 单元 | PR | 说明 |
|---|---|---|
| P1 | #309 | 游戏 API 仍是 HTTP 200 加 `api_result` -1；客户端不读 `api_result_msg`。 |
| P2 | #310 | `From<Model> for QuestProgress` 没有调用方，改成 `TryFrom`。 |
| P3 | #311 | 失败路径的原子性由写法保证，macOS 上造不出写到一半失败。 |
| P4 | #312 | 连接池只有 1 个连接，以前一次登录挡住所有数据库请求。 |
| P5 | #313 | 真实的竞态是夜战或归港与结算之间，不是两个相同请求。母港重复入账在单连接下复现不了，照样修了。 |
| P6 | #314 | 购买上限 10 取自客户端 `Math.min(10, this._count)`。 |
| P7 | #315 | 测试用 `From<BattleShipInput>` 改成造友舰（原来造敌舰，与文档不符）；`is_sortie` 仍是布尔。 |
| P8 | #318、#319 | 拆成两个 PR。抑制属性实际 141 处，缺 `Debug` 的公开类型 14 个。 |
| P9 | #316 | 与 #319 都改 `auth.rs` 和支付回调的 `st` 字段，后合的要解冲突。 |
| P10 | #317 | 编成跨舰队移船、HTTP/1 重定向主机是 bug，已修；SQLite、锁表不改，见下节。 |

## Implementation Units

### P1 HTTP 错误的状态码与日志

- `src/bin/net/err.rs`：`From<UserError>` 去掉 `_ => Unknown`，用户造成的错误（密码错、用户名已占用、用户名或密码太短）映射成 400、401 或 409；`From<GameplayError>` 中的 `WrongType`、`Insufficient`、`CapacityExceeded` 改为 400。
- `src/bin/net/resp/mod.rs` 的 `KcApiError::into_response` 用 `tracing::error!` 记录 5xx；`Db(e)` 给客户端返回固定文案，不再带数据库原文。
- 验证：为每个映射写 handler 测试，断言状态码与 `api_result`。

### P2 请求路径上对数据库值的 unwrap

- `game/compose/powerup.rs:129`、`game/incentive.rs:102`：`codex.find(..).unwrap()` 改成 `?`（`ManifestNotFound`）。
- `emukc_db/src/entity/profile/quest/progress.rs:150`：`From<Model> for QuestProgress` 改为 `TryFrom`，`serde_json` 的错误向上传递；同时改调用方。
- `emukc_crypto/src/password.rs:16`：`bcrypt::verify(..).unwrap()` 出错时按密码不匹配处理并记日志。
- `game/practice.rs:338`：`UserHQRank::n(..).unwrap()` 改为返回错误。
- 验证：每处构造一份坏数据，断言请求返回错误而不是 panic。

### P3 缓存下载的原子写入

- `emukc_network/src/download.rs:364-386`：先写同目录下的 `.partial` 文件，写完再 rename 到目标路径，写入改用 `tokio::fs`。`emukc_cache/src/history.rs:60` 已经是这种写法，照着做。
- 验证：测试中让下载在写入途中失败，断言目标文件保持原样、不留下残缺文件。

### P4 bcrypt 移出事务

- `gameplay/src/user/account.rs:71,113,242`：在 `begin()` 之前用 `spawn_blocking` 算哈希或校验。
- 验证：现有账号测试；另测一次登录耗时，确认事务不再包住 bcrypt 计算。

### P5 战斗 store 的剩余竞态

- `sortie_midnight_battle`、`practice_midnight_battle` 对 store 做取出、修改、写回，期间不加锁。`PracticeStore` 照 `SortieStore` 加 `with_profile_lock`，夜战和结算都在锁内执行。
- `view/port.rs:79,109`：两次并发的母港请求都会把资源点收益记上一遍。改为先在锁内取出收益，失败时放回。
- `sortie_goback_port`、`clear_sortie_state_if_any` 放进锁内。
- 验证：用 `tokio::join!` 发两次相同请求，断言只结算或记入一次。

### P6 输入校验

- `social/payment_create.rs`：`count` 加上限，避免 `price * count` 与库存累加溢出。
- `api_req_kousyou/destroyship.rs`：先用 `form_utils::deserialize_form_ivec` 解析全部 id，再执行解体。
- `game/map.rs:388`：用 `i32::try_from(rank)` 代替 `rank as i32`。
- `entity/profile/fleet.rs:180`、`entity/profile/mod.rs:343`：`ALTER TABLE` 只忽略列已存在的错误，其余错误照常返回。
- 验证：每处补一个边界输入测试。

### P7 用类型表达领域

- `SortieBattleResultSnapshot.win_rank` 从 `String` 改为 `KcSortieResultRank`，删除 `parse_sortie_result_rank`，只在生成 API 响应时转成字符串。
- `MaterialCategory::from_id` 改为返回 `Option`，调用方处理未知 id，不再默认当作燃料。
- `BattleRuntimeShip::new(input, bool, bool)` 改用 `Side` 枚举或两个构造函数；测试里的 `From` 实现目前把两个布尔值传反了，一并改正。
- `CellItem` 用一个 `kind()` 枚举取代两个互斥的 `Option` 方法。
- 验证：战斗相关改动跑 golden，必须逐字节不变。

### P8 机械清理与收紧 lint

- 132 处 `#[expect]` 或 `#[allow]` 补 `reason`，然后在 `[workspace.lints.clippy]` 开启 `allow_attributes_without_reason`。
- 24 个公开类型补 `Debug`（`crypto/port_api_key.rs`、`suffix_utils.rs` 手写实现，把密钥打码），然后开启 `missing_debug_implementations`。
- `&Option<T>` 改成 `Option<&T>`，`&PathBuf` 改成 `&Path`（17 处），然后开启 `clippy::ref_option`。
- `cargo doc --workspace --no-deps` 的 83 条告警，大多是 `api_foo[0]` 没加反引号。
- `emukc_time` 里的 15 处 `FixedOffset::east_opt(9 * 3600).unwrap()` 合并为一个 `const JST`。
- 删除空的 `emukc_macros` crate，以及没有用到的 `async-trait`（`emukc_db`）和 `env_logger`（根 crate）依赖。
- 验证：每开一条 lint 单独提交，严格 clippy 通过。

### P9 二进制 crate 的小问题

- `cli/mod.rs:123-127`：clap 已经读过 `EMUKC_CONFIG`，这段使 `:129` 的错误提示永远到不了，删掉。
- `cli/dev/new_session.rs`：`open::that(url).unwrap()` 改成记日志；输出的 URL 按 TLS 配置选 http 或 https，与 `auto` 一致。
- `social/payment_create.rs:57-58`：`name`、`description` 改用主数据的值，不再用客户端传来的值（模板关闭了自动转义）。
- `net/auth.rs:93-135`：删除没有用到的 `FromRequest` 实现；`cli/mod.rs:254` 的 `_ => Ok(())` 改为列出 `Version`。
- `cancel_payment.rs`、`confirm_payment.rs` 中只为消除 `dead_code` 存在的 `st` 字段删掉。

### P10 先取证据再决定

- SQLite 没有设置 `busy_timeout` 和 WAL（`emukc_bootstrap/src/db.rs:47`）。先用并发请求复现 `SQLITE_BUSY`，复现得了再改。
- `router/game.rs:41,142`：HTTP/1 下 `uri.authority()` 为空，重定向可能指向 `localhost:8080`。先用 `make serve` 实际请求一次，确认后再修。
- `api_req_hensei/change.rs`：把另一支舰队里的船编进当前舰队时，原舰队似乎不会移除这艘船。先写 handler 测试确认。
- `download_lock.rs:24`、`sortie_store.rs:139` 的锁表只增不减。先估算长期运行的条目数，再决定是否清理。

## 验证

- 每个 PR 都过 fmt、严格 clippy、`cargo test --workspace`，并报告跳过或忽略的测试。
- P1、P2、P6 中涉及 handler 的改动，再跑 `make headless-check SCENARIO=fresh_1_1`。
- P7 必须保持 golden 不变。

## 执行中的新发现

都没修，按需另开单元。

- **SQLite 跨进程写锁。** `.begin()` 是延迟事务。服务器运行时跑会写库的 CLI（`new-session --no-start`、`add-quest`、`import-account`），先读后写的事务立即报 `database is locked`。WAL 无效；`BEGIN IMMEDIATE` 有效，要给约 130 处 `.begin()` 包一层。sqlx 默认 busy_timeout 是 5 秒。
- **演习对手刷新。** `practice.rs:303` 的判断在日本时间 15:00 之后（以及 03:00 之前）每次请求都成立，对手每次都重抽。
- **错误映射。** `picture_book.rs` 的非法 `api_type`、`charge.rs` 的非法 `api_kind`、`RewardError::ChoicesLengthMismatch` 是客户端错误却是 500；地图未解锁（`Locked`）是 404；演习对手段位损坏走 `WrongType`，变成 400 且不记日志。
- **鉴权。** 游戏 API 的 token 无效时返回 HTTP 401，真实服务器返回 `api_result` 201；`auth_middleware` 把数据库错误也折成 401。
- **store 顺序。** `practice_battle` 在 `tx.commit()` 之前写入待结算的战斗。
- **`ALTER TABLE`。** `preset/mod.rs:22` 仍吞掉所有错误；`airbase/mod.rs:27` 用 `is_ok()`，任何错误都跳过重置。
- **批量解体。** 中途遇到不属于自己的船时，前面的船已经解体；没有整批一个事务的接口。
- **小问题。** `auto/mod.rs` 仍有 `open::that(url).unwrap()`；`net/mod.rs:85` 是第三处 TLS 判断；`find_config_file` 留着 `exe_path.parent().unwrap()`；`quest/mod.rs:224`、`quest/update.rs:232` 的 `serde_json::to_value(..).unwrap()`；`src/bin/cfg/mod.rs:14` 的 `CFG` 写入后没人读；`emukc_crypto` 的 `hash.rs` 两个测试钉着该 crate `Cargo.toml` 的 md5。
- **可能没用的依赖。** `cargo machete` 标出根 crate 的 `axum-extra`、`emukc_cache` 的 `emukc_db` 与 `emukc_model`、`emukc_bootstrap` 的 `futures-util` 与 `scraper`、`emukc_db` 的 `serde`，未逐个确认。
- **死代码（只标注）。** `emukc_log` 的 `#[macro_use] extern crate tracing`；`damage.rs` 的 `calculate_single_slot_airstrike_damage`；`res.rs` 的 `ResourceCategory`；`nuke.rs` 里注释掉的 `add_ship_incentives`；`cdn_operations.rs` 的 `setup_test_cache`。
- **测试工具。** `make headless-check SCENARIO=fresh_1_1` 偶尔在标题画面触发 300 秒看门狗；P4 的登录测试比较两段耗时，机器很忙时可能偶发失败。
