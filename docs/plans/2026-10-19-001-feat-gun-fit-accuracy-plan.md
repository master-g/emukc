---
title: "Gun Fit Accuracy - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Gun Fit Accuracy - Plan

Issue #193。来源 `kancolle-replay@69097abc`：`kcships.js:1041-1251` `getFit`；`:570-573` 在默认的 `fitGunUpdate1` 下对所有舰船取 `ACCfit = ACCfitN = getFit()`（旧表 `FITDATA` 只有 BB/FBB/BBV 有，这三种又被 `getFit` 覆盖，不移植）；用处 `kcsim.js:530`（昼战，`hitRate` 之后、`accMod2` 之前）、`:822`（夜战，`hitRate` 之后、探照灯 7 与 `ACCnbca` 一起，夜战 `accMod2` 恒为 1）。

**范围**：U1–U2，一个 PR。
**非目标**：轻巡的火力 fit（#204）；KC3Kai 的值（triage 已定用 kancolle-replay）；`enableASFit`（默认关）；挪动钉住的提交或补源码没列的 id；雷击、对潜、航空攻击的命中；AP 弹命中（#196）。

**基线**：`main` `29ba4d28`。`accuracy.rs` 模块注释与 `battle-hit-and-critical-rolls.md` 把 gun fit 列为未做。

**已核对**（一次性，脚本不提交）：源码 `SHIPDATA` 的 `type` 与 start2 `api_stype` 一一对应（FBB=8、BB=9、BBV=10、CL=3、CLT=4、CT=21、AV=16、AO=22）；`sclass` 全等于 `api_ctype`；BB 的 `HP <= 92` 与 `api_taik[0]` 判断全同；`MAINGUNM` 全等于 `api_type[2] == 2`。源码造敌舰也走 `loadEquips`，`getFit` 不分敌我；出击时 `slot_items` 含补強増設。

## Implementation Units

### U1 `gun_fit`：按舰种与舰级算命中 fit

- 新模块 `gun_fit.rs`，纯函数：攻击方的 `api_stype`、`api_ctype`、`api_ship_id`、`api_taik[0]`、`api_lv` 与所带装备的 master id → 百分点。按源码分支顺序：FBB、BB、BBV 各自返回；CL/CLT/CT；AV/AO；再按舰级（最上、Ташкент/Верный、J 级、John C. Butler、Fletcher、睦月）。
- 等级用 `api_lv >= 100`（源码 `LVL`），不用 `married`。
- 单测（数字取自源码）：金剛型 35.6cm ×1 = +7；46cm ×1 = −10、Lv100 −6；球磨型 15.5cm 三連装 ×1 = −3；阿賀野型 15.2cm 連装 ×1；J 级炮 + ASDIC；无规则舰种 = 0；带中口径炮的深海轻巡。
- 一次性对拍（不提交）：源码 `getFit` 对 start2 全部舰船（含深海）× 覆盖每个 id 列表的装备组合 × Lv99/Lv100，与 Rust 逐项比。

### U2 命中加 fit

- `Aim` 分出 `accMod2`：昼战炮击的 cut-in 倍率（`day_accuracy_modifier`）挪到 fit 之后；特殊攻击与夜战的倍率仍在 `hitRate` 里。`hit_value` = `(内项 + fit) × accMod2`，夜战再加探照灯 7；fit 只给 `Shelling` 与 `Night`。
- fit 为 0、`accMod2` 为 1 时与现在逐位相同；golden 变动的必须是舰队里有 fit ≠ 0 的舰，PR 写明。
- 单测：同一攻击的命中值只差 fit；昼战 cut-in 时 fit 乘以倍率，夜战 cut-in 时不乘；雷击、对潜不变。
- `accuracy.rs` 模块注释与 `battle-hit-and-critical-rolls.md`（Not modelled）去掉 gun fit，文档写明来源，新装备在钉住提交前移之前没有 fit。
