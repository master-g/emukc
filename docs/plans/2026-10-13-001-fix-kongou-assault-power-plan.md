---
title: "Kongou Night Assault Power - Plan"
type: fix
date: 2026-10-13
status: implemented
execution: code
---

# Kongou Night Assault Power - Plan

计划 2026-10-12-010「没有做」里记的两项：2026-04-23 更新的「3)火力微up」与「2)発生確率微up」。用户要求
先找资料、尽可能还原线上逻辑（2026-10-13）。威力有实测，改了；发动率没有人给出新式，没改。

## Evidence

| 项 | 资料 | 原代码 | 现在 |
| --- | --- | --- | --- |
| 威力 | @hedgehog_hasira 2026-04-25（`x.com/hedgehog_hasira/status/2047906206652813322`）：1-3，金剛改二丙＋榛名改二乙，反推出一个新的乘算补正，旗舰 1.06 ≦ a < 1.06006，僚舰 1.06 ≦ a < 1.06001，与交戦形態、损伤无关；2026-04-26 追记霧島改二丙＋South Dakota改 旗舰 1.06、僚舰 1.05512～1.07093，金剛改二丙＋Warspite改 1.05999～1.14999、1.02074～1.17991。cc-jabberwock 2026-05-30 更新解说：「火力上昇はキャップ前に1.06倍」。KC3Kai `estimateKongouCutinModifier`：`(forShipPos === 0 ? 2.6 : 2.7) * 1.06`。来源 `kcsim.js` 的 `kongouSpecialBuff5`：2.76／2.86，自注 estimate，是 2.756／2.862 的舍入 | 2.6／2.7 | 再乘 1.06 |
| 发动率 | cc-jabberwock 同文：「詳細不明。微アップということで体感も調査も無理だろう」；kankorekore：「検証データがあまりありません」；@Xe_UCH 2026-01 的另一条式子已撤回（wikiwiki 金剛改二丙评论 2026-05-26）；KC3Kai 没有 104 的发动率函数；来源的发动率由用户填；英文 wiki 有人机验证，存档只到 2025-12 | wikiwiki 的式子 | 没改 |

## 取舍

- **乘 1.06，不写 2.76／2.86。** 实测区间只容得下 1.06，来源的两个数是舍入值。
- **所有配对统一乘。** 精确测到的只有金剛＋榛名与霧島旗舰；另两组的区间容得下 1.06，照 KC3Kai 统一乘。
- **发动率不猜。** 公告只说「微up」，没有数。

## 核对时看到、没有改

- KC3Kai 对所有特殊砲撃（Nelson、長門型、Colorado、金剛型、大和型、Richelieu、Q.E. 级）都要求旗舰弹药
  大于 0，注释写「suspected no trigger if remaining ammo is zero」，出处是 @CC_jabberwock 2026-02 的
  同一次 Q.E. 级实测（计划 2026-10-12-010 用的那条），不是别的舰种的独立实测。这里只有 106 有这个门。
- 发动率里榛名的电探补正：wikiwiki 与 zekamashi 是乙 15、丙 20，英文 wiki 2025-12 的存档是乙 20、丙 15；
  霧島改二丙 zekamashi 是 20／20，英文 wiki 是 20?／15～20。

## 后续核对（2026-10-13）

用户要求对上面两条再找资料。都没有改代码。

- **榛名的电探补正：代码对。** 式子的原出处是 @Divinity_123 2024-08 的 Special Attacks Test
  （`x.com/Divinity_123/status/1820114420569162214`）：「+ 15 * FS_has_radar if FS is Haruna K2B
  + 20 * FS_has_radar if FS is Haruna K2C」，即乙 15、丙 20，与 wikiwiki、zekamashi、kankorekore 和
  `KONGOU_ASSAULT_SHIPS` 一致；英文 wiki 的表把两列写反了。霧島改二丙的 20／20 有 @Xe_UCH 2025-10-14
  的实测（`x.com/Xe_UCH/status/1977928479502217357`）：电探 895/1006＝88.97%，比不带的推定高 21.21%；
  大型探照灯 880/1009＝87.22%，高 19.36%。
- **其它特殊砲撃是否要求弹药不为 0：没有新资料。** 实测仍只有 @CC_jabberwock 2026-02-09 那一次
  （`x.com/CC_jabberwock/status/2020912627766133161`，Q.E. 级弹药为 0 连续 6 次不发动）；KC3Kai 七种都加
  的条件引的就是它，并且只看旗舰；wikiwiki 写不清楚是否通用；来源 `canSpecialAttackUnique` 没有弹药判断；
  没找到 Nelson、長門型等在弹药为 0 时发动或不发动的记录。维持只有 106 有这个门。

## Verification

- `the_kongou_night_assault_multiplier_follows_the_place_the_engagement_and_the_guns` 的期望值全部乘 1.06
  （看过红）。
- 只改伤害数值，报文形状不变，没有重跑无头场景。
