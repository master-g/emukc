---
title: "Whom a Carrier Attacks at Night - Plan"
type: feat
date: 2026-10-11
status: implemented
execution: code
---

# Whom a Carrier Attacks at Night - Plan

## Problem

计划 `2026-10-11-003` 给了空母夜间航空攻击自己的威力，但没有动夜战的选靶。审查时看到三处与来源不符，
都落在空母身上：

1. 能对潜的夜间空母在敌方有潜艇时优先打潜艇，而夜战打潜艇只有擦伤，夜间航空攻击的威力这一轮用不上。
2. 夜战里哪些空母能打潜艇，我方与来源正好相反：我方要求带对潜机、正规空母也可以；来源不要求飞机、只有轻空母可以。
3. 连击要求至少一门主砲，所以带两门副砲的 Graf Zeppelin 不连击；来源与 wiki 都说会。

## Evidence

**我方现状（`crates/emukc_battle/src/targeting.rs`、`simulation/night.rs`）：**

- `attack_capability_for_phase` 夜战分支：能出手且 `can_attack_submarine_night_shelling` 为真时返回
  `BothPreferSubmarine`；`select_random_target_index` 对它是「有潜艇就只在潜艇里选」。
- `can_attack_submarine_night_shelling`：海防、驱逐、轻巡、雷巡、練巡、補給艦要求 `api_taisen[0] > 0`；
  CV / CVL / CVB 要求 `can_attack_night_ship` 且 `has_active_asw_aircraft`
  （回転翼機、対潜哨戒機、水爆、大型飛行艇；不含艦攻）；其余舰种不能。
