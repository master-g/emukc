---
title: "Damage Pipeline, Night Contact and Cell Costs - Plan"
type: fix
date: 2026-10-13
status: implemented
execution: code
---

# Damage Pipeline, Night Contact and Cell Costs - Plan

计划 2026-10-13-003「没有做」里的四处，用户要求先找资料、尽可能还原线上逻辑（2026-10-13）。前三处做了，
第四处（对陆装备的倍率）查清了范围，没有做。

## Evidence

来源是 `KC3Kai/kancolle-replay` `69097abc` 的 `kcsim.js`、`kcships.js`；wikiwiki 是 2026-10-13 取的
「戦闘について」「資材」两页。

### 一、弹药量补正的位置

| | 来源 `damageCommon` | wikiwiki「弾薬量補正」 | 原代码 |
| --- | --- | --- | --- |
| 式子 | `dmg = pow − 防御`；`ammoleft < 5` 时 `dmg *= 0.2 × ammoleft`；`floor`；`dmg <= 0` 给擦伤 | 「ダメージ = [(最終攻撃力 − 防御力) × 弾薬量補正]」「弾薬量補正は攻撃力を下げない」 | 补正乘在攻击力上，再减防御 |
| 弹药为 0 | 擦伤 | 「0%：全てカスダメ」 | 0 伤害 |
| 范围 | 砲撃、雷撃、夜戦、対潜、航空戦都走 `damageCommon` | 「弾薬量補正は航空戦にも影響する」 | 砲撃、雷撃、夜戦 |

### 二、夜战的攻击力与夜间触接

- **+5：** 来源 `NBPower` 是 `FP + TP + 改修`，wikiwiki「基本攻撃力 = 火力 + 雷装 + 改修強化値(夜戦) + 夜間触接定数」，
  都没有常数；只有昼战的 `shellPower` 加 5。原代码的 `+ 5` 是 `d4de3ff4` 抽出 crate 时带进来的，没有出处。
- **夜间触接：** 来源 `getNightEquipsScout`：舰队里每架夜偵按 `floor(√Lv × √索敵) / 25` 判定，取成功者里命中最高的
  一架。条件 `fleet.AS != -2 && (NBonly || fleet.AS != 0)`：制空権喪失不飞；航空互角（含双方都没有飞机）只有開幕夜戦
  才飞。wikiwiki 同：開幕夜戦「相互の制空値に関わらず夜偵による触接が発動する可能性がある」，「劣勢の場合…夜偵発動可能」。
- **给什么：** 来源 `NBattack`：夜偵命中 ≤1／2／≥3 时攻击力 +5／+7／+9，基础命中 ×1.1／1.15／1.2，暴击系数
  1.57／1.64／1.7（原为 1.5）。wikiwiki 的 +5／+7／+9 与机种对照（九八式夜偵、Walrus；11型乙改夜偵、Loire 130M、
  Sea Otter；Loire 130M改熟練、11型甲改二）与 `start2` 里七架夜偵的命中值一致。
- **原代码：** 攻击舰自己带夜偵就给，不判定；按制空状态分 5／7／9；没有命中与暴击；`api_touch_plane` 恒为 −1。

### 三、按格子种类的消耗

wikiwiki「資材」的一览表（2024/03/01 时点），顺序照来源 `updateSupply`：

| 格子 | 燃料 | 弹药 | 这里怎么认 |
| --- | --- | --- | --- |
| 对 PT 战 | 4% | 8% | 敌方全是 PT（`is_pt_only`） |
| 空襲戦（5-2、7-2、活动海域） | 6% | 4% | `BattleType::LdAirBattle` |
| 空襲戦（6-4、6-5） | 4% | 8% | 同上，且海域是 6 |
| レーダー射撃 | 4% | 0% | `BattleType::LdShooting` |
| 对潜戦 | 8% | 0% | 敌方全是潜水艦，且不是 Boss 格，也不是 4-1 D、4-3 C |

