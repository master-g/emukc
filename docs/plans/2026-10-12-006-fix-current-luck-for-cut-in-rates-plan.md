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

**除数是错的。** wiki 的式子算出来直接就是百分比：長門 Lv99 运 40、陸奥 Lv99 运 16，
`30 + 2√99 + 1.2(√40 + √16) = 62.3`，kankorekore 写 62.2%；Nelson 全员 Lv99、运 24，式子得 62.7，
wikiwiki 的留言说运不改修的 Lv99 大约六成。代码把这个数再除以 140（Nelson、長門）或 130（Colorado、Richelieu、QE），
長門・陸奥的例子只剩 44.5%。140 与 130 是 `docs/battle/research.md` 里昼战切入的种别係数（主砲CI 140、主副CI 130），
不是特殊砲撃的数；原计划（`archive/2026-05-14-002`）里的两条式子也没有除数。

## Decision

1. 五处 `api_lucky[1]` 改成 `api_lucky[0]`。
2. 除数改成 100：式子的值就是百分比。不取整（wikiwiki 的式子没有取整；英文 wiki 的版本有）。
3. Colorado、Richelieu、Queen Elizabeth 三条没有出处的式子删掉，改用長門型的式子（旗舰与二番舰的等级与运，加 30）。
   这是用户定的处理，依据是 wikiwiki 对 QE 的推测；按这条式子 Warspite改（运 70）与 Valiant改（运 40）都 Lv99 得 67.5%，
   与 wikiwiki QE 页写的 67.5% 相同。Colorado 与 Richelieu 没有这样的对照，是照搬。

## Scope

**不做：** Colorado 的 SG レーダー(後期型) 加 5%（wikiwiki 有记载，原来也没有实现）；金剛型（104）与大和型（400、401）的
特殊攻击没有实现，不在这次范围里。

## Implementation Units

- **U1** `special_attack.rs` 四个发动率函数。测试 `the_trigger_rates_read_the_current_luck_not_what_it_can_be_modernised_to`：
  运 `[10, 100]` 的舰队与 `[10, 10]` 的四个发动率相同，都低于 `[100, 100]`。
- **U2** `day_cutin.rs` `day_ci_trigger_rate`。测试 `the_day_cut_in_chance_reads_the_current_luck_not_what_it_can_be_modernised_to`：
  运 `[9, 100]` 与 `[9, 9]` 相同，低于 `[100, 100]`。

- **U3** 除数与三条式子。测试 `the_trigger_rates_are_the_percentages_the_wiki_gives`：長門・陸奥 62.2%、
  Warspite改・Valiant改 67.5%、Nelson 全员 Lv99 在 60%–65% 之间。

## 实施记录（2026-10-12）

- U1–U3 做完，分支 `fix/current-luck-for-cut-in-rates`，两个提交（先运，后除数与式子）。U1、U2 的测试在改之前失败、改之后通过。
- **基线没有变：** `crates/emukc_battle/tests/golden/day_seed_01..20.txt`、`night_seed_01..20.txt` 与
  `tests/gameplay_tests/battle_golden.rs` 原样通过。运只移动抽签的门槛，不改变抽签的次数与先后；基线里的舰队没有能打特殊砲撃的旗舰。

## Risks

- **没把运改修到上限的舰，昼战切入的发动率下降。** 运已满的舰与敌舰（两个值相同）不变。
  例：大和改二（运 18，上限 108）昼战切入的 `floor(√运)` 从 10 变成 4。
- **特殊砲撃的发动率整体上升**（除数的影响大于运的影响）。都按 Lv99、运不改修算：

  | 特殊砲撃 | 原来（读上限、带除数） | 只改运之后 | 现在 |
  | --- | --- | --- | --- |
  | 長門改二 + 陸奥改二 | 52.6% | 44.5% | 62.3% |
  | Nelson（僚舰 Lv99） | 49.3% | 44.8% | 62.7% |
  | Colorado改 + Maryland改 | 55.7% | 51.9% | 64.9% |
  | Richelieu改 + Jean Bart改 | 47.3% | 42.9% | 61.4% |
  | Warspite改 + Valiant改 | 48.3% | 46.1% | 67.5% |

- **Colorado 与 Richelieu 的式子是照搬長門型的**，没有各自的出处；二番舰的运现在也算进去。
