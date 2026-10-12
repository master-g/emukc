---
title: "Special Gunnery Attacks Against the Source - Plan"
type: fix
date: 2026-10-12
status: implemented
execution: code
---

# Special Gunnery Attacks Against the Source - Plan

计划 2026-10-12-008「核对时发现、没有改的出入」六条与「没有做」的夜战发动，用户逐项定了全部要做
（2026-10-12）；Colorado 的 SG レーダー(後期型) 发动率 +5% 定为只算一次，保持现状。规则沉淀在
`docs/solutions/architecture-patterns/day-shelling-order.md` 的「The flagship's special gunnery attacks」，
这里记证据、与原代码的差别、实施单元与结果。

## Evidence

来源是 `KC3Kai/kancolle-replay` `69097abc` 的 `kcsim.js`，wiki 是 2026-10-12 取的 wikiwiki 各舰娘页。

| 项 | 来源 | wikiwiki | 原代码 |
| --- | --- | --- | --- |
| 报文 | `apiAdjustHougekiSpecial` 把各发并成一行 | — | 每艘参加舰一行 |
| 旗舰损伤 | `HP/maxHP <= .5` 不发动 | 小破以下 | 要过 75% |
| 僚舰损伤 | `<= .25` 不发动；Nelson 的三、五番舰不看 | 中破以下；Nelson「参加艦が大破していても」 | 要过 50% |
| 舰数 | 活着的非潜水艦 6 艘以上 | 水上艦6隻 | 不看 |
| 105、106 | `[ships[0], ships[0], ships[1]]`，两艘都在名单里即可 | 旗舰与二番舰都是该级，3 連続の砲撃 | 一、二、三番舰各一发，三番舰要是戦艦；106 要低速 |
| 101、102 | 旗舰 541 是 101、573 是 102 | 長門/陸奥改二が艦隊旗艦 | 只认 541，二番舰是 573 时发 102 |
| 101、102 倍率 | 1.4 / 1.2，按二番舰再乘 | 同 | 二番舰 1.1，只有 573 一档 |
| Nelson Touch T不利 | `mod = 2`，再 `*= 1.25` | 2.5 倍 | 2.0 × 2.5 |
| 目标 | `shellPhaseTarget(ship, alive2, [])`，不含潜水艦 | 特殊攻撃で潜水艦を狙わない | 选到潜水艦就丢掉这一发 |
| 命中 | 100 是 1.05，其余 1.4，徹甲弾、水上電探各再 1.15 | — | 没有 |
| 夜战 | 100–103、105、106 的分支不看 `isNB` | 夜戦でも発動する | 只在昼战 |
| 弹药 | `updateSupply`：101、102、105 的两艘 ×1.5，104 按版本 1.1–1.3 | 長門型、Richelieu 追加半分；金剛型 1.2 | 没有 |
| PT 小鬼群 | `fleet.isNoSpecialNode` | 敵がPT小鬼群のみのマスでは発動しない | 没有 |

**客户端（`main.decoded.js`）：** 昼战与夜战的 `PhaseHougeki._nelson_touch`、`_nagato_cutin`、`_mutsu_cutin`、
`_colorado_cutin`、`_richelieu_cutin`、`_qe_cutin` 都从一行读 `d_indexes[0..3]` 与 `getDamage(0..2)`。

## 取舍

- **弹药的名单照来源，金剛型的数照 wikiwiki。** 来源只对 101、102、105 收，Colorado（103）不收；
  wikiwiki 的 Colorado 页写三艘「長門型改二の特殊砲撃と同じく」追加消费，没有给数。这里没有收 103，留作未定。
  金剛型取 wikiwiki 的 1.2（与倍率取 wikiwiki 的 2.6 / 2.7 一致），来源最新一档是 1.1。
- **弹药的基数是这里已有的「每场 20%」。** 来源与 wiki 的基数分昼、夜、開幕夜戦与敌联合舰队，取整也各不相同
  （wikiwiki：通常艦隊切上、Richelieu 切捨）；这里只在已有的基数上乘系数、向下取整。
