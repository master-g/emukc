---
title: "Quest Reset Months and Dropped Rewards - Plan"
type: fix
date: 2026-10-14
status: implemented
execution: code
---

# Quest Reset Months and Dropped Rewards - Plan

Issues #112、#113。季常与年常任务在错误的月份重置；领奖时家具、战果、远征补给解锁被吞掉。验收条件在两个
issue 的 Agent Brief 里，这份计划补证据、决定和实施顺序。

## Evidence

- **#112** `Period::reset_time`（`emukc_db/src/entity/profile/quest/mod.rs:106`）：季常调
  `KcTime::jst_next_quarter_day_one_0500`（`emukc_time/src/lib.rs:294`，按 1/4/7/10 月），年常调
  `jst_next_year_day_one_0500`（`:331`，一律 1 月 1 日）。两者都只看日历月份，日本时间重置日 00:00–04:59 会跳过
  当天 05:00。`jst_next_1st_day_of_the_month`（`:233`）的写法是对的：先算本期 05:00，`start` 早于它就返回它，
  否则返回下一期。
- `ShouldReset` 是对 `HasTimestampAndPeriod` 的泛型 impl，`periodic::Model` 与 `progress::Model` 都实现。
  调用点只有 `game/quest/update.rs:44`（完成记录）与 `:59`（进行中记录），两处都有 `codex`。
- 年常的重置月在码典里：`Kc3rdQuest::label_type` = 100 + 月份。55 条年常的分布：101×4、102×7、103×5、104×2、
  105×5、106×11、107×5、108×1、109×6、110×5、111×3、112×1。KC3Kai `QuestManager.js` 的
  `yearlyMay`、`yearlySep` 清单与 `label_type` 一致，**只有 928 不一致**（`label_type` 107，KC3Kai 放在
  `yearlySep`）。
- `KcTime::jst_0500_day_one_of_quarter`（读时钟）只在 `emukc_time/tests/test.rs:60` 被调用，与任务无关，不动。
- **#113** `apply_single_reward`（`game/quest/mod.rs` 约 486 行）的 `match` 以 `_ => {}` 结尾；选择奖励与附加奖励都
  走它。领奖响应（`thirdparty/quest/reward.rs`）照常把家具、战果列给客户端，所以客户端以为拿到了。
  - `add_furniture_impl`（`game/furniture.rs:130`）遇到已拥有的家具直接返回已有记录，不报错。
  - `profile.war_result` 存在（`api_war_result` 从它出），没有任何写入点。
  - `profile.extra_supply_expedition` 默认 `false`，没有任何写入点；`api_extra_supply[0]` 从它出。
  - `unlock_large_construction_impl`（`game/basic.rs:383`）是改 profile 一个字段的现成写法。

## Decision

1. **#112 月份来源**：调用点从码典取，`Period::reset_time(self, start, yearly_month: u32)`，只有 `Annually` 读它；
   `ShouldReset::should_reset(now, yearly_month)` 跟着加参数。`update.rs` 两处按 `quest_id` 查
   `codex.quest`，`label_type` 在 101–112 时取 `label_type - 100`，否则（包括码典里已经没有的旧任务）取 1，
   也就是现在的行为。不往数据库加列：月份是码典事实，存进行里会在码典更新后过期。
2. **928 以 `label_type` 为准**（7 月）：`label_type` 是官方服务器发给客户端的标签栏归属，KC3Kai 的清单是人工
   维护的。列为已知差异，不加特例。
3. **季常/年常两个函数照 `jst_next_1st_day_of_the_month` 的写法重写**：算出 `start` 所在期的起点（季常为 3/6/9/12
   月中不晚于 `start` 所在月的最近一个，年常为指定月），转成该日 05:00 JST；`start` 早于它就返回它，否则返回
   下一期的同一时刻。只依赖传入时刻。
4. **#113**：`match` 写全所有 `Kc3rdQuestRewardCategory`，不留 `_`。
   - `Furniture` → `add_furniture_impl(c, profile_id, reward.api_id)`。
   - `WarResult` → 新 `add_war_result_impl(c, profile_id, amount)`，放在 `game/basic.rs`，照
     `unlock_large_construction_impl` 写。
   - `ExpeditionSupplyUnlock` → 新 `unlock_expedition_supply_impl`，置 `extra_supply_expedition = true`。
   - `FactoryImprovementUnlock`、`AirbaseUnlock` → 空分支，注释写明原因（改修工厂服务端不设开关；
     航空队由 `ensure_airbases_impl` 按海域开放）。
   - `Material` 等已有分支不动。

## Implementation Units

- **U1 #112 时间函数**：`jst_next_quarter_day_one_0500` 改为 3/6/9/12 月；`jst_next_year_day_one_0500` 加
  `month` 参数（或另起一个函数，旧函数无其他调用方时删掉）。`emukc_time/tests/test.rs:204`、`:214` 的断言按新
  规则改，并补 brief 里列的边界时刻。
- **U2 #112 接线**：`Period::reset_time`、`ShouldReset` 加参数；`update.rs` 两处取月份；
  `emukc_db` 里 `Period` 的单元测试补季常与年常。集成测试放 `tests/gameplay_tests/quest/`（可并入
  `weekly_reset.rs` 的写法，另起 `periodic_reset.rs`），用 `quest_add_at` 与 `get_quest_records_at` 造固定时刻：
  季常 2/10 开始 3/1 05:00 消失、3/1 04:59 开始 05:00 消失；一条 `label_type` 105 的年常 4/30 开始 5/1 05:00 消失、
  6/1 开始挺到次年 5/1。
- **U3 #113**：奖励分支与两个 `_impl`。测试：246 加家具 183；已拥有家具再领不报错；284 使 `war_result` +80；
  427 后 `api_extra_supply` 首位为 1（经 `get_member/basic` 或 `require_info` 的视图函数读）；146 只发其余奖励。
- **U4 文档**：`quest.md` 的 Quest lifecycle 写季常/年常规则与 928；Quest rewards 一节补三类奖励与两个空分支。

U1→U2 有依赖；U3 独立。

## Risks

- **`ShouldReset` 签名变更**会碰到所有调用方；现在只有 `update.rs` 两处，若实施时多了，逐个补上月份。
- **升级当天的旧记录**：季常记录按旧规则可能挂到 4/1；新规则下在下次取列表时按 3/1 判断，过期的立即删除重建。
  这是修正，不是回归。
- **427 已领取的存档**：旧存档里 427 领过但标志仍是 `false`。不补发，记入已知差异；需要时由用户手动重领
  （调试入口）。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test --workspace --no-fail-fast`，报告 ignored 数。
- 不需要客户端演示：`api_extra_supply` 已有输出路径，只是值从 0 变 1。
