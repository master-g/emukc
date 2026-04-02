# 7-3 第一阶段击破后真实数据利用计划

## Summary

新增的两份真实数据：

- `crates/emukc_bootstrap/assets/real_map_start_data/map_7-3-part2.json`
- `docs/real_data/map_next/7-3-part-1.txt`

它们的核心价值不是“多一份抓包”，而是第一次把 `7-3` 的两件关键事实同时锁住：

1. 第一阶段击破后，需要通过再次请求 `api_req_map/start` 进入第二阶段。
2. 第二阶段的 start 结构不是当前 `pre_p_unlock` 的简单重用，而是带有 `17..25` 号 cell 和 `4801..4826` master cell id 的新结构。

当前 repo asset 中，`7-3` 的 `pre_p_unlock` 和 `post_p_unlock` 仍然共享 `0..16` 的 cell 集合，public overlay 也只覆盖了 `pre_p_unlock`。因此这批新数据的主要作用是补强：

- `7-3` phase transition 的真实行为回归
- `post_p_unlock` start 结构的真实覆盖
- overlay/stage matching 对 7-3 第二阶段的识别能力

## Evidence Value

- `map_7-3-part2.json`
  - 直接提供 `7-3` 第二阶段再次 `start` 后的 `api_cell_data`
  - 可作为 `post_p_unlock` 的首份权威 start capture
  - 能证明第二阶段当前至少包含 `0..25` 的 cell 结构，而不只是 `0..16`
  - 能提供 `4801..4826` 的 `master_cell_id`，用于 overlay 回填

- `7-3-part-1.txt`
  - 把第一阶段 boss 战到结算再到重新 start 的完整时序串起来
  - 能证明“phase switch != map complete”，因为 `battleresult.api_first_clear == 0`
  - 能证明阶段切换发生在第一阶段击破后的重新 start，而不是同一 sortie 内热切图

- 两者组合后的价值
  - 既能锁“何时切换”，也能锁“切换到什么结构”
  - 足以补 7-3 的 phase transition regression
  - 还不足以单独定义第二阶段完整 route graph / 全量 enemy semantics / battle fidelity

## Key Changes

- 更新 bootstrap real start 资产
  - 将 `map_7-3-part2.json` 纳入 embedded real map start assets
  - 让 overlay build 流程把它识别为 `73:post_p_unlock`
  - 重新生成 `public_map_catalog_overlays.json` 后，map 73 应同时覆盖 `pre_p_unlock` 与 `post_p_unlock`

- 强化 7-3 stage matching 回归
  - 新增 7-3 双阶段匹配测试
  - 断言 `map_7-3.json -> pre_p_unlock`
  - 断言 `map_7-3-part2.json -> post_p_unlock`
  - 测试必须基于真实 cell 集合区分，而不是依赖 default stage 偏好兜底

- 把文本抓包整理为结构化 fixture
  - 从 `7-3-part-1.txt` 提炼最小必要字段，形成 gameplay 或 router 级 fixture
  - 只保留阶段切换需要的关键事实：
    - 第一阶段 boss 节点进入方式
    - battle result 成功击破
    - `api_first_clear == 0`
    - 再次 start 后进入第二阶段结构
  - 不要求原样保留整段 curl transcript 作为测试输入

- 新增 7-3 外部行为回归
  - 保留现有 `first_gauge_clear_switches_map_variant_without_finishing_map`
  - 另补一条真实数据驱动的行为测试：
    - 第一阶段击破后 map record 切到 `post_p_unlock`
    - 再次 `start_sortie(7, 3, ...)` 返回第二阶段 `cell_data`
    - 关键断言至少验证：
      - `cell_data` 含 `17` 以上 cell
      - 或 `master_cell_id` 落在 `4801..4826`

- 更新 overlay/report 预期
  - map 73 不应再只覆盖一个 stage
  - 若 overlay/report 仍显示 `73:post_p_unlock` 未覆盖，则视为失败

## Public Interfaces / Types

- 不新增 gameplay 或 network 对外接口
- 不修改 `MapCatalog` 公开语义
- 允许新增：
  - bootstrap 内部 fixture
  - overlay coverage/report 断言
  - gameplay 的 7-3 second-phase regression test

## Test Plan

- Bootstrap
  - `map_overlay` 新增 7-3 双阶段匹配测试
  - `map_overlay` 新增 7-3 `post_p_unlock` overlay merge 测试
  - overlay/report 测试验证 map 73 的两个 stage 都被覆盖

- Gameplay
  - 保留 `first_gauge_clear_switches_map_variant_without_finishing_map`
  - 新增“7-3 第一阶段击破后再次 start 返回第二阶段结构”的回归测试
  - 明确断言第二阶段 start 不再退回 `0..16`

- Optional router-level regression
  - 若现有 test helper 足够，补 `api_req_map/start` 的 7-3 second-phase response 测试
  - 目标是确保 response 组装层不会把第二阶段结构压回旧数据

## Assumptions

- `map_7-3-part2.json` 是第一阶段击破后再次请求 `api_req_map/start` 的权威结果
- `7-3-part-1.txt` 的主要用途是锁阶段切换时机，不要求原样转成测试输入
- 本轮先解决 map phase transition / overlay coverage，不把 7-3 battle fidelity 一起扩大成新范围
- 如果接入后发现 `post_p_unlock` 的 wikiwiki 结构与真实 start 差异已超出 overlay 能表达的边界，再单独开一个“7-3 phase-2 graph correction”增量计划
