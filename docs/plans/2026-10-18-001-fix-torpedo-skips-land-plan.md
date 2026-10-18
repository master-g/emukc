---
title: "Torpedoes Skip Land Targets - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# Torpedoes Skip Land Targets - Plan

Issue #217。来源 `kancolle-replay@69097abc`：`torpedoPhase`（`kcsim.js:1777-1782`）开头把 `isInstall` 从两边的目标里去掉，开幕（`opening = true`）与闭幕共用。

**范围**：U1–U2，一个 PR。
**非目标**：雷击的伤害、命中与能否出雷；航空、对潜、夜战的目标；友方联合舰队里敌方雷击打哪支舰队（#219）。

**基线**：`main` `5922daf5`。

## Implementation Units

### U1 雷击的候选

- `target_candidates`：`OpeningTorpedo` 与 `ClosingTorpedo` 不收陆上设施，敌我两边相同。
- `deck_candidates` 的闭幕雷击按舰队求候选，只剩陆上设施的舰队自然算空，不掷 0.35，直接打另一支；不另写判断。

### U2 测试与文档

- 单测（雷击阶段）：雷巡与陆上设施混编时，开幕与闭幕、敌我两边的雷都打雷巡；只有陆上设施时不出雷。
- 单测（抽选函数）：敌联合一支舰队只剩陆上设施时，闭幕雷击不掷骰，直接打另一支。
- 原单测 `torpedo_targeting_keeps_installations_in_surface_bucket_for_now` 记的是旧行为，改为断言新行为。
- `day-shelling-order.md` 写明雷击不打陆上设施并注明来源；golden 若动，按流程重冻并说明。
