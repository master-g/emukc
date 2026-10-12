---
title: "Enemy Carriers Attack at Night - Plan"
type: feat
date: 2026-10-12
status: implemented
execution: code
---

# Enemy Carriers Attack at Night - Plan

规格在 GitHub issue #55（问题、用户故事、范围都在那里）。实现照 issue 的 Implementation Decisions 做，
这里只记 issue 里没有的东西：对来源核出的证据、与规格原文不同的三处、实施单元与结果。

## Evidence

**来源（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcSHIPDATA.js`：深海舰带 `nightattack` 的 132 艘（取值 1 的 86、2 的 11、3 的 35），带
  `hasBuiltInNightCrew` 的 74 艘（其中 52 艘同时是取值 1，另 22 艘轻空母没有标记，只能靠夜间航空攻击出手）。
  与本地 codex 逐艘对过：全部在 `start2` 与 `enemy_ship_extra.json` 里，舰种都是空母，74 艘都带夜间机且机数大于 0。
- `kcships.js` `CV.canNB`：`(nightattack && HP/maxHP > .25) || canNBAirAttack()`。取值 1 与 2 在模拟里没有区别，
  2 只表示演出是炮击。
- `hasNBAirGear` 只看装备种别（`equiptypesB`），不看机数；我方现有实现要求槽里还有飞机，这次不动，只有敌方不查。
- `CV.NBPower` 把 `B_NIGHTBOMBER2` 当作不计机数、系数 0.3 的夜间机；`kcEQDATA.js` 里这一类的深海机是
  夜深海艦爆(1582)、夜深海艦爆II(1618)、深海不屈艦爆 Mk.II(1651)，主数据里它们的图标是普通艦爆。
- 敌舰的 `statsBase.FP` 等于主数据的火力（`loadEquips(.., true)` 先加装备再减回去），与我方「主数据 + 近代化改修」同一个口径。
- 客户端 `_getNormalAttackType`（夜战）：空母舰种里按名字把リコリス棲姫、深海海月姫画成炮击，1055、1060 画成炮击，
  1061 带魚雷时画成雷击；其余一律画成舰载机。

## 与规格原文不同的三处

1. **取值 3 看的是被攻击一方的探照灯。** 规格与用户故事 4 写「敌方舰队自己的探照灯」。来源的 `nightPhase` 对敌舰调用
   `nightPhaseTarget(order2[i], alive1, subsalive1, slrerolls1, light1)`，`light1` 由 `alive1`（玩家舰队）算出；
   友军舰队那段（4923 行）同一个方向。也就是玩家带了探照灯，这 35 艘轻空母才打水面舰。用户已确认照来源做。
2. **「敌方总是跳到下一个能出手的舰」只在友军舰队的夜战里**（4922 行的注释）。主夜战 `nightPhase` 对敌方是
   `order2[i].canNB()`，按位置轮流，与现有的 `simulate_night_hougeki` 相同，出手顺序不改。
3. **sim→validate 门不需要放开「只有 `night_air_attack` 预设有航空攻击」。** 常规图里自带夜战要员的敌舰只在 7-4 与 5-6 的
   第四阶段，门打的是每张图的第一个格子，碰不到。新预设遇到的 軽母ヌ級flagship 没有要员，标记是 0。
   门里新加的是「有敌方空母在夜战出手」的断言。敌方的夜间航空攻击由 `night.rs` 的单元测试覆盖。

## Decision

- 两张名单由 `scripts/gen_abyssal_night_carriers.py` 从钉住的 `kcSHIPDATA.js` 生成进
  `simulation/abyssal_night.rs`；脚本打印各取值的条数与 id 之和，测试对这组数。
- `can_attack_night_ship`：我方的夜战炮击名单或深海标记，且血量超过四分之一；或者能做夜间航空攻击。
- `has_night_air_gear`：名单里的深海舰只要带夜间戦闘機/攻撃機/爆戦这类装备就算，不看机数。
  名单不进 `is_cv_night_ci_eligible`，敌方空母不出切入（Out of Scope）。
- 探照灯：夜战开始时每支舰队算一次（有没有未沉的舰带探照灯或大型探照灯），整场不变；不抽随机数。
- 取值 3 而对方没有探照灯：这一回合不出手，在选目标之前返回，不抽随机数。来源还允许她此时打潜艇
  （自身対潜大于 0 时）：来源的数据里这 206 艘中有 40 艘轻空母対潜大于 0，而 `enemy_ship_extra.json` 里敌方空母的
  対潜全是 0，`bare_asw` 读的又是玩家的表，所以这里没有一艘敌方空母在夜战打潜艇。用户故事 9 因此只做到一半：
  不该打潜艇的敌方空母不打，该打的那 40 艘也不打。记入已知缺口，要补得先有敌舰対潜的数据。
- Béarn 三个形态进 `NIGHT_SHELLING_CV_IDS`，注释标为推断。

## Scope

**不做**（issue 的 Out of Scope）：しまね丸改、敌方空母的切入、夜間瑞雲与特殊攻击、敌方装备加成与熟练度。

**沿用而不修的：** 探照灯的其它效果（吸引火力的重抽、被照的舰回避下降）没有实现，这次只读它亮不亮。
5 艘自带要员的敌舰（2041、2042、2044、2060、2061）在 `enemy_ship_extra.json` 里的机数与来源的 `SLOTS` 不同，
是上游数据的差异，没有动。

## Implementation Units

- **U1** `scripts/gen_abyssal_night_carriers.py`、`simulation/abyssal_night.rs`：两张表。
  测试 `the_abyssal_tables_follow_the_source`：条数与 id 之和对脚本的输出，每艘都是 codex 里的空母，74 艘都能飞。
- **U2** `targeting.rs` `can_attack_night_ship`、`night.rs` `abyssal_night_attack`：有标记的出手。
  测试：ヲ級flagship 出手而 ヲ級 不出手、四分之一以下不出手、リコリス棲姫出手且标记为 0。
- **U3** `night.rs` `has_night_air_gear`、`night_air_power`：敌方的夜间航空攻击。
  测试：南太平洋空母棲姫的威力手算为 498.61，报文标记 1；血量一半时不飞但仍出手，标记 0；机数为 0 仍算能飞。
- **U4** `night.rs` `simulate_night_hougeki`、`night_turn`：探照灯与取值 3。
  测试：軽母ヌ級改elite 对不带探照灯的驱逐舰不出手，对带探照灯的出手。
- **U5** `NIGHT_SHELLING_CV_IDS` 加 1055、1060、1061。测试：三艘不带装备时出手，标记 0。
- **U6** 预设 `enemy_night_carrier`（金剛与三艘驱逐舰进 2-5，被送到 C）、`PhaseExpectation::EnemyCarrierNightAttack`、
  夜战门的断言、无头场景与它的检查。
- **U7** 文档：`carrier-night-air-attack.md`、`headless-client-check.md`、计划 `2026-10-11-003` 里那句
  「敌舰没有装备槽」、`CLAUDE.md` 的场景清单、`PROJECT_MEMORY.md`。

## 实施记录（2026-10-12）

- U1–U7 做完，分支 `feat/enemy-night-carriers`。
- 两类基线都没有变化：`crates/emukc_battle/tests/golden/*.txt` 与 `tests/gameplay_tests/battle_golden.rs`
  的夜战里没有名单上的敌舰，也没有带种子的测试移位。
- `make headless-check SCENARIO=enemy_night_carrier` 通过：没有页面错误，没有缺失资源，夜战里 軽母ヌ級flagship
  的攻击演成深海舰载机（截图 `night2`）。带要员的敌舰（报文标记 1）在常规图里只有 5-6 第四阶段与 7-4，
  取值 2 的炮击演出只在 6-4，两者都没有在客户端里演过。
- 无头检查的 `check_fair_battle` 原来写死六艘舰，改成按舰队的舰数。
- issue 的 Further Notes 建议合并前用 `battle sim` 对有 ヲ級flagship 的格子跑一批种子看受损分布：没有做。

## Risks

- **夜战明显变难。** 多数常规图都有取值 1 的敌舰（1-5、1-6 起就有，2-5 的 C 格是 100%），她们过去在夜战不出手。
- **带探照灯会招来轻空母。** 5-4、5-6 等图的取值 3 轻空母只在玩家带探照灯时出手，这是来源的规则。
- **敌方的夜间航空攻击吃的是开战时的装备种别**，飞机在昼战被打光也照飞，威力里的机数项随之变小。
