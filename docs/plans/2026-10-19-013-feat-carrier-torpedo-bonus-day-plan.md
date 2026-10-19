---
title: "Carrier Torpedo Fit Bonus by Day - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# Carrier Torpedo Fit Bonus by Day - Plan

Issue #260。来源 `kancolle-replay@69097abc`（`MECHANICS.eqBonusTorp` 默认开）：`kcships.js:1299-1340` `getEquipBonusCVTorp`；`kcships.js:1986-2004` `CV.prototype.shellPower`；`kcsim.js:2022-2025` `airstrike`。

**基线**：`main`（#262 之后）。
**非目标**：`DBCrew`（`gear_bonus.json` 没有爆装加成，恒为 0）；机体改修（#264）与按机体倍率（#263）。

## Implementation Units

### U1 昼战炮击

- `damage.rs` `carrier_gun_power`：雷装加上整舰的 `TPPlane + TPCrew`（复用夜战的 `carrier_torpedo_bonus`），对地时雷装照旧清零，加成也随之不计（来源先加后清）。删掉 `ponytail:` 注释。

### U2 航空战（每槽）

- `night.rs` 拆出 `torpedo_bonus_parts`（plane, crew），整舰加成仍是两者之和；新 `carrier_slot_torpedo_bonus(slot)`：该槽是「雷装（艦爆系看爆装）最高、平手看机数更多、再平手取前者」的那一槽才拿 plane；艦攻再拿 crew。艦爆系照来源 `isdivebomber`：7、11、25、26、47、53、57。
- `kouku.rs` `calculate_single_slot_airstrike_damage`：命中后把它加进槽的数值（`√机数` 之前），对地的艦攻也加（来源不分目标）。只有舰队航空战走这里，支援与喷气阶段不受影响。
- 机数读打击时的 `api_onslot`（stage 1、2 损失之后，与来源 `planecount` 同时刻）。

### 验证

- 单测：每槽选择（最高雷装、平手看机数、非最高槽为 0、艦爆不拿 crew）、昼战炮击对海加、对地不加。
- golden：180 份与 `battle_golden.rs` 未变（默认舰队不带有加成的舰载机）。
- 文档：`carrier-night-air-attack.md` Known gaps。
