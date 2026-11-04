---
title: "Review Bugs - Plan"
type: fix
date: 2026-11-04
status: implemented
execution: code
---

# Review Bugs - Plan

2026-11-04 按 rust-best-practices 审查全仓库，确认 9 个 bug。本计划只修这 9 个；其余发现见 `2026-11-04-002-chore-review-followups-plan.md`。

**基线**：`main` @ `29889292`（#305 之后）。fmt、严格 clippy 通过。
**非目标**：不引入 ID 新类型，不拆 `GameplayError`，不改 HTTP 错误码映射（都在后续计划里）。

## 事实

- `slot_exchange_impl` 对 `[_; 5]` 做 `swap(idx as usize, ..)`，下标原样来自 `api_src_idx`/`api_dst_idx`。
- `set_slot_item`、`set_exslot_item`、`unset_all_slots`、`slot_exchange`、`find_ship` 不带 `profile_id`，`_impl` 用 `find_by_id` 取舰船与装备。`toggle_ship_locked_impl` 已有归属检查，返回 `EntryNotFound`。
- `consume_cond_use_item_impl` 不分 `use_type` 先各扣一个間宮、伊良湖；`use_type == 2` 时 `choose_iter(ships.iter().skip(1)).unwrap()`，旗舰单舰时迭代器为空。
- `map_pipeline::sources::download_stat_json` 在同步函数里 `Runtime::new().block_on`，调用链是 `cli/bootstrap.rs` 的 `async fn` → `parse_partial_codex` → `load_stat_catalog`。
- `cli/dev/new_session.rs`、`cli/auto/mod.rs` 写死 profile 1；`start_game` 按账号与 profile id 同时过滤。
- `quest/update.rs` 对进行中的任务 `codex.find::<Kc3rdQuest>(..).unwrap()`；同文件 `yearly_month` 已处理码典里没有的任务。
- `social/confirm_payment.rs` 先 `get` 再 `take().unwrap()`，`add_pay_item` 的错误被丢弃且会话已消耗。
- `sortie_battle_result` 在事务前取出待结算结果；`sortie_battle_impl`、`sp_midnight_battle_impl`、`practice` 在 `tx.commit()` 前写 store。

## Implementation Units

每个单元一个提交，能写出失败测试的先单独提交测试。

### U1 换装备栏下标越界

- `slot_exchange_impl` 两个下标都校验在 `0..5`，否则 `WrongType`。

### U2 改装与舰船查询校验归属

- 上述五个 `Ctx` 方法与对应 `_impl` 加 `profile_id`，舰船与装备都按 `ProfileId` 过滤，别人的当作不存在。改 `kaisou` 四个 handler 与 `ship2`、`ship3`、`ship_deck`，测试调用点随签名迁移。
- 排查同类查询，又发现 13 处缺口（解体、改造、结婚、强化、装备转移、装备预设、补强增设、编成变更、入渠；入渠把船坞编号当主键用，第一个之后的玩家都在用第一个玩家的船坞）。这部分放在叠加的 `fix/ownership-checks` 里单独审查；同时修了编成变更里旗舰换位后同一艘船出现两次的问题。

### U3 給糧艦

- 按 `use_type` 扣：1 扣間宮，2 扣伊良湖，3 各扣一个。伊良湖在旗舰单舰时只给旗舰。

### U4 stat.json 回退下载

- 下载放到独立线程上的独立 runtime，不再在调用方的 runtime 里 `block_on`。

### U5 CLI 会话的 profile

- 新增 `Ctx::account_profiles(access_token)`；`new-session`、`auto` 用账号自己的 profile，没有就新建。

### U6 码典缺任务

- 进行中的任务在码典里找不到时跳过重算，不 panic。

### U7 确认付款

- `PaymentStore` 一次加锁完成「属于该 profile 才取出」；`confirm`、`cancel` 用它。加道具失败时记日志并放回会话，可重试。

### U8 store 与提交顺序

- 先提交，再改 store：取出待结算结果、写入待结算结果、推进当前格都放到 `tx.commit()` 之后。

## 验证

- 每个单元的新测试先红后绿（U2、U5 的签名变化使测试只能与修复同提交）。
- fmt、严格 clippy、`cargo test --workspace`；golden 不应变化。
- U4、U5 另在 CLI 上手工复现与复验。U4：`map build-overlays` 在去掉 `stat.json` 的数据目录上 panic，修复后下载成功，文件与本地缓存一致。U5：第二个账号 `new-session` 报 `Profile not found`，修复后每个会话都对应账号自己的 profile。
- U7 并发取出没有能稳定失败的测试，原子性由同一把锁内完成检查与删除来保证。
