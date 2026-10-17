---
title: "Air Corps Strike Follows the Source - Plan"
type: fix
date: 2026-10-17
status: implemented
execution: code
---

# Air Corps Strike Follows the Source - Plan

Issue #208。来源 `kancolle-replay@69097abc` `kcsim.js`：`LBASPhase`（2988）、`airstrikeLBAS`（3135）；`kcships.js:143` 每场战斗清 `LandBase.airStatePrev`/`contactPrev`。每波：第一阶段 → 触接（承接上一波）→ 每个出击中队挨对空炮、抽目标，全部抽完 → 逐个结算。

**范围**：U1–U3，一个 PR。
**非目标**：掷骰顺序（issue 第二条评论撤掉）；基地航空队特殊加成（#200）、気球（#198）、喷气机强袭；对联合舰队的 postMod ×1.1；敌方联合舰队时 `air_base_attack` 的 stage3 不分主/护卫两段（既有缺口）。

**基线**：`main` `0704abd4`。

## Implementation Units

### U1 熟练度读真实经验

- `AirSquadronInput.exp: Option<i64>`；gameplay 两处填 `Some(slot_item.aircraft_exp)`，缺失时用等级的代表值。
- `squadron_proficiency(type3, alv, exp)`：命中 `√(exp×0.1)` + 等级表（0 级为 0），暴击伤害 `floor(√exp + critval)/100`；反潜哨戒机、オートジャイロ的经验 ×0.825、等级 −1 照旧。

### U2 触接与目标

- `roll_contact`/`contact_chance` 改吃 `(装备 id, 机数)` 列表，舰载机与基地共用一份。
- 跨波：均势时用上一个非均势状态掷触接，否则记下本波状态（丧失也记）；掷中则记倍率并写 `api_touch_plane[0]`；有效状态为丧失时沿用上一次的倍率，不写 touch_plane。状态在 `execute_air_base_attacks` 里每场战斗新建，跨所有基地。
- 陆上偵察機（49）：命中 ≥3 给 ×1.15，否则 ×1.12，乘在 cap 之前；触接倍率乘在 cap 取整之后。
- 出击中队：水面攻击机（现有的攻击机种类，及有爆装的反潜哨戒机/オートジャイロ）或对潜 ≥7 的飞机。挨对空炮的照旧。
- 候选：水面机打浮着的非潜艇；反潜机加上潜艇（只反潜的只打潜艇），有潜艇就只留潜艇。敌方联合舰队 .45 主队、否则护卫队，一边空就用另一边。`choice_with_protect(…, 0, false, 敌阵形)`；候选为空跳过该中队。
- 对潜艇：威力取对潜，pre-cap 倍率整个换成 `对潜 ≥10 ? 0.7+r×0.3 : 0.35+r×0.45`（陆攻的 0.8 被覆盖），旗标为爆击。
- 先抽完再结算；打在已沉船上的伤害作废；かばう 写 `DamageCell::Shielded`。
- 删去「装备威力为 0 就不打」的提前返回：东海打水面舰按基础 25 出击，与 #207 用户选定的「照来源」一致。

### U3 测试与文档

- 单测：70/81 经验 → 1.13/1.14；跨波触接序列；45% 分流；反潜机选潜艇；旗舰被かばう；两中队打同一船、第二击在船沉后。
- golden 若动，按流程重冻并在 PR 说明；定种子测试落空就换种子。
- `plane-losses-and-proficiency-growth.md` 删掉基地航空队那句；`air-corps.md` 改目标与触接的描述。
