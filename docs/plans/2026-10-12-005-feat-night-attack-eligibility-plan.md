---
title: "Who Attacks at Night Against the Source - Plan"
type: feat
date: 2026-10-12
status: implemented
execution: code
---

# Who Attacks at Night Against the Source - Plan

规格在 GitHub issue #64（问题、用户故事、范围都在那里）。实现照 issue 的 Implementation Decisions 做，
这里只记 issue 里没有的东西：对来源与客户端核出的证据、规格没有写明的几处、实施单元、测试与结果。

## Evidence

**来源（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcships.js` 1365 行 `Ship.prototype.canNB`：`HP/maxHP > .25 && !retreated`。`SS`（2128 行起）只改写
  `canShell` 与 `canOpTorp`，`SSV` 继承 `SS`，两者都没有自己的 `canNB`。
- `kcships.js` 1952 行 `CV.prototype.canNB`：`(nightattack && HP/maxHP > .25) || canNBAirAttack()`，
  与 `can_attack_night_ship` 的空母分支逐项相同。
- `kcsim.js` 1739 行 `nightPhaseTarget`：`canASWNight()` 为真的舰先打潜艇，否则打水面舰。
  `Ship.canASW` 默认返回 false，`SS` 没有改写它，所以潜水艦只打水面舰；敌我全是潜艇时没有目标、不出手。
- `kcships.js` 1557 行 `NBPower`：火力加雷装加改修。潜水艦唯一的专属项是打陆上目标且有对应倍率时火力加 30
  （`this.isSub ? this.FP + 30 : this.FP`），且打陆上目标不加雷装。本项目的夜战伤害没有陆上这一层（一律火力加雷装），
  属伤害公式、在范围外，没有做；潜水艦打陆上目标因此偏高，已记入 `night-attack-types.md` 的 Known gaps。
- `kcsim.js` 741 行：中破加成写作 `HP/maxHP <= .5`，而 `NBattack` 第一行就是 `if (!ship.canNB()) return false`。

**客户端（`main.decoded.js`，夜战的 `PhaseHougekiBase`）：**

- `_getNormalAttackType`：攻击方 `isSubMarine()` 时无条件返回 3，由 `PhaseAttackRaigeki` 演成雷击，不看
  `api_si_list`。那件展示装备只交给 `CutinAttack` 画名牌，为空时不加载任何东西。
- 种别 3 走 `_special` → `PhaseAttackSpRR` → `CutinSpRR`。它的 `preload` 对第二件做了空值判断，但 `_ready`
  直接读 `this._slot1.mstID` 与 `this._slot2.mstID`：`api_si_list` 只有一件时会抛异常。
- 名牌（`btxt_flat`）：8 件後期型魚雷与 4 件潜水艦電探（210、211、384、458）都在 `BTXT_FLAT_IDS` 里，不缺资源。

## 规格没有写明的几处

1. **一条後期型魚雷加潜水艦電探组成的魚雷切入，第二件展示電探。** 原来只发一件魚雷，在客户端会抛异常
   （见上）。现在魚雷不足两件时用潜水艦電探补上第二件；魚雷的取法（至多三件）不变。实机发什么没有样本，这是照客户端的读法定的。
2. **潜水艦的普通攻击不点名装备**（`api_si_list` 是 `-1`），客户端照样演成雷击。没有改。
3. **`is_chuuha` 没有动。** 它的下界（四分之一以上）在大破的舰不出手之后走不到，与来源的 `<= .5` 等价；
   用户故事 8 的不一致由出手资格本身消除。`night_ci_taiha_no_chuuha_bonus` 与 `dd_ci_taiha_no_chuuha_bonus`
   测的是大破舰的发动率，这个值现在在实战里读不到，两条测试原样保留。
4. **敌方潜水艦同表。** 敌方潜水艦在夜战也出手，只打我方的水面舰。
5. **血量用整数比较**（`hp * 4 > maxhp`），与空母分支、かばう的写法一致，和来源的浮点比较在边界上结果相同。

## Decision

- `targeting.rs::can_attack_night_ship`：删掉潜水艦返回 false 的分支，空母以外统一为 `hp * 4 > maxhp`。
  空母分支不动。选目标、威力、攻击种别都不用改：`can_attack_submarine_night_shelling` 对潜水艦返回否，
  所以她是 `SurfaceOnly`；威力走 `calculate_night_damage`；种别在 #53 里已经做好。
- `night.rs::night_attack_display_ids`：种别 3 的第二件可以是潜水艦電探。

## Scope

**不做**（issue 的 Out of Scope）：退避、潜水艦的昼战与开幕雷击、夜战的命中与伤害公式、探照灯重抽、
舰队特殊攻击里的潜水艦队攻击。`special_attack.rs` 读 `api_lucky[1]` 的问题（issue 的 Further Notes）没有核对。

## Implementation Units

- **U1** `can_attack_night_ship`。测试 `a_ship_attacks_at_night_with_more_than_a_quarter_of_her_hull`
  （驱逐、战舰、潜水艦、潜水空母在 100、26、25、0 血）、`a_heavily_damaged_ship_of_either_fleet_makes_no_night_attack`
  （经夜战模拟入口，敌我各一条）、`a_submarine_attacks_the_surface_ships_at_night_and_no_submarine`。
- **U2** 种别 3 的展示装备。测试 `a_submarine_makes_her_torpedo_cut_in_in_battle_and_shows_two_pieces`
  （经夜战模拟入口：两条後期型魚雷、一条加電探，各 40 个种子里打出种别 3、两次伤害、两件展示装备）。
- **U3** 预设 `night_asw` 的旗舰带後期型艦首魚雷(6門) 与潜水艦搭載電探；无头检查 `check_night_asw`
  加上对我方攻击的断言。
- **U4** 文档：`night-attack-types.md` 的已知缺口、`headless-client-check.md`、`PROJECT_MEMORY.md`。

## 改写的旧测试

没有。断言旧规则的测试只有空母的几条（原样通过）与上面第 3 条说的两条发动率测试（原样保留）；
没有测试断言过「潜水艦不出手」或「大破的舰出手」。

## 实施记录（2026-10-12）

- U1–U4 做完，分支 `feat/night-attack-eligibility`。
- **基线没有变：** `crates/emukc_battle/tests/golden/night_seed_01..20.txt` 与
  `tests/gameplay_tests/battle_golden.rs` 原样通过。那些战斗里没有潜水艦，也没有舰在轮到她时处于大破，
  所以血量超过四分之一的非潜水艦、非空母的舰逐位不变（用户故事 9）。
- `make headless-check SCENARIO=night_asw` 跑了六次都通过：没有页面错误，没有缺失资源。まるゆ 每次以普通攻击
  出手（演成雷击）；旗舰五次打出种别 3，展示後期型艦首魚雷(6門) 与潜水艦搭載電探（`['213', '210']`），
  一次带着 1 点血进夜战、没有出手。
- `night_cut_in` 与 `gunnery_cutin` 重跑通过；`night_cut_in` 那一次演了种别 3（两条魚雷）、5、7 与连击。

## Risks

- **夜战明显变了：** 敌我双方大破的舰都不再出手，潜水艦队开始有输出；夜战里被打到四分之一以下的舰，
  轮到她时也不出手。没有用 `battle sim` 跑一批种子看受损分布。
- **敌方潜水艦夜战出手**，对潜图的夜战会多挨几发雷击。
