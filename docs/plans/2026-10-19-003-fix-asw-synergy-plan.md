---
title: "ASW Synergy by Item - Plan"
type: fix
date: 2026-10-19
status: implemented
execution: code
---

# ASW Synergy by Item - Plan

Issue #243。来源 `kancolle-replay@69097abc`：`kcships.js:1583-1604` `ASWPower`：`hasdcP && hasdcO` 时 ×(小型声纳 1.25，否则 1.1)，`hassonar && hasdc` 时再 ×1.15；`kcEQDATA.js` 的 `isDCProjector` 为 44、45、287、288、377、472、569，`isDCOnly` 为 226、227、378、439、488（用 node 从源码取出，与票面一致）。

**范围**：U1，一个 PR。
**非目标**：移动钉住的提交；对潜攻击力的其余部分（#231 已做）。

**基线**：`fix/accuracy-rounding`（#244）之上。

## Implementation Units

### U1 投射机与爆雷按 id 区分

- `damage.rs`：`DC_PROJECTORS`、`DC_ONLY` 两表；`asw_synergy_modifier` 写成源码的两个乘数。
- 单测：票面六组（44；44+226；小声纳+44+226；大声纳+44+226；小声纳+226；44+45）。原「单个爆雷算 1.1」的断言改为 44 单装 1.0。
- golden：180 份中 139 份变化，即 day、night 两族之外几乎全部。这些家族有潜艇，五十鈴改二与夕立改二装小声纳、探信儀与 45（投射机），旧倍率 1.4375，源码 1.15，对潜伤害约降两成（例 142→112）。day、night 两族无潜艇，未变；`battle_golden.rs` 未变。
