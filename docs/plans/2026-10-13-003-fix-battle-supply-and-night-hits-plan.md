---
title: "Battle Supply Cost and Night Hits - Plan"
type: fix
date: 2026-10-13
status: implemented
execution: code
---

# Battle Supply Cost and Night Hits - Plan

三条旧缺口，用户要求先找资料、尽可能还原线上逻辑（2026-10-13）：每场战斗的燃料弹药消耗只有「各扣 20%」
一档；夜战打陆上目标仍算雷装；连击或切入没打中时是 0 伤害。

## Evidence

来源是 `KC3Kai/kancolle-replay` `69097abc` 的 `kcsim.js`、`kcships.js`；wikiwiki 是 2026-10-13 取的「資材」页
「燃料・弾薬の消費」一节。

### 一、消耗

| 情况 | 来源 `updateSupply` | wikiwiki 資材 | 原代码 | 现在 |
| --- | --- | --- | --- | --- |
| 昼战 | `floor(max × 0.2) \|\| 1` | 20%／20%，切捨，不足 1 取 1 | 同 | 同 |
| 昼战后夜战 | 弹药再加 `ceil(max × 0.1)`，燃料不加 | 弹药 1.5 倍、端数切上；睦月型 3→5 | 不加 | 加 |
| 開幕夜戦 | 各 `floor(max × 0.1) \|\| 1`（`didNB = !NBonly`，不再加夜战那份） | 10%／10% | 各 20% | 各 10% |
| 对敌联合舰队进夜战 | `didNB && !isECombined` 才加 | 長門改二「もともと夜戦突入による追加弾薬消費が無い」 | — | 不加 |
| 特殊砲撃 | 整个弹药量 `floor(× 系数)`，`!isECombined \|\| !didNB` 时才乘 | 長門改二成本表：对敌联合昼战发动后进夜战，成本 0% | 只乘在 20% 上 | 照来源 |
| 結婚 | 没有 | 「海域航行中や戦闘での資材消費量の直接的な減少効果はない」 | 没有 | 没有 |

開幕夜戦一档有实测：@kobabu2424 2026-04-25 金剛改二丙（弹药 160，結婚）一场 13、带夜战突击 14，即 16 与
⌊16×1.1⌋＝17 各乘補給时的 0.85；若再加夜战那份就对不上。

来源唯一的调用处把 `isECombined` 写死成 `false`，对敌联合的两条是照函数体与 wikiwiki 的成本表做的。

### 二、夜战打陆上目标

来源 `Ship.prototype.NBPower`：目标是陆上型时 `fp = (isSub ? FP + 30 : FP) + 改修`，再乘对陆倍率、加对陆定数
（默认倍率 1、定数 0），不含雷装；其它目标是 `FP + TP + 改修`。

### 三、没打中的连击与切入

来源 `shell`（昼）与 `NBattack`（夜）同一句：命中判定失败时 `else if (da || cutin) dmg = getScratchDamage(target.HP)`，
`api_cl_list` 写 `res > 1 ? 2 : da || cutin ? 1 : res`。特殊砲撃走的是同一段（`cutin = attackSpecial`）。
`getScratchDamage` 是 `floor(HP × 0.06 + 0.08 × floor(random × HP))`，这里已有 `calculate_scratch_damage`。

## 取舍

- **取整照来源。** wikiwiki 長門改二对通常舰队写「⌈搭載弾薬 × (戦闘消費％＋コスト％)⌉」，長門改二（225）昼战发动
  是 68，来源 `floor(45 × 1.5)` 是 67；带夜战两边都是 102。Richelieu 页自己写的是切捨。照来源，记在
  `day-shelling-order.md` 的 Unsettled。
- **对陆只去雷装、给潜水艦 +30。** 三式弾、WG42、大発等的对陆倍率与定数这里昼夜都没有，这次不建。
- **普通攻击没打中仍是 0。** 只有连击、切入、特殊砲撃的那一发变成擦伤并写成命中。

## 没有做

- **按格子种类分的几档**（wikiwiki 的一览表）：对潜戦 8%／0%（1-5 等；4-1 D、4-3 C 与各图 Boss 例外）、
  空襲戦 6%／4%（6-4、6-5 是 4%／8%）、对 PT 4%／8%、レーダー射撃 4%／0%、对潜空襲 12%／6%。这里的快照
  不带格子种类，而且对潜戦的例外是按格子配的，看不出规则。
- **核对时看到、没有改：** 夜战的基本攻击力这里是「火力＋雷装＋5」，来源 `NBPower` 没有这个 5（只有昼战的
  `shellPower` 有）；夜偵的 +5／+7／+9 这里按制空状态给，来源按夜偵的命中值给；弹药量补正这里乘在攻击力上，
  wikiwiki 写的是乘在「攻击力 − 防御力」上。

## Implementation Units

1. `battle_supply_cost` 与快照的 `night_fought`、`night_start`、`enemy_combined`
   （`fix(battle): charge a battle's fuel and ammunition by when it was fought`）。
2. `night_gun_power`（`fix(battle): leave the torpedoes out of a night attack on an installation`）。
3. `cut_in_hit`，接在昼战连击、昼战切入、特殊砲撃昼夜两处与夜战攻击五处
   （`fix(battle): let a cut-in's hit that misses scratch, and write it as a hit`）。

## Verification

- `what_a_battle_costs_follows_when_it_was_fought_and_against_whom`：睦月型昼战 3、带夜战 5；金剛改二丙開幕夜戦 16、
  带突击 17；長門改二昼战发动 67、带夜战 102；对敌联合 67、进夜战 45。看过红（带夜战仍是 3）。
- `a_night_attack_on_an_installation_leaves_the_torpedoes_out`：看过红（135 对 55）。
- `a_hit_of_a_cut_in_that_misses_scratches_and_is_written_as_a_hit`：看过红（0 伤害）。
- 擦伤要多掷一次随机数，`crates/emukc_battle/tests/golden/*.txt` 有 23 个文件变了，已用 `EMUKC_BLESS_GOLDEN=1`
  重新冻结；前两个单元之后这些文件没变。`tests/gameplay_tests/battle_golden.rs` 没有变。
- 无头场景 `night_cut_in`、`gunnery_cutin` 重跑通过，没有页面错误；没有核对这两次里是否正好出现了没打中的切入。
