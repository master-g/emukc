---
title: "Decoder Ship Rules Say What The Client Asks - Plan"
type: refactor
date: 2026-10-08
status: implemented
execution: code
---

# Decoder Ship Rules Say What The Client Asks - Plan

## Problem

计划 004 查出的漏项，根源是解码器抽规则时丢了信息，而当时是在 Rust 的清单生成里打补丁绕过去的：
深海舰的破损立绘靠一个硬编码函数（`add_enemy_break_banners`），特殊攻击中破图无条件按手工表生成。
规则资产 `cache_rules.json` 与调用点清单 `resource_manifest.json` 并不知道这些，下次客户端加类似资源还会漏。
用户 2026-10-08 选定：修解码器的规则抽取，Rust 侧改成读规则。

## Decision

- **调用点清单如实记录。** 加载器作为函数参数传入时（没有 `new ShipLoader()` 可跟踪），按调用形态识别：
  `<标识符>.add(id, 中破, "<舰船资源类型>"[, 破损])`。第四个参数记为 `brokenSource`。
- **舰船语义规则加一个范围。** `break-abyssal`：`api_sp_flag` 为 1 的深海舰。只有她们保留中破状态；
  哪些类型还会带 `_b` 再要一次，从"类型字面量后面跟着字面量 `true`"的调用里读出，写进 `brokenTargetTypes`。
- **特殊攻击规则加一个标记。** 有调用点用非字面量 `false` 的中破状态加载 `special` 时，`mayBeDamaged` 为真；
  Rust 只在此时生成 `special_dmg`。哪些舰有这张图仍是手工表（主数据里没有依据），由对拍的已知空洞兜底。
- **验收是清单逐行不变。** 这是换实现不换结果的重构：删掉 Rust 补丁后重新生成的清单必须与改动前逐行一致。
- 不做：手工拼地址的两族（舰船动画、基地扩张确认图）仍由 Rust 按主数据生成，对拍的目录检查兜底；
  语义表本身仍是解码器里的手写表，由对拍第 4 项（运行客户端的 `ShipLoader.getPath`）验证。

## Outcome

- 调用点清单 420 → 437 条：多识别出 16 个经参数传入加载器的舰船调用点（战斗的横幅预加载等），另有 6 条带 `brokenSource`。
- `cache_rules.json` 多 8 条 `break-abyssal` 语义；`special.mayBeDamaged` 为真。
- Rust 删掉 `add_enemy_break_banners`，范围判定加 `BreakAbyssal`，语义目标带"是否也要破损版"。
- 重新生成的清单 73,160 条，与改动前逐行一致。
- 再生资产：`cache_rules.json`、`resource_manifest.json`（其余五个只有时间戳变化，已还原）。

## Verification

- 三道质量门；`cd main-decoder && bun run check && bun test`。
- `cache make-list` 前后 `sort | comm` 无差异；`make cache-list-oracle` 五项 0 差异。
- 新测试：`an_abyssal_ship_with_a_broken_look_gets_the_banners_her_rule_names`（Rust）、
  `follows a loader handed in as a parameter, and records the broken look argument`（解码器）。
