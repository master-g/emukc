---
title: "Special Attacks: Formations and the Limit a Sortie - Plan"
type: fix
date: 2026-10-12
status: implemented
execution: code
---

# Special Attacks: Formations and the Limit a Sortie - Plan

计划 2026-10-12-007 留下的两件事：特殊砲撃的出击级次数限制，与 `special_attack.rs` 里写成 3 的梯形陣。
用户的要求是先找资料、尽可能还原线上的逻辑。这里记证据、做了什么、以及核对时发现但没有改的出入。

## Evidence

**来源（`KC3Kai/kancolle-replay` `69097abc`，`kcsim.js` `canSpecialAttackUnique`）：**

| 种别 | 陣形 | 次数 |
| --- | --- | --- |
| 100 Nelson Touch、105 Richelieu | `formation.id` 是 2 或 12 | `fleet.didSpecial` 为真就不发动 |
| 101、102 長門型、103 Colorado、106 Queen Elizabeth | 4 或 12 | 同上 |
| 104 金剛型 | 1、4、12、14 | `fleet.numSpecialKongou >= 3` 才不发动，不看 `didSpecial` |

- `didSpecial` 记在舰队上：发动时置 1，战后 `updateSupply` 改成 2，都是真；只有重置舰队（`kcships.js` 132 行，
  新的一次出击）才清掉。所以是每次出击一次，六种共用一个标记。
- 六种都有 `ship.isescort` 就不发动的判断（104 除外，她在第二艦隊）；`ship.side == 1`（敌方）一律不发动。
- 100–103、105、106 的分支都不看 `isNB`，夜战里旗舰的回合同样判定。

**wikiwiki（2026-10-12 取，長門改二、Nelson、Colorado、Richelieu改、Warspite改、金剛改二丙 六页）：**

- 陣形：長門型、Colorado、Queen Elizabeth 是「梯形陣(連合ならば第二警戒航行序列)」，Nelson、Richelieu 是
  「複縦陣(連合ならば第二警戒航行序列)」。与来源一致。
- 次数：「発動するのは出撃1回につき1度のみ」，五页相同；金剛型「出撃1回毎にビッグセブンや大和型改二の特殊砲撃とは別に、
  最大3回発動可能」。
- 「演習では発動しない」，六页相同。
- 「夜戦でも発動する(つまり最大3回の判定回数)」：通常艦隊昼战两巡加夜战，連合艦隊两次。
- `kankorekore.2-d.jp/touch/` 的早见表与上面逐项相同（発動回数 1回、金剛型＝3回(独立カウント)）。

## Decision

1. **陣形。** 梯形陣的常量改成 4，另加第二警戒航行序列 12，五个判定都认「自己的陣形或 12」。
2. **第二艦隊的旗舰不发动。** 原来靠陣形对不上挡着；认了 12 之后，`shell_turn` 对第二艦隊那一段也是 `idx == 0`，
   所以按舰船自己的所属标记挡掉（来源的 `isescort`）。
3. **每次出击一次。** `emukc_battle::SpecialAttacksMade { gunnery, kongou_assaults }` 记这次出击已经发动过什么：
   `ActiveSortieState` 保存，经 `BattleContext` 与 `NightBattleInput` 交给战斗，战后从报文里我方的攻击种别
   （昼战 `api_at_type`、夜战 `api_sp_list`）回写。新的出击新建 `ActiveSortieState`，自然清零。
   `SortieStore` 只在内存里，没有加表。
4. **金剛型每次出击三回，单独计数。** 已经三回就不判定、不抽随机数。
5. **演習不发动。** 两处都按舰船的 `is_sortie` 挡。

## Scope

**没有做：昼战那六种在夜战里发动。** 证据两边都有（见上），但现在昼战的报文本身有问题（见下一节第 1 条），
夜战要照同一个格式发，应当先把那个定下来。次数状态已经能挡住「昼战发过、夜战再发」，夜战接上时不用再改这一层。

