---
title: "Weekly Quest Reset Without the Clock - Plan"
type: fix
date: 2026-10-13
status: implemented
execution: code
---

# Weekly Quest Reset Without the Clock - Plan

Issue #101。周常任务的重置读系统时钟，固定时刻的测试跨不过周一；重置规则也没有写下来。

## Evidence

- `Period::reset_time` 对周常调 `KcTime::jst_next_monday_0500(start_time)`，而它用 `jst_monday_0500_utc()`
  （读系统时钟的「本周一 05:00」）与 `start_time` 比：早于它就返回它，否则返回它加一周。
  - 记录在过去的某一周：返回的是**今天所在周**的周一，按固定时刻走到那一周的周一时不重置。
  - 记录在未来：返回今天所在周的下周一，早于记录本身，立刻重置。
  - 用真实时钟时两种情况都碰不到。真实时钟下只有一处变了：日本时间周一 00:00–04:59，一条一周以前的旧记录
    原先要等到 05:00 才重置（「本周一 05:00」还没到），现在当场重置，因为它自己的下一个周一早就过了。
- 日常一侧已经只看传入时刻：`jst_next_day_0500`、`jst_quest_day`，换日在日本时间 05:00。
- `quest_add` 给记录盖的是 `Utc::now()`；`init`（建档、重置存档时）也是。

## 规则

周常任务的一周从日本时间周一 05:00 到下周一 04:59，与任务日同一个换日点：周一 04:59 还算周日，也就还算
上一周。一条记录（进行中的或已领取的）在它开始之后的第一个周一 05:00 重置；正好在周一 05:00 开始的，
持续到下一个周一 05:00。

## Implementation Units

- **U1** `jst_next_monday_0500` 只由传入时刻算：取任务日，加到下一个周一，05:00。
- **U2** `quest_add_at`，`quest_add` 读一次时钟后转给它。
- **U3** `quest.md` 的 Quest lifecycle 写下周常规则，删掉「仍读系统时钟」那句。

## 没有做

`init` 仍读时钟：建档的时刻就是当下，要传时刻得给 `new_profile`、重置存档各加一个 `_at` 入口，只为测试。
它建出的记录带真实时间；固定时刻的测试用 `quest_add_at` 与 `_at` 入口造自己的记录，不靠建档时那批。

`jst_monday_0500_utc` 等读时钟的函数还在 `KcTime` 上，任务一侧不再用它们。
