---
title: "Month-End Limits of 戦果 and EO Gauges - Plan"
type: feat
date: 2026-10-15
status: implemented
execution: code
---

# Month-End Limits of 戦果 and EO Gauges - Plan

Issue #147 的剩余部分：月末截止时间（戦果 21:59、1-6 プレゼント箱 23:59）。前一半见 plan
`2026-10-15-005`。

**基线**：`main` `c8b70978`，分支 `feat/war-result-month-end`。

## Evidence（只列读过的页面）

- **戦果月在月末最后一天 22:00 JST 结束**，提督经验折算的部分（素戦果）也一样：
  - zekamashi.net `kancolle-kouryaku/senka-500`：「提督経験値(素戦果)は、先月末の22:00~戦果月の月末22:00迄に稼いだものが有効です」。
  - sftsyr.hatenablog.jp `2018/12/03/024651`：「ただし最終日のみ「22:00」が集計締め切りとなり、22:00以降に取得した戦果は翌日1日の更新分で集計されます」。
  - dsco11.web.fc2.com `kancolle-deadline.html`：「通常海域、EO戦果」一行的截止是 `getVeteransLimit` = 最后一天 22:00。
- **22:00 以后破 EO ゲージ得到的戦果作废**，两个月都不算；勲章（1-6 是プレゼント箱）到 23:59 照给：
  - zekamashi.net `senka-500`：「戦果を意識する場合、月末の22:00迄にクリアする必要があります。月末の23:59までは勲章をもらえますが、戦果消失します」。
  - blog.makapy.com `archives/8725`：「22時以降のEOゲージ破壊分の戦果は、翌月分に引き継ぎもされないようなので要注意です」。
  - 分歧：chiebukuro `q10221262190` 最佳答案说「EO海域の戦果は消滅してしまいます」，同页另一回答说「翌月の戦果に引き継がれます」。取多数。
- **EO ゲージ在 1 日 00:00 JST 恢复**（现在代码是 1 日 05:00）：
  - sakamoto-kouji.com `2019/08/04/kancolle-eo-kousin-zikan`：「EO更新時間と勲章が貰えるのは「月末23:59:59」」。
  - chiebukuro `q10146468100`：「0時を過ぎた時点で勲章などの報酬は貰えなくなります」。
  - dsco11 `getMedalLimit` = 下月 1 日 00:00（「EO勲章、プレゼント箱」一行）。
- **月末出击、跨过 00:00 才破 1-6**：没有プレゼント箱、下月ゲージ不扣、燃料 1000 照给（sakamoto-kouji.com 同页，作者 2020-03-01 自测，单一来源）。

## Decisions

- 戦果月 = UTC+11 的月份（22:00 JST 正是 UTC+11 的 0 点）。`war_result_month` 列沿用，含义改成「戦果月」。
- EO 的 75 只在戦果月等于 JST 日历月时记入；最后一天 22:00～23:59 发箱不记 75。客户端在这段时间显示什么没有证据，响应保持原样。
- 月度地图在 JST 1 日 00:00 重置（影响所有 `Monthly` 图，不止 1-6）。月度远征不动，仍是 05:00。
- 出击开始时已刷新地图，所以 00:00 后还拿着上月ゲージ的只有跨月的那次出击：这时到达 1-6 终点不写记录、不发箱、不记 75，本该破ゲージ的那次给燃料 1000，其余给 100 单位的资源。

## Not done（记进 Known gaps）

- 任务戦果在最后一天 14:00 截止、季末月作废（来源互相矛盾；用户已定任务戦果按月清零保持现状）。
- 引き継ぎ戦果（年累计经验 / 50000 + 上月 EO 戦果 / 35，zekamashi `senka-500`）。
- 除 1-6 以外的 EO 图的戦果。
- 排名每天 03:00、15:00 更新（sftsyr）；这里是实时的。

## Implementation Units

- **U1（先写测试）**：`basic.rs` 戦果月以 22:00 为界：21:59 计入本月、22:00 起算下月；EO 戦果在 22:00～23:59 不计。
- **U2（先写测试）**：`map.rs` 月度重置以 JST 1 日 00:00 为界，抽出纯函数；`escort.rs` 跨月到达和 22:00 后破ゲージ两种情况。
- **U3**：`war-result-ranking.md`、`reconnaissance-and-escort-cells.md` 更新；计划状态改 implemented。