## 核对时发现、没有改的出入

都在 `special_attack.rs`，与这两件事无关，但同样与线上不符：

1. **一次特殊砲撃发成了几行。** `execute_special_attack` 每艘参加舰记一行，每行都带种别 100–106。
   客户端昼战 `PhaseHougeki._nelson_touch`、`_nagato_cutin` 等从**一行**里读三个目标与三次伤害
   （`d_indexes[0..3]`、`getDamage(0..2)`），所以现在一次長門型一斉射会被演成两次切入，每次只有一两发有数。
   这是读代码得出的，没有在客户端里跑过；现有的无头场景里没有特殊砲撃。金剛型（104）是照一行做的。
2. **损伤门槛偏严。** 代码要旗舰血量过 75%、僚舰过 50%；来源与 wiki 是旗舰「小破以下」（过 50%）、
   僚舰「中破以下」（过 25%），Nelson 的三、五番舰大破也参加。
3. **没有「水上艦 6 隻」。** 来源要活着的非潜水艦 6 艘以上，代码不看舰数（金剛型的 5 艘有）。
4. **Richelieu（105）与 Queen Elizabeth（106）的参加舰不对。** 来源与 wiki 都是旗舰两发、二番舰一发，
   旗舰可以是姐妹里任一艘；代码是一、二、三番舰各一发，三番舰要求是戦艦，Richelieu 只认 Richelieu 当旗舰，
   Queen Elizabeth 还要求低速舰队（没有出处）。
5. **長門型只认長門改二当旗舰。** 来源与 wiki 是長門改二或陸奥改二。
6. 弹药的追加消费（長門型、Richelieu ×1.5，金剛型 ×1.2）与「只有 PT 小鬼群的格子不发动」都没有做。

## Implementation Units

- **U1** 陣形与第二艦隊的守卫（提交 `fix(battle): take the special attacks' formations from the source`）。
  测试 `the_formations_are_the_echelon_the_double_line_and_the_second_cruising_formation`（陣形 1–6、11–14 逐个断言，
  用字面量）、`the_flagship_of_the_escort_fleet_makes_no_special_attack`（第二艦隊旗舰是長門改二、陣形 12，40 个种子）。
- **U2** 次数。战斗层测试 `a_special_attack_is_made_once_a_sortie_and_never_in_an_exercise`、
  `the_kongou_night_assault_is_made_three_times_a_sortie_and_never_in_an_exercise`；出击层测试
  `a_sortie_makes_its_special_gunnery_attack_once`（经 `sortie_battle`，1-1，梯形陣）、
  `a_night_battle_counts_the_kongou_assault_against_the_sortie`（经 `run_night_battle`，第三回发动、第四回不发动）。
- **U3** 文档：`day-shelling-order.md`、`night-attack-types.md`、`PROJECT_MEMORY.md`。

## 实施记录（2026-10-12）

- U1–U3 做完，分支 `fix/special-attack-sortie-limits`，两个提交。
- 每条测试都看过失败：陣形测试把常量改回 3 失败；第二艦隊测试在加守卫之前种子 0 就发动；
  次数的四条分别在去掉 `from_context` 的初值、去掉演習判断、去掉出击层的读写之后失败。
- **基线没有变：** 两类战斗基线与 `battle_golden.rs` 原样通过。基线里没有能打特殊砲撃的旗舰，新加的判断都在抽随机数之前。

## Risks

- **長門型、Colorado、Queen Elizabeth 的特殊砲撃换了陣形：** 輪形陣不再发动，梯形陣开始发动；
  連合艦隊在第二警戒航行序列下五种都开始发动。
- **一次出击里特殊砲撃少了：** 以前每场战斗都可能发动一次，现在整次出击一次，演習不再发动。
- 出击状态只在内存里，服务重启后正在进行的出击本来就丢，次数跟着丢。
