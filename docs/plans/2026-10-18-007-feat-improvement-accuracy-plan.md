---
title: "Improvement Accuracy and Evasion - Plan"
type: feat
date: 2026-10-18
status: implemented
execution: code
---

# Improvement Accuracy and Evasion - Plan

Issue #194。来源 `kancolle-replay@69097abc`：`kcEQDATA.js` `EQTDATA[type].improve` 的 `ACCshell`/`ACCnb`/`ACCtorp`/`ACCasw`/`EVshell`/`EVtorp`，`kcships.js:2480-2484` 雷达（命中 ≥ 3 用 1.7/1.6）；用处 `kcsim.js:507`（昼战 `evFlat`）、`:525`（昼战 `accflat`）、`:710`（夜战 `accFlat`）、`:978`（对潜 `sonarAcc`）、`:1857`（雷击 `floor(ACCtorp)`）、`:1869`（雷击 `evFlat`），`accuracyAndCrit` 在曲线之后加 `evFlat`、再取整。

**范围**：U1–U2，一个 PR。
**非目标**：火力键（#206 已做）；对空、索敌、装甲；航空攻击与基地航空队的命中；对潜公式（#231）。

**基线**：`main` `83671d8e`。改修表 `improvement_coefficients` 只有火力四项。

## Implementation Units

### U1 表里加命中、回避六项

- `Improvement` 加 `shell_accuracy`、`night_accuracy`、`torpedo_accuracy`、`asw_accuracy`、`shell_evasion`、`torpedo_evasion`；逐项组合只写一次（`zip`，求和与 √★ 缩放都经过它）。
- `improvement_coefficients` 按源码补各类型的系数；`improvement` 加雷达特例。

### U2 命中与回避读这几项

- `hit_value` 按攻击种类加攻击方的命中项（雷击取整）。
- 回避：昼战加 `shell_evasion`、雷击加 `torpedo_evasion`，在曲线之后、取整之前。
- 单测：票面数值；★0 不变。一次性对拍（不提交）：源码 `setImprovement` 对 start2 全部装备 ★1–★10 比六项。
- `accuracy.rs` 模块注释与 `battle-hit-and-critical-rolls.md` 去掉「改修」；`equipment-improvement-bonus.md` 补命中、回避。
