---
title: "Special Attack Trigger Rates - Plan"
type: fix
date: 2026-10-13
status: implemented
execution: code
---

# Special Attack Trigger Rates - Plan

三条发动率的出入，用户要求先找资料、尽可能还原线上逻辑（2026-10-13）：金剛型夜战突击的式子是否偏低；
長門型与 Nelson 的式子有两个版本；Colorado、Richelieu、Q.E. 级照搬長門型。

## Evidence

主要资料是 @Divinity_123 与 fourinone 2024-08 的「Special Attacks Test」
（`docs.google.com/spreadsheets/d/1kd3t0wjLdpJZJSea7heivd-WByXHFEe1xWoUE3Kmkrg`，数据取自 TsunDB，
`x.com/Divinity_123/status/1820114418904002935` 起的一串）。表里按理论发动率分档列出实测次数与 95% 置信区间。

| 种别 | 样本 | 旧式（这里原来用的） | 拟合式 | 现在 |
| --- | --- | --- | --- | --- |
| Nelson Touch（100） | 349,722 | — | `floor(1.1√Lv1 + 1.4√運1 + √Lv3 + √Lv5) + 25`，总体 64.53% 对实测 64.53%，各档都在区间内；旗舰以外的运不起作用 | 原来就是这条，没有外层的 floor；补上 |
| 長門型（101、102） | 67,059 | wikiwiki／@kanprint 的 `√Lv1 + √Lv2 + 1.2(√運1 + √運2) + 30`：总体 63.3% 对实测 61.25%±0.37%，九档里七档出界 | `floor(√Lv1 + 1.5√運1 + √Lv2 + 1.5√運2) + 25`，总体 61.51%，各档都在区间内 | 改用拟合式 |
| Colorado（103） | 4,749 | 这里原来套長門型的式子（只看一、二番舰） | 表里直接套 Nelson 的式子：总体 62.58% 对实测 62.77%±1.37%，八档都在区间内 | 改用 Nelson 的式子，取一、二、三番舰 |
| 金剛型（104） | 10,929 | @Xe_UCH 的旧式多档出界 | `floor(3.5√Lv1 + 1.1√運1 + 3.5√Lv2 + 1.1√運2) - 33`，加旗舰的电探与大型探照灯补正；总体 62.84% 对实测 63.07%±0.90% | 原来就是这条，没改 |

KC3Kai 的 `nelsonTouchRate`、`nagatoClassCutinRate` 用的也是这两条拟合式。

其它资料：

- **Colorado 的 SG レーダー(後期型) +5%**：wikiwiki Colorado，出处 `x.com/yukicacoon/status/1751457085865496772`
  （2024-01）。保留。
- **Richelieu（105）**：表里只说它与長門型一样在空母機動部隊对通常艦隊时不发动，没有式子。
  38cm四連装砲改 deux 提高发动率的说法（@chang1124414276 2024-08-08）在 2026-08-10 被 @yukicacoon 用 POIDB
  的数据否定（`x.com/yukicacoon/status/2086695096469405967`）；这里本来就没有这个加成。
- **Q.E. 级（106）**：cc-jabberwock 2026-02-21 三组各 200 次：136、144、135 次发动（两艘 Lv99、98，运 40、70）。旧式给 67.5%，拟合式给 66%，两条都在区间内，分不出来。
- **金剛型的反例**：@Xe_UCH 2026-01-13 金剛改二丙 Lv166.7 运 82 带 SG レーダー(初期型)＋Valiant改 Lv97.9 运 40，
  571/582＝98.2%±1.11%，拟合式给 93%。他据此提的新式后来撤回（wikiwiki 金剛改二丙评论 2026-05-26）。

## 取舍

- **照表格的拟合式。** 作者自己写明「拟合得上的式子不止一条」，長門型另一条
  `floor(1.2*floor(√Lv1 + √運1 + 1.2√Lv2 + √運2)) + 26` 也拟合；这里取他们给出的第一条，KC3Kai 也是。
- **Colorado 的「ship2、ship3」读作二、三番舰**（参加的三艘）。表里只有汇总，没有写明，这是这里的读法。
  作者也说 Colorado 的数据不够拟合精确的式子。
- **105、106 跟着長門型一起变。** 它们没有自己的式子，wikiwiki 推测 106 与長門型同式；長門型换式子后它们用新的。
- **金剛型不改。** 现用的式子对一万多个样本拟合良好；反例只有一组，替代的式子已撤回；2026-04-23 的
  「発生確率微up」之后没有人给出新式（计划 2026-10-13-001）。

## 影响

長門 Lv99 运 40＋陸奥 Lv99 运 16 从 62.2% 变成 60%；Colorado 三艘 Lv99、旗舰运 30 是 63%（原来随二番舰的运变）。
发动率变了，带种子的测试里随机数的走向可能变。

## Verification

- `the_trigger_rates_are_the_fitted_formulas`（替换 `the_trigger_rates_are_the_percentages_the_wiki_gives`），
  期望值手算；看过红（長門＋陸奥得 0.6229，期望 0.60）。
- `the_trigger_rates_read_the_current_luck_not_what_it_can_be_modernised_to` 只跟着改了 `nelson_touch_rate` 的调用方式。
- 只是概率变了，报文形状不变，没有重跑无头场景。
