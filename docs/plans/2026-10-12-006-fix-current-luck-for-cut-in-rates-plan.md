---
title: "Current Luck for the Day Cut-In and Special Attack Rates - Plan"
type: fix
date: 2026-10-12
status: implemented
execution: code
---

# Current Luck for the Day Cut-In and Special Attack Rates - Plan

来自 #64 的 Further Notes：`special_attack.rs` 读 `api_lucky[1]`，与 #53 改掉的夜战切入是同一类问题。
这里记核对的结果、改了什么、没改什么。

## Evidence

**`api_lucky` 的两个值。** `codex/ship.rs` 取主数据的 `[初始, 改修上限]`，再把运的改修值加到 `[0]` 上；
入库时 `[0]` 是 `luck_now`、`[1]` 是 `luck_max`。命中、回避（`accuracy.rs`）与夜战切入（#53）读的都是 `[0]`。

**读上限的五处：**

| 位置 | 用途 |
| --- | --- |
| `special_attack.rs` `nelson_touch_rate` | 旗舰的运 |
| `special_attack.rs` `nagato_broadside_rate` | 旗舰与二番舰的运（两处） |
| `special_attack.rs` `colorado_rate` | 旗舰的运 |
| `special_attack.rs` `three_ship_special_rate` | 旗舰的运 |
| `day_cutin.rs` `day_ci_trigger_rate` | 弾着観測射撃 |

**该读哪个：** 来源（`KC3Kai/kancolle-replay` `69097abc`）的昼战切入是 `Math.floor(Math.sqrt(this.LUK)+10)`
（`kcships.js` `ASchance`），`LUK` 是当前的运。特殊砲撃的推定式在 wiki 上，例子用的也是当前的运
（長門 Lv99 运 40、陸奥 Lv99 运 16，两艘的初始值）。

## 四条特殊砲撃发动率公式的核对（2026-10-12）

来源模拟器里**没有**这些公式：六种特殊砲撃的发动率都是可配置的定值（`SIMCONSTS`，60%，大和型 80%）。
代码里的式子来自 wiki 的推定式，所以对的是 wiki。`en.kancollewiki.net` 的页面有人机验证，取不到全文，
下面英文 wiki 的部分只有搜索结果的片段与 `kankorekore.2-d.jp/touch/` 的转述。

| 公式 | 代码里的系数 | 出处 | 结论 |
| --- | --- | --- | --- |
| Nelson Touch | `1.1√旗艦Lv + √3番艦Lv + √5番艦Lv + 1.4√旗艦運 + 25` | wikiwiki `Nelson` 页「発動率推定式」 | 系数逐项相同 |
| 長門・陸奥 一斉射 | `√旗艦Lv + √2番艦Lv + 1.2(√旗艦運 + √2番艦運) + 30` | wikiwiki 的式子（经 kankorekore 转述） | 系数相同；英文 wiki 是另一个版本（1.5、+25） |
| Colorado | `30 + √Lv×3 + 1.2√旗艦運` | wikiwiki 写「発動率計算式」未给出；英文 wiki 有带 V、W、X、Y 的式子，取不到数值 | **无法核对**，出处没有记录 |
| Richelieu、Queen Elizabeth | `20 + √Lv×3 + 1.2√旗艦運` | wikiwiki 两页都写「発動率計算式は不明」；QE 页说可能与長門型同式（运都不改修时 67.5%），英文 wiki 的片段也是两艘的等级与运、加 30 的形状 | **存疑**：代码用三艘的等级、只算旗舰的运、加 20，与两边的说法都对不上 |

**除数是错的（没有改，见 Scope）。** wiki 的式子算出来直接就是百分比：長門 Lv99 运 40、陸奥 Lv99 运 16，
`30 + 2√99 + 1.2(√40 + √16) = 62.3`，kankorekore 写 62.2%；Nelson 全员 Lv99、运 24，式子得 62.7，
wikiwiki 的留言说运不改修的 Lv99 大约六成。代码把这个数再除以 140（Nelson、長門）或 130（Colorado、Richelieu、QE），
長門・陸奥的例子只剩 44.5%。140 与 130 是 `docs/battle/research.md` 里昼战切入的种别係数（主砲CI 140、主副CI 130），
不是特殊砲撃的数；原计划（`archive/2026-05-14-002`）里的两条式子也没有除数。

## Decision

五处 `api_lucky[1]` 改成 `api_lucky[0]`。

## Scope

**不做：** 除数（140 / 130 → 100）与 Colorado、Richelieu、Queen Elizabeth 三条式子本身。前者是比这次更大的数值改动
（发动率相对多三到四成），后者没有能核对的出处；都留给单独的决定。金剛型（104）与大和型（400、401）的特殊攻击没有实现，
不在这次范围里。

## Implementation Units

- **U1** `special_attack.rs` 四个发动率函数。测试 `the_trigger_rates_read_the_current_luck_not_what_it_can_be_modernised_to`：
  运 `[10, 100]` 的舰队与 `[10, 10]` 的四个发动率相同，都低于 `[100, 100]`。
- **U2** `day_cutin.rs` `day_ci_trigger_rate`。测试 `the_day_cut_in_chance_reads_the_current_luck_not_what_it_can_be_modernised_to`：
  运 `[9, 100]` 与 `[9, 9]` 相同，低于 `[100, 100]`。

## 实施记录（2026-10-12）

- U1、U2 做完，分支 `fix/current-luck-for-cut-in-rates`。两条新测试在改之前失败、改之后通过。
- **基线没有变：** `crates/emukc_battle/tests/golden/day_seed_01..20.txt`、`night_seed_01..20.txt` 与
  `tests/gameplay_tests/battle_golden.rs` 原样通过。这次只移动抽签的门槛，不改变抽签的次数与先后。

## Risks

- **没把运改修到上限的舰，昼战切入与特殊砲撃的发动率下降。** 运已满的舰与敌舰（两个值相同）不变。
  例：大和改二（运 18，上限 108）昼战切入的 `floor(√运)` 从 10 变成 4；長門改二 + 陸奥改二（Lv99，运 40 与 16）的一斉射
  在现有除数下从 52.6% 变成 44.5%。
- **特殊砲撃的发动率因除数本来就偏低，这次又降了一些**，离 wiki 的数更远（長門・陸奥应为约 62%）。修除数之前是这个状态。
