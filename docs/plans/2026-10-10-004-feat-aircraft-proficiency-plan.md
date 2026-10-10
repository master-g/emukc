---
title: "Aircraft Proficiency in the Hit and Critical Rolls - Plan"
type: feat
date: 2026-10-10
status: implemented
execution: code
---

# Aircraft Proficiency in the Hit and Critical Rolls - Plan

## Problem

`battle-hit-and-critical-rolls.md` 的「Not modelled」第一项：舰载机熟练度没有进战斗。结果是开幕航空战
永远不出暴击（`api_fcl_flag` / `api_ecl_flag` 恒为 0），空母昼战炮击的命中与暴击也不看熟练度。

## Evidence

**我方现状：**

- 熟练度存在 `slot_item.aircraft_lv`（0–7），以 `KcApiSlotItem::api_alv` 进战斗：`sortie/setup.rs:405`
  把舰上非空的装备按槽位顺序收进 `BattleShipInput::slot_items`。数据已经在手，没有人读。
- `accuracy.rs` `roll_strike` 给 `roll` 传暴击系数 0；`roll` 在系数为 0 时不判暴击。
- `HitOutcome::power` 的暴击倍率固定 1.5。
- `simulation/kouku.rs:503` 把两个 `cl_flag` 写死为 0。客户端把它读成暴击标记
  （`AirWarStage3Model.getHitType` 返回 `flag + 1`）。
- 敌方舰由 manifest 构造，`slot_items` 为空，天然没有熟练度。
- 全仓库没有让熟练度成长的写入点：`aircraft_lv` 只在发放装备（场景 `slots_maxed`、奖励）时设定，
  任务换装时清零。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcships.js` `setProficiency`（2501）：等级对应内部经验 `[0,10,25,40,55,70,85,120]`。
- `kcships.js` `updateProficiencyBonus`（1252）按舰汇总，下标 `i` 是装备在舰上的次序（空槽不占位，
  `loadEquips` 436 行 `if (!equips[i]) continue`，与我方 `slot_items` 一致）。对「带攻击熟练度」的每件装备：
  - 等级系数 `mod`：等级 1–7 对应 `1,2,3,4,5,7,10`。
  - 暴击率加成 `critratebonus += mod × (i==0 ? 0.8 : 0.6)`。
  - 暴击伤害加成 `critdmgbonus = 1 + Σ floor(sqrt(exp) + mod) / (i==0 ? 100 : 200)`。
  - 经验为 0 的装备两项都不加，但计入平均经验的分母。
  - 命中加成 `ACCplane`：平均经验 ≥10 时为 `sqrt(平均经验 × 0.1)`，再按平均经验
    ≥100/80/70/55/40/25 加 `9/6/4/3/2/1`。
- 带攻击熟练度的机种（`kcEQDATA.js` 的 `hasAttackProficiency`）：艦攻、艦爆、水爆、大型飛行艇、陸攻、
  大型陸上機、噴式戦闘爆撃機；对潜哨戒机与旋翼机在爆装大于 0 时也算（`kcships.js:2367`），
  经验乘 0.825、等级减 1。
- `kcsim.js` `accuracyAndCrit`（2144）：命中率封顶 96 **之后**加 `ACCplane`；暴击阈值
  `sqrt(命中率) × 系数 + critratebonus`，两者都取整后与同一个抽签比较。
- `airstrike`（1968）用系数 0，所以航空攻击的暴击率就是 `critratebonus`；`rollHit`（2191）暴击返回
  `1.5 × critdmgbonus`。支援航空不带熟练度。
- 空母昼战炮击（599）在 `CVshelltype` 时传 `isPlanes`，用同一组三个值，系数 1.3。

## Decision

1. 按舰算一次熟练度加成（命中、暴击率、暴击伤害），公式照上面逐项转写；`api_alv` 按表换成内部经验。
2. 开幕航空战的每次攻击带上攻击舰的加成；出暴击时把目标的 `cl_flag` 置 1。
3. 空母（`is_cv_type`）的昼战炮击带上同一组加成。
4. 保持一次攻击一次抽签。没有熟练度的舰，抽签与结果和现在逐位相同。

## Scope

**不做：**

- 制空値的熟练度加成（`APbonus`）：会改变每个带熟练机场景的制空状态，另立计划。
- 熟练度的成长与损失（出击后上涨、槽被打空后清零）：来源模拟器不建模，需要另找依据。挂点在战斗结算。
- 航空队（`AirSquadronInput` 不带熟练度）、夜间航空攻击、带飞机的对潜攻击、空母切入的专用暴击补正。

## Implementation Units

- **U1** `accuracy.rs`：`PlaneProficiency` 与它的汇总函数；`roll` 接受命中加成与平加暴击率；
  `HitOutcome::power` 之外加一个带暴击伤害加成的入口。单元测试钉住满熟练度首槽的三个值
  （命中 `sqrt(12)+9`、暴击率 8、暴击伤害 1.2）与「无熟练度不变」。
- **U2** `simulation/kouku.rs`：攻击舰的加成进 `roll_strike` 与航空伤害；`cl_flag` 按目标置位。
  带种子的测试：满熟练度的艦攻打出暴击且 `api_ecl_flag` 为 1；无熟练度时 flag 全 0。
- **U3** `simulation/shelling.rs` 与 `damage.rs`：空母炮击带加成。测试：同一抽签下熟练度把一次普通命中
  变成暴击。
- **U4** 基线若有变化则重新冻结并说明；更新 `battle-hit-and-critical-rolls.md`（公式、「Not modelled」）；
  回写 `PROJECT_MEMORY.md`。

## 实施记录（2026-10-10）

- U1–U3 按计划做完。`roll` 多收一个 `PlaneProficiency`；无熟练度时传 `NONE`，算式与原来逐位相同。
  空母炮击的加成在 `roll_attack`（`Shelling`）与 `calculate_shelling_damage` 里各取一次，判断都是 `is_cv_type`。
- 基线没有变化：`crates/emukc_battle/tests/golden/` 与 `battle_golden.rs` 都不需要重新冻结
  （基线里的舰都没有熟练度）。
- 测试：满熟练度首槽的三个值、第二槽与零熟练度的折算、抽签的两个阈值；200 个种子里无熟练度的
  艦攻不出暴击、满熟练度出暴击并置 `api_ecl_flag`，且两者之后的随机流相同。

## 审查后的修正（2026-10-10）

- 暴击伤害的乘法顺序改成与来源一致：先 `1.5 × 加成`，再乘威力后取整（威力 100、加成 1.2 得 179）。
- 空母切入原先会吃到汇总的暴击率，而来源把它抵掉、换成 `13 × 平均经验 / 120`。现在切入走这一项；
  来源按首槽机种与经验再加的项、切入专用的暴击伤害仍然不做。
- 补了 U3 缺的测试：带熟练度的空母经 `calculate_shelling_damage` 的暴击更重，普通命中不变。
- 来源的 `CVshelltype` 还包括带艦攻/艦爆的補給艦；我方只认 CV / CVL / CVB，未改。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings`、`cargo test --workspace`。
- 无头检查 `gunnery_cutin` 通过。这个场景的赤城只带零熟练度的艦戦，所以它只说明无熟练度时没有回归；
  客户端对 `cl_flag` 的读法来自已解码的 `getHitType`，暴击演出没有在真实客户端里观察过
  （现有出击场景都没有带熟练度的攻击机）。
