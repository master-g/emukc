---
title: "ASW Power from the Source - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# ASW Power from the Source - Plan

Issue #231。来源 `kancolle-replay@69097abc`：`kcships.js:1598-1615` `ASWPower`：`(2√statsBase.ASW + 1.5·equipASW + classASW + Pasw) × synergy`；`equipASW` 只算 `EQTDATA[type].canASWDamage` 的装备，外加装备加成（`MECHANICS.eqBonusASW`）；`classASW = planeasw ? 8 : 13`，`planeasw` 在 `CAV`/`BBV`/`AV`/`LHA`（`:1897`、`:1929`、`:2144`、`:2207`）、`CVL`（`:2091`），以及 `canAirAttack` 的補給艦带舰攻或舰爆时（`:2152-2154`；`kcSHIPDATA.js` 的 352、717、1008）。

**范围**：U1–U2，一个 PR。
**非目标**：协同倍率（`asw_synergy_modifier` 不分投射机与爆雷）；对潜命中、上限、上限后修正；谁能对潜；夜战与支援对潜规则；钉住的提交。

**基线**：`main` `0c7fcdb0`。对照：源码 `SHIPDATA` 的 `type` 与客户端 `api_stype`，在上述舰种与補給艦上，start2 的 866 艘里源码有的 865 艘零差异（902 源码没有）。

## Implementation Units

### U1 对潜基础攻击力

- `damage.rs`：抽出 `asw_basic_power`（协同倍率之前）；`base_asw` 另减装备加成；新的装备项只算 8 类对潜装备并加上装备加成，线性 1.5 倍；种别常数按舰种二选一。
- `calculate_asw_damage` 用 `asw_basic_power × synergy`。

### U2 测试与文档

- 单测：票面数值（声纳加爆雷 30、只装声纳、轻空母与水母取 8、水侦不计入、装备加成 1.5 倍）；改写按开方断言的旧单测。
- `damage-formula-reference.md` 对潜一节改为源码公式。
