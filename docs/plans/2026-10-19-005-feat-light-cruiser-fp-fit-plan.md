---
title: "Light Cruiser Gun Fit Firepower - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Light Cruiser Gun Fit Firepower - Plan

Issue #204（自 #193 分出）。来源 `kancolle-replay@69097abc`：`kcships.js:1862`、`:1869`、`:2198`（CL、CLT、CT 的 `fitclass = 100`），`:476-477`（按装备 `fitclass` 计数），`:565-568`（`FPfit = √n101 + 2√n102`）；昼战 `kcsim.js:602` `(shellPower + FPfit) × preMod`，夜战 `:729-731` `bonus += FPfit`，`(NBPower + bonus) × preMod`（`:877`）。

**范围**：U1，一个 PR。
**非目标**：命中 fit（#193 已做）；`fitclass` 1–11（战舰）的火力，源码没有。

**基线**：`fix/air-corps-enemy-combined`（#242）之上。已有的 `light_gun_bonus` 只给 CL、CLT，按装备类型（小口径主炮算单装，中口径主炮算连装）计数，且夜战不加。

## Implementation Units

### U1 按 id 计数，夜战也加

- `damage.rs`：`LIGHT_SINGLE_GUNS`（101：4、11）、`LIGHT_TWIN_GUNS`（102：12 项），用 node 从 `kcEQDATA.js` 取出与常数比对，差集为空；舰种加 CT；`calculate_night_damage` 把它与夜侦加成一起加在倍率之前。
- 单测：CL 装两门 139 得 2√2，CT 装 4 得 1，CA 得 0；昼战 55→57、夜战 50→52。
- golden：180 份中 16 份变化（ctf 6、ec 4、te 6）。軽巡ホ級与雷巡チ級带 1504（5inch単装高射砲，中口径主炮），旧代码给 2，源码不在 102 表里，给 0。验证：把 1504 临时加入连装表并去掉夜战一项，180 份与旧版逐字节一致。`battle_golden.rs` 未变。