对潜戦的三条例外都是 wikiwiki 写的：「ボスマスは対潜戦闘マスであっても通常戦闘マスと同じ」「4-1のDマス, 4-3のCマスは
潜水艦のみ出現する道中マスでありながら燃料・弾薬を通常戦闘通り(各20%)消費する」。

## 取舍

- **对潜戦按出现的敌舰队认。** 游戏里是格子自己带标记，这里没有这份数据；按「这次遇到的敌方全是潜水艦」认，
  再把 wikiwiki 点名的两格排除。常规海域里有几格是混编（6-3 B、7-1 G、5-6 J），抽到纯潜水艦编成时这里也按
  对潜戦算，线上是否如此不知道。
- **夜偵只看还浮着的舰。** 来源 `getNightEquipsScout` 遍历舰队全部舰，不筛；这里跳过已沉的。
- **6-4、6-5 按海域认。** 来源的开关就是 `airRaidCostW6`。
- **对潜空襲 12%／6% 没做。** 这里没有这种格子。
- **弹药量补正扩到对潜与空袭。** 来源里它们走同一个结算；基地航空队没有弹药，仍不受影响。

## 第四处：对陆装备的倍率（没有做）

来源 `kcships.js` 的 `loadEquips` 里约三百行：软目标、砲台、離島、港湾夏姫、北端、集積地六组倍率，各自再分
昼战专有的一段；约三十种装备（三式弾、WG42、各种大発与内火艇、迫撃砲、噴進砲、陸軍歩兵部隊、戦車等）的计数、
改修平均值与组合加成；集積地另有 cap 后的倍率；艦爆对陆的例外。敌舰的子类（`installtype`）在来源的舰船数据里，
这里只按名字认出「陆上目标」一类。它要一份自己的计划与数据生成脚本，体量与本计划其余三处相当或更大，
没有并进来。

## 核对时看到、没有改

- wikiwiki：对陆上目标时「素手(装備を一切装備していない)の場合に限り雷装値が有効になる」，这里一律不算雷装。
- 照明弾的命中 +5 仍没有接（`AttackKind::base` 的备注）。

## Implementation Units

1. `resolve_damage` 带弹药量补正（`fix(battle): put the ammunition modifier on the damage, not on the power`）。
2. `night_gun_power` 去掉 5（`fix(battle): take the day's 5 out of the night's attack power`）。
3. `roll_night_contact`、`NightContact`、`Aim::under_contact`
   （`fix(battle): roll night contact for the fleet, and give what the source gives`）。
4. `cell_supply_cost` 与快照的 `cell_cost`（`fix(battle): charge the cells that cost otherwise what they cost`）。

## Verification

- `low_ammunition_scales_the_damage_not_the_power`：攻击力 100、防御 30、补正 0.4 得 28（看过红：10）；补正为 0 是擦伤。
- `a_night_attack_on_an_installation_leaves_the_torpedoes_out` 的期望值去掉 5（看过红：135 对 130）。
- `a_night_scout_comes_into_contact_by_the_air_and_gives_by_its_accuracy`：九八式夜偵 Lv99 的发动率 17/25，各制空
  状态下飞不飞，三档加成。写在实现之后；把互角一条规则改错后确认它失败。
- `the_cells_that_cost_otherwise` 与 `what_a_battle_costs_follows_when_it_was_fought_and_against_whom`（看过红）。
- `a_submarine_cell_costs_a_little_fuel_and_no_ammunition`：真实出击流程打 1-5 的第一格，每艘只扣 8% 燃料、弹药不变；
  去掉接线后确认它失败（扣了 20%）。
- `crates/emukc_battle/tests/golden/*.txt`：第 1 单元动了 3 个夜战文件（原来向下取整成 0 的命中变成擦伤），
  第 2 单元动了 20 个（每发夜战命中少 5），都已重新冻结；第 3、4 单元没有动。`battle_golden.rs` 没有变。
  第 3 单元没有动基线，说明基线里没有一场触接成功的夜战：这条路径只有单元测试盖着。
- 无头场景 `night_cut_in` 重跑通过、没有页面错误；它的舰队不带夜偵，夜间触接没有在客户端演过。
