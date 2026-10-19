---
title: "Air Corps Strike Bonus against a Combined Fleet - Plan"
type: fix
date: 2026-10-19
status: implemented
execution: code
---

# Air Corps Strike Bonus against a Combined Fleet - Plan

Issue #251。来源 `kancolle-replay@69097abc` `kcsim.js` `airstrikeLBAS`：`:3285` `postMod` 陸上攻撃機 1.8，`:3332` `target.fleet.combinedWith` 时 `postMod *= 1.1`（两层舰队都设 `combinedWith`），`:3356` 在 `floor(softCap(pow))` 之后乘 `contactMod*postMod`。

**范围**：U1，一个 PR。
**非目标**：`enableAirstrikeSpecialBonus` 的目标专属倍率（#200 / #197 另做）。

## Implementation Units

### U1 post-cap ×1.1

- `simulation/air_base.rs`：`strike_power` 多一个 `combined`（目标 `enemy_deck.is_some()`，与回避 0.68 同一判定），在接触与 ×1.8 之后乘 1.1。
- 单测：同一中队打单舰队 117，打连合舰队 128.7。
- 文档：`air-corps.md` 删「left out」一条，正文补一句。
- golden：180 份与 `battle_golden.rs` 未变（敌连合 golden 无基地航空队）。
