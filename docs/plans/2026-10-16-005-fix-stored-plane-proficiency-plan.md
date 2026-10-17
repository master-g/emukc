---
title: "Stored Aircraft Experience in Battle - Plan"
type: fix
date: 2026-10-16
status: implemented
execution: code
---

# Stored Aircraft Experience in Battle - Plan

Issue #202。`plane_proficiency`（`crates/emukc_battle/src/accuracy.rs`）按 `api_alv` 取等级的代表经验（`PROFICIENCY_EXP`），而服务器存着真实经验 `slot_item.aircraft_exp`（0–120）。来源 `updateProficiencyBonus`（`kancolle-replay@69097abc` `kcships.js:1252`）用的是装备自己的 `exp`：暴击伤害 `floor(√exp + 等级值)`、命中的平均经验都随它变；只要 `exp > 0` 就计入（不看等级）。

**范围**：U1–U2，一个 PR。
**非目标**：基地航空队（`AirSquadronInput.alv`、`squadron_proficiency`），另开 issue；出击途中经验变化后的下一战（输入在出击时建一次，和等级一样到港才更新）。

**基线**：`main` `ec20be74`。

## Implementation Units

### U1 把经验带进战斗

- `BattleShipInput`、`BattleRuntimeShip` 加 `aircraft_exp: Vec<i64>`，与 `slot_items` 按位置对应；空表示不知道（敌舰、演习对手、测试），回退到等级代表值。
- 出击（`sortie/setup.rs` `build_sortie_friend_ships`）与演习己方（`practice.rs`）从数据库行填；`sortie_result.rs` 的 `to_inputs` 照抄。其余构造处给空。

### U2 `plane_proficiency` 读它

- 经验取存储值，没有就取 `PROFICIENCY_EXP[level]`；巡逻机、自转旋翼机照旧 ×0.825、等级减一；是否计入改为 `exp > 0`。
- 单测：同为 5 级、经验 70 与 81 的暴击伤害不同（1.13 / 1.14）；7 级 100 与 120 暴击伤害相同、命中不同；0 级经验 5 的飞机按来源计入。
- `plane-losses-and-proficiency-growth.md` 的 Not modelled 改为只剩“出击途中读出港时的经验”与基地航空队。