- `is_double_attack_eligible`：`主砲 ≥ 2`，或 `主砲 ≥ 1 且 副砲 ≥ 1`，或 `主砲 ≥ 1 且 魚雷 ≥ 1`。
- 夜战打潜艇：必中、擦伤，`api_n_mother_list` 为 0（计划 003 审查后的修正）。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcsim.js` `nightPhaseTarget`（1740）：
  `有潜艇 且 canASWNight 且 (不能做夜间航空攻击 或 没有水面目标)` → 打潜艇；否则有水面目标就打水面目标。
  即能做夜间航空攻击的空母先打水面舰，水面舰打完才轮到潜艇。
- `kcships.js` `canASWNight`（1370）：
  - `planeasw == 1`（航巡、航战、水母、揚陸艦）：夜战不能对潜。我方已是这样。
  - `planeasw == 2`（轻空母，2091）：素対潜大于 0 即可，不看飞机。
  - 正规空母、装甲空母没有 `canASW`，夜战不能对潜。
  - 其余舰种同昼战。
- `kcships.js` `NBtypes`（1435、1441）：`主砲 + 副砲 ≥ 2` 即连击。「主砲加魚雷」在来源里是切入（1440），不是连击。
- 夜战对潜的命中与伤害，`kcsim.js` `ASW`（975–1011）：有命中抽签；昼战之后的夜战威力乘 0，命中只有擦伤，
  开幕夜战的格子才按对潜公式算。
- wikiwiki Graf Zeppelin 页（2026-10-11 取）：「副砲2つで夜戦連撃も一応可能」。

**主数据：** 49 艘轻空母里 22 艘 `api_tais[0] > 0`；素対潜随等级成长，取法见 Risks。

## Decision

1. **能做夜间航空攻击的空母先打水面舰。** 只在没有活着的水面目标时才选潜艇（且她得能在夜战对潜）。
   按炮击出手的空母不属于这一条：其中的轻空母照旧有潜艇先打潜艇（来源同），正规空母按 Decision 2 夜战不对潜。
2. **夜战里空母对潜的资格照来源。** 轻空母：素対潜大于 0，不要求对潜机。正规空母、装甲空母：不能。
   前提仍是她在夜战能出手（Decision 2 of plan 003）。
3. **两门副砲也连击。** `is_double_attack_eligible` 改为 `主砲 + 副砲 ≥ 2`，或原有的 `主砲 ≥ 1 且 魚雷 ≥ 1`。
   这一条不只影响空母：任何带两门副砲、没有主砲的舰都会开始连击。**待确认**（见下）。
4. **没有潜艇的战斗、不带副砲的舰，抽签与结果逐位不变。**

## 待确认

- **Decision 3 做不做。** 它是普通夜战规则，不是空母专属；空母是受它影响最明显的一类（Graf Zeppelin、
  加賀改二護等按炮击出手的 11 艘）。我建议做：改动是一行条件，来源与 wiki 一致。
  不做的话这份计划只有选靶与对潜资格两条。
- **「主砲加魚雷」算连击**是我方既有规则，来源把它算作切入。它牵涉夜战切入的整张判定表，这次不动，只记入已知差异。

## Scope

**不做：**

- 夜战对潜的命中抽签与开幕夜战格的对潜公式：是独立的一份计划（`PROJECT_MEMORY.md` 的候选之一）。
- 非空母舰种夜战对潜的资格（我方用含装备的 `api_taisen[0]`，来源用素対潜）。
- 敌方空母的夜战、しまね丸改。
- 夜战切入的判定表。

## Implementation Units

- **U1** `targeting.rs`：`AttackCapability` 加一档「两者都能打、先打水面」；`attack_capability_for_phase` 的夜战分支
  对 `can_night_air_attack` 的舰返回它；`select_random_target_index` 相应选靶。
  测试：夜间空母对「一艘驱逐加一艘潜艇」总选驱逐；只剩潜艇时，能对潜的轻空母选潜艇，正规空母不出手；
  按炮击出手的轻空母（大鷹改二）先选潜艇。
- **U2** `targeting.rs` `can_attack_submarine_night_shelling`：空母分支按 Decision 2。
  测试：带回転翼機的正规空母夜战不选潜艇；不带对潜机而素対潜大于 0 的轻空母可以；素対潜为 0 的不行。
  现有断言旧规则的测试按新规则改写并记入实施记录。
- **U3**（若确认）`night.rs` `is_double_attack_eligible`：两门副砲连击。
  测试：Graf Zeppelin改带两门副砲、无夜间机 → 连击且 `api_n_mother_list` 为 0；带一门副砲 → 普通攻击。
  计划 003 里为够到连击而塞主砲的那条测试改用两门副砲。
- **U4** 文档：`carrier-night-air-attack.md` 的已知缺口去掉已做的两条；`PROJECT_MEMORY.md`。

## 实施记录（2026-10-12）

- 用户确认三条都做。U1–U4 做完，改动在 `targeting.rs`、`types/domain.rs`、`night.rs` 与文档。
- 素対潜取到了：`Codex::find_ship_extra` 的 `tais` 按等级成长再加近代化改修（`bare_asw`），与 `cal_ship_status` 同一算法，
  没有退到主数据的初始値。
- 现有测试里只有一条受影响：计划 003 的 `the_packet_says_who_flew_her_night_attack`，其中打潜艇的空母从
  「带回転翼機的轻空母」换成龍鳳改二戊，连击那一例不再需要塞主砲，改用两门副砲。
  没有带种子的测试移位，没有换种子；两类基线都没有变化。
- 新测试两条：选靶（夜间空母先打水面舰、只剩潜艇才打潜艇；大鷹改二先打潜艇；素対潜为 0 的轻空母与正规空母不打潜艇）、
  两门副砲连击而一门不连击。

## 审查后的修正（2026-10-12）

- 两路审查确认选靶与夜战对潜资格和来源一致，`bare_asw` 与 `cal_ship_status` 逐项相同。问题都出在 Decision 3：
  - 能做夜间航空攻击的空母带两门副砲时被判成了炮击连击。来源里她只查空母切入的表（`kcships.js` 2064），
    查不到就是普通的夜间航空攻击。`detect_night_attack_type` 现在对带夜间航空队的空母不再落到炮的规则。
    初版的测试与文档把错的行为写成了规则，一并改了。
  - 切入抽签失败后的回落，初版让「无主砲、两门副砲、两条魚雷」的舰多出一种连击。回落恢复为改动前的条件（要有主砲）。
- 补了测试：无夜间机、两门副砲 → 连击且标记为 0；带夜间航空队、同样两门副砲 → 普通夜间航空攻击、标记为 1；
  魚雷切入失败不回落到两门副砲的连击。
- 成长公式搬到 `Codex::stat_at_level`，`cal_ship_status` 与 `bare_asw` 共用。
- 既有的两处偏差（主砲加魚雷算连击、带主砲时切入失败回落到连击）加了 `ponytail:` 标记，不在这次范围。
- 没有改的：选靶测试留在 `night.rs`（依赖那里的测试辅助函数）；两个四行的镜像分支。

## Risks

- **素対潜的取法。** `api_taisen[0]` 含装备与装备加成，不能直接用。先看 codex 里舰的裸数值是怎么算的
  （`codex/ship.rs` 按等级从 `api_tais` 成长），取同一个值；取不到就退到 `mst.api_tais[0] > 0`，
  并在计划里记下与来源的差别（来源是当前等级的素対潜）。
- **带种子的测试移位。** U1 不改抽签次数，只改候选集合；敌方有潜艇且我方有夜间空母的现有测试会变。
  U3 让带两门副砲的舰多打一次，夜战基线若含这样的舰会变。落空时换种子、不放宽断言；基线变了先查是谁再决定重新冻结。
- **Decision 2 让一类舰少了出手的机会**：带回転翼機的正规空母夜战不再打潜艇；敌方只剩潜艇时她不出手。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、
  `cargo test --workspace --no-fail-fast`。
- sim→validate 门不变（现有预设里没有潜艇加夜间空母的组合）。无头检查不另加场景：这次不改报文的形状，
  只改选谁；连击与对潜的演出路径已有场景覆盖。
