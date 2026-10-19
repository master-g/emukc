---
title: "Hit and Evasion Rounding - Plan"
type: fix
date: 2026-10-19
status: implemented
execution: code
---

# Hit and Evasion Rounding - Plan

Issue #244。来源 `kancolle-replay@69097abc`：`kcsim.js:2174-2175` `accuracyAndCrit`：`hit = floor(round(hit*1e6)/1e4)`，`dodge` 同。

**范围**：U1，一个 PR。
**非目标**：曲线内部的 `floor`（源码在那里也直接取整）；燃料不足项的口径；命中率乘目标士气之后的 `floor`。

**基线**：`main` `676d0ead`。

## Implementation Units

### U1 取整前先舍入到四位

- `accuracy.rs`：`floor_percent(x) = floor(floor(x*1e4 + 0.5) / 1e4)`（`Math.round` 即 `floor(x+0.5)`），用于命中（`hit_chance`）、回避（`dodge` 加上改修回避之后）和航空攻击的回避乘修正之后（`roll_strike`）。
- 单测：`100 × 1.15` 记作 115。
- 结果：180 份 golden 与 `battle_golden.rs` 未变（概率问题只在极少数值上出现）。
