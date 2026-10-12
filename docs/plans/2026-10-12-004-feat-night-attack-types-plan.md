---
title: "Night Attack Types Against the Source - Plan"
type: feat
date: 2026-10-12
status: implemented
execution: code
---

# Night Attack Types Against the Source - Plan

规格在 GitHub issue #53（问题、用户故事、范围都在那里）。实现照 issue 的 Implementation Decisions 做，
这里只记 issue 里没有的东西：对来源核出的证据、规格没有写明的几处、实施单元、改写的旧测试与结果。

## Evidence

**来源（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcsim.js` `NBATTACKDATA`：连击 1、通用切入 2–5、驱逐舰切入 7–14 共 13 行（倍率、命中补正、发动率系数、次数、
  是否带魚雷、80 级可换成的两次版本与概率）。第 7、8 行的字面量就是 `nbattack7New`/`nbattack8New`；
  `toggleDDCIBuff` 能换回旧值，来源里默认是新值。
- `kcships.js` `NBtypes`（1407 行）：驱逐舰四种各自入列，然后一条 else-if 链取一种通用种别。
  主魚電与魚見電要的是**水上電探**（`btype == B_RADAR && LOS >= 5`），不是任意電探；ドラム缶是装备 75 本身。
- `kcships.js` `NBchance`（1445 行）与 `kcsim.js` `NBattack`（739–741 行）：运 50 以上时 `sqrt(运-50)` 也取整；
  两种見張員各算各的（412 限驱逐、轻巡、雷巡，129 不限舰种），可以叠加；中破写作 `HP/maxHP <= .5`。
- `NBattack` 745–790 行：按列出的顺序逐种抽签。80 级以上的驱逐舰**先**抽「换成两次版本」，再用换过之后那一行的
  系数抽发动（7→11 的系数由 1.15 变成 1.3，其余三种不变）。打陆上目标时，7–14 没有主砲就跳过，带魚雷的 2、3
  在主砲副砲合计两门时改成连击，否则跳过。潜水艦的魚雷切入与 D 型砲的倍率是这个函数里的字面量，不在表里。
- `nightPhase`（1658 行）：照明弾在夜战开始时每支舰队抽一次，带照明弾且血量大于 4 的舰逐艘抽 70%，
  第一艘抽中的发射，位置写进 `api_flare_pos`。
- `kcEQDATA.js`：标 `specialCutIn` 的魚雷 8 件（213、214、383、441、443、457、461、512）；潜水艦電探是装备种别 51。

**客户端（`main.decoded.js`）：** 夜战 `PhaseHougeki._special` 把种别 2 交给 `PhaseAttackSpSR`，只取两件展示装备，
两次伤害相加显示；`CutinSpSR` 把第一件画在中间、第二件画在上下，并为两件都加载名牌。所以种别 2 的
`api_si_list` 是「主砲、魚雷」。`api_flare_pos` 由 `bannerGroupLayer.getBanner(side, index)` 读，是舰队内的位置，
联合舰队的第二舰队从 6 起。

**wikiwiki 夜戦页（2026-10-12 取）与来源不同的三处，数值以来源为准：**

| 项 | 来源 | wiki |
| --- | --- | --- |
| 魚水魚（种别 9、13）的种别係数 | 125 | 126 |
| 主魚電打两次（种别 11）的种别係数 | 130（先抽两次，再按 130 抽发动） | 只有 115 一个数 |
| 连击的发动率 | 99% | 「概ね95%以上」，要検証 |

倍率、次数、装备条件、各项补正（旗舰、中破、探照灯、照明弾、見張員）、驱逐舰四种的先后，两边一致。

## 规格没有写明的几处

1. **照明弾原来没有实现。** `api_flare_pos` 一直是 `[-1, -1]`。这次按来源补上发射判定，并写进报文；
   只有舰队里有带照明弾的舰时才抽随机数。照明弾对命中的 +5 属于命中公式，在范围外，没有动。
2. **中破的判定沿用 `is_chuuha`（四分之一以上、一半以下）。** 来源写 `<= .5`，而来源里四分之一以下的舰夜战不出手
   （`canNB`），所以在来源里两者等价。这里空母以外的大破舰仍会出手（`can_attack_night_ship` 的既有行为，没有动），
   她们不加 18。
3. **空母切入的发动率跟着统一的公式变。** 种别 6 的子类、倍率、系数都没动，但来源里它与其它切入共用 `NBchance`，
   所以空母切入现在也吃探照灯、照明弾、見張員与取整后的运。
4. **运改读 `api_lucky[0]`。** 原来读的 `[1]` 是运的改修上限（`codex/ship.rs`），来源用的是当前的运；审查指出后改正。
5. **潜水艦在夜战不出手**（`can_attack_night_ship` 对 SS/SSV 返回 false，既有行为）。潜水艦的魚雷切入照来源做了并有
   单元测试，但在实战里走不到，用户故事 11 要等「谁在夜战出手」这条规则对照来源重做之后才生效。

## Decision

- 表由 `scripts/gen_night_attack_table.py` 从钉住的 `kcsim.js` 与 `kcEQDATA.js` 生成进
  `simulation/night_attack_table.rs`（13 行与 8 件特殊魚雷）；`NightAttackType` 的倍率、命中补正、次数、
  发动率系数都读这张表，空母切入照旧读自己的子类。`MainTorpRadar` 改名 `MainTorp`。
- `detect_night_attack_type` 只取一种通用种别；`night_attack_types` 在它前面放驱逐舰的四种。
- `night_cut_in_chance` 一个函数取代原来通用与驱逐舰两套公式。
- `resolve_night_attack` 照 `NBattack` 逐种抽签，返回种别与倍率（D 型砲与潜水艦魚雷会改倍率）；
  切入失败回落到连击的分支删除，连击自己抽 99%。
- `roll_star_shells` 在夜战炮击之前抽照明弾，结果经 `NightBattleParams.flare_pos` 进发动率，
  经 `finalize_night` 进报文；联合舰队的位置在 `combined_packet.rs` 里与其它索引一起换算。

## Scope

**不做**（issue 的 Out of Scope）：夜間瑞雲切入、舰队特殊攻击、夜战命中公式本身、敌方专属规则。

**沿用而不修的：** 切入或连击没打中时来源给擦伤，这里是 0（伤害公式的事）；空母在来源里可以同时列出几种切入
各自抽签，这里只取一种；潜水艦用「電探加一条後期魚雷」组成魚雷切入时只有一件魚雷可展示。

## Implementation Units

- **U1** `scripts/gen_night_attack_table.py`、`simulation/night_attack_table.rs`。
  测试 `the_night_attack_table_follows_the_source`：13 行对脚本打印的字面量，特殊魚雷的件数与 id 之和。
- **U2** `night.rs` `NightGear`、`detect_night_attack_type`、`night_attack_types`。
  测试 `a_ship_has_one_general_attack_type`、`a_destroyer_rolls_her_own_cut_ins_first_and_in_the_source_order`。
- **U3** `night.rs` `night_cut_in_chance`。测试 `the_cut_in_chance_follows_the_source`。
- **U4** `night.rs` `resolve_night_attack`、`d_gun_modifier`。测试 `a_failed_cut_in_is_a_plain_attack`、
  `a_destroyer_of_level_80_may_make_her_cut_in_twice`、`the_d_type_guns_raise_…`、
  `late_model_torpedoes_raise_a_submarine_torpedo_cut_in`、`a_torpedo_cut_in_is_not_made_on_an_installation`。
  这些测试用一个只发给定随机数、多抽一次就报错的 RNG，所以抽签的次数与先后也在断言里。
- **U5** `roll_star_shells`、`NightBattleParams.flare_pos`、`finalize_night`、`combined_packet.rs`。
  测试 `the_first_ship_to_roll_under_seventy_fires_her_star_shell`。
- **U6** 展示装备：种别 2 是主砲、魚雷两件。测试 `the_packet_carries_a_mixed_cut_in_as_type_two_with_the_gun_first`。
- **U7** 预设 `night_cut_in`（六艘驱逐舰各带一种装备，旗舰带照明弾）、`PhaseExpectation::NightCutIn`、
  夜战门的断言、无头场景与它的检查。
- **U8** 文档：`night-attack-types.md`（新）、`carrier-night-air-attack.md` 的已知缺口、
  `headless-client-check.md`、`CLAUDE.md`、`PROJECT_MEMORY.md`。

## 改写的旧测试（都在 `night.rs`）

| 测试 | 原来断言 | 现在 |
| --- | --- | --- |
| `night_ci_torpedo_torpedo_detects_as_torp_ci` | 魚雷切入 1.3 倍 | 1.5 倍 |
| `night_double_attack_with_main_and_torpedo` | 主砲加魚雷是连击、1.2 倍 | 改名 `a_main_gun_with_a_torpedo_is_a_cut_in_without_a_radar`：主魚切入、两次、1.3 倍 |
| `dd_ci_non_dd_does_not_trigger` | 轻巡带主魚電是 `MainTorpRadar` | 是 `MainTorp` |
| `dd_ci_gtr_detection`、`dd_ci_trl_detection` | 任意小型電探 | 换成 22号対水上電探（13号対空電探的索敵是 3，不算） |
| `two_secondary_guns_make_a_double_attack` | 调用旧签名 | 调用新签名，断言不变（失败不回落到连击） |
| 发动率的 14 条（`night_ci_*`、`dd_ci_*` 的运、旗舰、中破边界） | 两套公式各测一遍 | 经测试里的 `trigger_rate` 走统一公式，断言不变 |
| `carrier_night_ci_enum_multiplier_coefficient_sp_list` | `ci_coefficient` 105/120/130 | `chance_modifier` 1.05/1.2/1.3 |

## 实施记录（2026-10-12）

- U1–U8 做完，分支 `feat/night-attack-types`。
- **基线：** `crates/emukc_battle/tests/golden/night_seed_01..20.txt` 全部重新冻结。这 20 场是两艘
  榛名对两艘山城改二（按初期装备），四艘都组成连击。原来的连击不抽随机数，现在每次出手多抽一次（99%），
  之后的随机数全部移位；旧基线里的种别全是 1，新基线里是 1 与少数没抽中的 0，没有出现切入。`tests/gameplay_tests/battle_golden.rs` 没有变化。
- `make headless-check SCENARIO=night_cut_in` 通过：没有页面错误，没有缺失资源；那一次夜战里演了种别 2
  （展示 12.7cm連装砲、61cm四連装(酸素)魚雷）、种别 5、种别 7，照明弾由旗舰发射（`api_flare_pos [0, -1]`）。
  种别 3、4、8–14 与潜水艦的魚雷切入没有在客户端里演过。
- `gunnery_cutin` 与 `night_air_attack` 两个无头场景重跑通过（没有页面错误，没有缺失资源）。

## Risks

- **带切入装备的舰夜战结果变了：** 魚雷切入变强（1.3→1.5），主砲加魚雷从 99% 的连击变成要抽签的切入，
  切入失败只剩一发普通攻击。
- **敌舰同表：** 敌方驱逐舰带主砲、魚雷与水上電探时也会打驱逐舰切入，80 级以上的还会抽两次。
- **切入的发动率普遍下降一些：** 运从改修上限改成当前值，未满运的舰都受影响。
- **照明弾与探照灯现在影响发动率：** 敌方的照明弾会让我方切入明显变少（−10）。
