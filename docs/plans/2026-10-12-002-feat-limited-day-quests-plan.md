---
title: "Limited-Day Daily Quests - Plan"
type: feat
date: 2026-10-12
status: implemented
execution: code
---

# Limited-Day Daily Quests - Plan

规格在 GitHub issue #54（问题、用户故事、范围都在那里）。这份计划是实施之后补写的：实现照 issue 的
Implementation Decisions 做，这里只记 issue 里没有的东西——读代码得到的证据、规格里需要解释的三处、
实施单元与结果。

## Problem

「敵空母を３隻撃沈せよ！」（Bd4，任务 211）与「敵輸送船団を叩け！」（Bd6，任务 212）在实机只在日期个位是
3/7/0、2/8 的日子出现，次日 05:00 重置。服务器不看日期：前置完成就出现，重置定在「下一个对应日的 05:00」，
一个任务会连着挂几天。任务的出现与重置都自己读系统时钟，没法用固定日期测。

## Evidence

- 码典里 `Daily3rd7th0th` 与 `Daily2nd8th` 各只有一个任务，就是 211 与 212，前置都是 201（Bd1，`Daily`）。
- `update_quests_impl` 只有三个调用点：`get_quest_records`、`quest_clear_and_claim_reward`、建档时的 `init`。
  issue 的 Further Notes 提到的「战后结算里的更新」不存在，战后只走 `observe`，它不重建任务树。
- **`KcTime::jst_next_day_0500(ts)` 并不只依赖 `ts`**：它取的是真实的「今天 05:00」。周常的
  `jst_next_monday_0500` 同样。月、季、年三个是只依赖 `ts` 的。
- 重置的比较是 `now > reset_time`，恰好 05:00:00 那一刻记录还在。

## Decision

1. **出现**：`Period::appears_on(day)`，只有两种限定日周期看日期个位，其余恒真。`reconstruct_quest_tree` 在
   前置检查之后调它，`day` 是 `KcTime::jst_quest_day(now)`（日本时间减 5 小时后的日期）。
2. **重置**：`Period::reset_time(start)`；两种限定日周期与 `Daily` 走同一个分支。
3. **时间参数**：`update_quests_impl`、`reconstruct_quest_tree`、`mark_quest_as_completed`、
   `ShouldReset::should_reset` 都接收 `now`。`Ctx` 上加 `get_quest_records_at` 与
   `quest_clear_and_claim_reward_at`，原来的两个方法取一次系统时间后转给它们，签名不变。
4. **老存档**不需要代码：旧记录按新的重置规则判断，过了次日 05:00 的在第一次任务更新时被删，之后按日子重建。

## 规格里三处需要解释的地方

- **「其它周期行为不变」与「重置时间与普通日常任务相同、测试用固定日期」不能同时照字面做。**
  `jst_next_day_0500` 改成了只依赖 `ts`（任务日的次日 05:00）。对 `start <= now` 的所有情形，结果只在一处与旧实现
  不同：开始时间早于**昨天** 05:00、而现在还没到今天 05:00 时，旧实现不重置，新实现重置。也就是「前天开始的日常
  任务在今天凌晨仍然挂着」这个既有偏差被修掉了。这是普通日常任务的行为变化，范围只有这一个边角。
- **周常没有动。** `jst_next_monday_0500` 仍然读系统时钟，有同样的偏差，也同样不能用固定日期测。不在 #54 的
  范围，留给另一份规格。本次的集成测试用过去的日期，周常记录在这些测试里不会被重置，断言也不涉及它们。
- **「05:00 消失」。** 比较改成 `now >= reset_time`，恰好 05:00:00 就重置，与「任务日在 05:00 换日」的出现判断
  对齐；否则那一刻会留着一条属于前一天的记录。所有周期都受这一个瞬间的影响。

## Scope

**不做**（issue 的 Out of Scope）：其它有出现条件的任务、换日对演习与远征的影响、客户端分页与类别展示、
删除 `emukc_time` 里两个不再被运行时调用的函数。

**标出而不动的**：`KcTime::jst_next_370th_day_of_the_month`、`jst_next_28th_day_of_the_month` 以及它们背后的
`NthDay` / `jst_next_nth_day_of_the_month` 现在只剩测试在调，去留由维护者决定。调试入口 `quest_add` 与建档的
`init` 仍用系统时间。

## Implementation Units

- **U1** `emukc_time`：`jst_quest_day`；`jst_next_day_0500` 改为只依赖传入时间。
  测试：固定时间戳下的次日 05:00 与换日（含跨月）。
- **U2** `emukc_db` `entity/profile/quest/mod.rs`：`Period::appears_on`、`Period::reset_time`，
  `should_reset(now)`。测试：三月与二月的出现日列表；三种日周期的重置时间相同。
- **U3** `emukc_gameplay` `game/quest/{update,record,mod}.rs`：Decision 1–3。
  测试 `tests/gameplay_tests/quest/limited_day.rs`，全部经 `Ctx` 的任务操作、用固定时间：各自的日子、30 与 31 日、
  二月末跨到三月 1 日 05:00、3 日 23 点接取后 04:59 还在而 05:00 消失、下一个对应日以未接取的状态重新出现、
  完成后下一个对应日重新出现、前置未完成时不出现、普通日常任务在同一时间序列里照常。
- **U4** 文档：`docs/solutions/architecture-patterns/quest.md` 的 Quest lifecycle，`PROJECT_MEMORY.md`。

## Risks

- **Bd1 自己是日常任务**，它的完成记录在次日 05:00 重置，所以对应日当天要先再做一次 Bd1，Bd4/Bd6 才出现。
  这与实机一致，但读测试时容易以为是缺陷。
- **`quest_add` 会绕过列表**：它直接插入进度记录，如果此前没有在当前时间取过列表，旧的完成记录还在，新的完成
  就记不上。真实流程里任务必须先经列表重建才存在，不会遇到；测试辅助函数为此先取一次列表。

## 实施记录（2026-10-12）

- U1–U4 做完，分支 `feat/limited-day-quests`。
- 把出现判断关掉后，集成测试里有三条失败，确认它们在检查这条规则。
- 两路审查（规范、规格）没有阻断性问题。审查后改的：`appears_on` 的参数改名为 `quest_day`；二月的用例拆成
  独立存档。没有改的、留作已知差异的：
  - 升级当天在非对应日已经建出的旧记录要到次日 05:00 才消失（issue 的 Implementation Decisions 就是这么定的，
    比用户故事 14 的字面宽一天）。
  - 重置只在取任务列表与领取奖励时发生；跨过 05:00 而没有刷新列表时，旧记录仍会累积进度。所有周期都如此，
    本次没有动。
  - 「进度清空」的用例只接取了任务、没有打出进度：记录是删掉重建的，状态回到未接取即可说明，但进度值本身
    没有区分力。
  - 建档的 `init` 与调试入口 `quest_add` 仍写系统时间，集成测试因此只用过去的日期。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings` 通过。
- `cargo test --workspace --no-fail-fast`：见 `PROJECT_MEMORY.md` 的 Last Session。没有 ignored 的测试。
- 没有用客户端跑：任务列表的报文形状没有变，变的只是哪些任务在列表里。
