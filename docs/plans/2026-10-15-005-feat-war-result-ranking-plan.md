---
title: "Monthly 戦果 and a Real Ranking - Plan"
type: feat
date: 2026-10-15
status: implemented
execution: code
---

# Monthly 戦果 and a Real Ranking - Plan

Issue #147（只取 戦果 保存这一条；月底截止时间 21:59 / 23:59 继续留在 #147）。用户 2026-10-15 定下：
来源是提督经验加 1-6 终点的 75；每月 1 日（JST）清零；排名画面按本服务器所有账号的真实 戦果 排序。

**基线**：`main` `4ac1f153`，分支 `feat/war-result-ranking`。

## Evidence

- 服务器不存 戦果：1-6 终点的 75 只在 `api_req_map/next` 里发给客户端（`sortie/escort.rs` `RANK_POINTS`）。
- `profile.war_result` 已在累计任务奖励的 戦果（`add_war_result_impl`，12 个任务），从不清零；只经 `api_war_result`（自定义字段，原版 API 没有）发出，客户端不读。
- 提督经验在三处增加：出击结算（`sortie_result.rs` `update_sortie_result_stats`）、演习（`practice.rs`）、远征（`expedition.rs`）。
- 排名 `api_req_ranking/mxltvkpyuklh` 是占位：自己第 1 名，另 9 行 Anonymous；字段含义标错。客户端（`main.decoded.js` `SetAll`、`__drawRanking`）读：
  - `api_mxltvkpyuklh` 名次，`api_mtjmdcwtvhdr` 名字，`api_itbrdpdbkynm` 留言，`api_pcumlrymlujh` 提督阶级（≤2、≤6 换旗），`api_pbgkfylkbjuy` 不画；
  - `api_itslcqtmrxtf` 勲章：显示 `raw / (KEY[名次 % 13] + 1853) − 157`，小于 1 不画；
  - `api_wuhnhojjxmke` 戦果：显示 `raw / KEY[名次 % 13] / SEED(看的人的 member_id) − 91`；
  - `KEY` = [8931, 1201, 1156, 5061, 4569, 4732, 3779, 4568, 5695, 4619, 4912, 5669, 6586]，`SEED(id)` = `PORT_API_SEED[id % 10]` 去掉末两位：[31, 35, 64, 96, 72, 76, 60, 25, 95, 92]；
  - 每页 10 行；请求 `api_pageno` 缺省（0）表示「自己那一页」，回 `api_disp_page`；`api_count`、`api_page_count`。
- 占位实现把 member_id 填进 戦果 字段、1001 填进勲章，所以画面上的数字本来就是乱的。

## Decisions

- 戦果 = `floor((提督经验 − 本月起点经验) × 7 / 10000) + 本月加成`。加成是 `war_result`：任务奖励和 1-6 的 75 都记在这里（任务奖励本来就记在这里，一并改成按月）。来源：「獲得戦果 = 提督経験値 × 7 ÷ 10,000」（anastasiatetris.com、kankore-sheep.blog.jp）。
- 新列 `war_result_month`（JST 的 年×100+月，0 表示从未开始）与 `war_result_base_exp`。每次加经验或加成之前先「翻月」：月份不同就把起点经验设成加之前的经验、加成清零、月份改成本月。读的时候月份不同即为 0，不写库。
- 旧库：两列用 `ALTER TABLE ... ADD COLUMN ... DEFAULT 0` 补上（同 `fleet.launch_fully_supplied`）；月份 0 会在下一次加经验时翻月，之前的任务 戦果 清零，本月在升级之前得到的经验不计。
- 排名：所有 profile 按本月 戦果 降序、同分按 profile id 升序；勲章用 `profile.medals`，阶级用 `hq_rank`，`api_pbgkfylkbjuy` 与两个不画的分数字段填 0。编码用看的人的 member_id。
- `api_war_result` 改发本月 戦果。

## Implementation Units

- **U1（先写测试）**：`basic.rs` 的翻月与计算：同月累加、跨月清零、读时跨月为 0、经验换算取整。三处加经验与 `add_war_result_impl` 先翻月；1-6 终点调用 `add_war_result_impl`。迁移两列。
- **U2（先写测试）**：排名：`Ctx` 上的排名查询（排序、分页、自己那一页）；编码函数与客户端解码公式往返。重写 handler。
- **U3**：无头核对：打开 戦績 → ランキング，截图里自己的 戦果 与勲章是服务器算的值。
- **U4**：`reconnaissance-and-escort-cells.md` Known gaps 改写；新增 `docs/solutions` 一节记 戦果 与排名的编码；计划状态改 implemented。
