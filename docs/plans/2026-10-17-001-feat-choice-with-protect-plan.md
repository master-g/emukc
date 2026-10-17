---
title: "Shared choiceWProtect Target Draw - Plan"
type: feat
date: 2026-10-17
status: implemented
execution: code
---

# Shared choiceWProtect Target Draw - Plan

Issue #201。来源 `choiceWProtect`（`kancolle-replay@69097abc` `kcsim.js:2295`）：抽目标 → 探照灯重抽 → 警戒陣前半重抽 → 目标是旗舰时先按阵形掷拦截率，掷中才收集拦截者（非目标、HP>75%、非陆上、按调用方决定含不含第二舰队），没人就留原目标。这里的 `select_escort_shield` 先收集、有人才掷；闭幕雷击和开幕对潜不调用它；第二舰队一律不拦截。

**范围**：U1–U3，一个 PR。
**非目标**：警戒陣重抽（#195，留位置）、航空战（#207）、基地航空队（#208）、反 PT 舰的拦截者名单（#197）、未知阵形的 .6 回退（实际阵形只有 1–6、11–14）。

**基线**：`main` `bae38aa6`。

## Implementation Units

### U1 共用的抽目标函数

- `targeting.rs` 加 `choice_with_protect(codex, rng, defenders, candidates, rerolls, include_escort, formation) -> Option<(usize, bool)>`：抽一个 → 探照灯重抽 → 留 #195 的位置 → 拦截。
- 拦截先掷后收集；`include_escort` 为真时第二舰队也能拦截。收集仍从整支舰队按种类筛，和来源从候选里取对现有调用方等价。
- 候选：`target_candidates` 公开到 crate，另加水上、潜艇两个；删去四个 `select_*` 抽取函数，旧测试改为直接断言候选。

### U2 调用方

| 阶段 | 探照灯 | 第二舰队拦截 |
|---|---|---|
| 昼炮击（含对潜）、特殊攻击 | 0 | 否 |
| 开幕对潜 | 0 | 是 |
| 开幕、闭幕雷击 | 0 | 否 |
| 夜战水上（含特殊攻击行） | 舰队的 | 是 |
| 夜战潜艇 | 0 | 否 |

开幕对潜与闭幕雷击的伤害按拦截标记显示。

### U3 测试、golden 与文档

- 红测：旗舰旁只有健康的陆上设施时掷一次、留原目标；第二舰队在 `include_escort` 时能拦截；闭幕雷击、开幕对潜在若干种子下出现拦截。
- 两份 golden 若变，按各自流程重冻并在 PR 说明。
- `anti-installation.md` 删 Known gaps 那条；`night-attack-types.md` 的抽取顺序按新函数写。
