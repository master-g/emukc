---
title: "Improvement Firepower from the Source - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# Improvement Firepower from the Source - Plan

Issue #206。来源 `kancolle-replay@69097abc`：`kcships.js:2387-2491` `Equip.prototype.setImprovement`，`kcEQDATA.js` `EQTDATA[type].improve`；用处 `kcships.js:1530`（`shellPower`）、`:2000`（`CV.shellPower`）、`:1558`（`NBPower`）、`:1598`（`ASWPower`），`kcsim.js:1851`（雷击）。

**范围**：U1–U2，一个 PR。
**非目标**：命中、回避键（#194，往同一张表里加）；对空、索敌、装甲、航空攻击的改修；基地航空队；移动钉住的提交；空母炮击公式本身（`1.5 × 爆撃機 + 55`）；改修项照源码在 `1.5 × (…)` 里，按 1.5 倍计入。

**基线**：`main` `52f1fd90`。#194 未做，代码里没有按种类的改修表，本次建表，只放火力四项。

## Implementation Units

### U1 按种类的改修表

- `damage.rs`：`Improvement { shell, night, torpedo, asw }`，`improvement_coefficients`（客户端 `api_type[2]` → √★ 系数），`improvement`（单件：表前的飞机线性项、表、火力 13 的切换、副炮按 id、潜水艦魚雷、226/227/488），`ship_improvement`（全舰求和）。
- 替换 `improvement_bonus_day/_torpedo/_night`；ASW 在协同倍率前加 `asw`。

### U2 测试与文档

- 单测：票面九个数值，外加 ★0 不变、整舰求和。
- 一次性对拍（不提交）：Node 跑源码自己的 `setImprovement`，对 start2 全部 584 件 ★1–★10 比四项。
- 重写 `equipment-improvement-bonus.md`；`damage-formula-reference.md` 改指针。