- **Warspite、Valiant 弹药为 0 不发动**（wikiwiki，注明不知是否各种共通）没有做。
- **敌方不发动**没有另加判断：敌舰的 id 不在任何名单里。

## Implementation Units

- **U1** 一行报文（`fix(battle): write a special attack as one row`）。
- **U2** 无头场景 `nagato_broadside` 与检查（`test(headless): …`）。
- **U3** 资格、倍率、目标、命中（`fix(battle): take who makes a special attack from the source`）。
  测试 `who_makes_each_attack_and_what_her_hits_are_multiplied_by`、`how_damaged_the_ships_of_an_attack_may_be`、
  `a_fleet_of_fewer_than_six_surface_ships_makes_no_special_attack`、`the_hits_of_an_attack_are_one_row_on_surface_ships`。
- **U4** 夜战发动（`feat(battle): roll for the special gunnery attacks at night`）：金剛型的出手循环抽成
  `special_attack_row`，昼战六种经 `special_gunnery_at_night` 走同一行。测试
  `a_special_gunnery_attack_is_made_at_night_unless_the_sortie_has_made_it`。
- **U5** 弹药与 PT 小鬼群（`feat(battle): charge a special attack's ammunition, …`）。测试
  `the_ammunition_of_a_special_attack_is_its_ships_alone`、`the_ships_of_a_special_attack_pay_more_ammunition_for_the_battle`
  （经 `sortie_battle` 与 `sortie_battle_result`）、`no_special_attack_is_made_against_pt_boats_alone`、
  `no_special_attack_is_made_at_night_against_pt_boats_alone`。
- **U6** 文档。

## 改写与删掉的旧测试

旧的资格测试断言的是原来的规则，删掉 12 条、由 U3 的四条取代：`nelson_touch_detection`、
`nelson_touch_no_equipment_bonuses`、`nelson_rodney_bonus`、`rodney_flagship_also_triggers_nelson_touch`、
`nagato_broadside_detection`（断言二番舰 1.1）、`nagato_mutsu_detection`（断言 541+573 是 102）、
`nagato_mutsu_flagship_produces_two_hits`、`richelieu_detection`（断言三艘）、`richelieu_requires_jean_bart`、
`qe_detection`、`qe_fast_fleet_rejected`（断言低速条件）、`valiant_flagship_triggers_qe`。
`a_special_attack_is_made_once_and_costs_only_the_flagship_her_turn` 改成断言旗舰一行三发、僚舰另有一行；
#78 的三条带舰队的测试把舰队补到六艘。

## 实施记录（2026-10-12）

- U1–U6 做完，分支 `fix/special-attack-one-row`，接在 #78 的分支上。
- PT 的两条测试看过去掉实现后失败；其余（U3、U4 与弹药的两条）是和实现一起写的，没有逐条先看失败，
  期望值取自上表的来源与 wikiwiki。
- **基线没有变：** 两类战斗基线与 `battle_golden.rs` 原样通过；基线里没有能打特殊砲撃的旗舰。
- `make headless-check SCENARIO=nagato_broadside` 在 U3 之后跑了六次：五次在昼战发动（一行三发），一次在夜战发动
  （只剩一艘敌舰，一发），都没有页面错误，没有清单外的资源；昼战发动后进了夜战的两次都没有再发动。
  U3 之前（一行、种别还是 102）的两次也没有页面错误。`kongou_night_assault` 重跑两次通过。
- 顺带修了 #78 里一条偶发失败的测试（`a_night_battle_counts_the_kongou_assault_against_the_sortie`，
  运 99 时发动率只有 81%），已放回 #78 的分支。

## Risks

- **特殊砲撃变强也变容易发动：** 旗舰小破、僚舰中破仍发动；105、106 不再需要第三艘戦艦；
  夜战多一次判定；命中有了 1.4 的修正。
- **也有变弱的地方：** 少于六艘水上艦不发动；Nelson Touch 在 T字不利是 2.5 倍，原来是 5 倍；
  長門型、Richelieu 与金剛型的参加舰多扣弹药。
- 長門改二 + 陸奥改二的种别从 102 变成 101，客户端演的是長門的切入。
