---
title: "Special Attack Ammunition - Plan"
type: fix
date: 2026-10-12
status: implemented
execution: code
---

# Special Attack Ammunition - Plan

计划 2026-10-12-009 留在 `day-shelling-order.md` Unsettled 里的三条，用户要求先找资料、尽可能还原线上
逻辑（2026-10-12）。三条都找到了来源以外的实测，各改一处。

## Evidence

来源是 `KC3Kai/kancolle-replay` `69097abc` 的 `kcsim.js`；网页都是 2026-10-12 取的。

| 项 | 来源 | 其它资料 | 原代码 | 现在 |
| --- | --- | --- | --- | --- |
| Colorado（103）弹药 | `updateSupply` 只列 101、102、105 | wikiwiki Colorado「長門型改二の特殊砲撃と同じく参加した艦3隻には弾薬の追加消費が発生する」；zekamashi「1番/2番/3番艦の弾薬消費が5割増し」；agurikera 2019-06-13「弾薬を1.5倍消費」（昼 30%、開幕夜戦 15%、夜戦 45%）；kankorekore 早见表「追加消費：弾薬，Nelson級＝なし」 | 不收 | 一、二、三番舰 ×1.5 |
| 金剛型（104）弹药 | `kongouSpecialBuff5 ? 1.1 : kongouSpecialBuff ? 1.2 : 1.3`，`kongouSpecialBuff5: true` | 运营 2026-04-23 公告「4)消費微down」；@kobabu2424 2026-04-25 開幕夜戦一场 13 → 发动后 14；zekamashi「2026/04/23…1.2倍→1.1倍」；wikiwiki 金剛改二丙正文仍是 2021 年的 1.2，评论区 2026-04-26 有人贴了 1.1 | 1.2 | 1.1 |
| Q.E. 级（106）弹药为 0 | 没有这个判断 | wikiwiki Warspite改 发动条件第 6 条「Warspite/Valiantの少なくとも一方の弾薬残量が0ではないこと」；cc-jabberwock 2026-02-21：弹药为 0 时连续 6 次不发动，理论发动率 67.5%，偶然的概率 0.12% | 不看 | 旗舰或二番舰 `api_bull` 为 0 时不发动 |

金剛型那条实测对得上 1.1（按結婚舰算）：金剛改二丙弹药 160，開幕夜戦 10% 是 16，×0.85 取整得 13；发动后
⌊16×1.1⌋=17，×0.85 得 14；若是 1.2 则是 ⌊19.2⌋=19，×0.85 得 16。2026-04-23 那次更新的另外两项（一次出击
三回、中破也能发动）计划 2026-10-12-007、008 已经做了；火力与发动率两项见「没有做」。

## 取舍

- **103 不照来源。** 来源漏了 103，三份日文资料一致说收、其中两份给了 1.5，照资料。
- **106 的弹药门只给 106。** wikiwiki 自己写「これが独自仕様なのか、タッチ全体の共通仕様なのかは不明」，
  其余六种没有人测过，不推广。
- **106 收不收弹药没动。** 来源不收，wikiwiki 的 Warspite改 页没有「攻撃コスト」一节，kankorekore 的表只把
  Nelson 级列为例外；没有一份资料给过数，维持不收。

## 没有做

- 弹药消费仍是在「每场扣 20%」上乘系数。线上按昼战 20%、昼战后夜战 30%、開幕夜戦 10% 分档，取整方式
  因敌方是否联合舰队而不同，对敌联合舰队昼战发动后进夜战时追加消费被抵消（wikiwiki 長門改二「攻撃コスト」、
  来源 `updateSupply` 的 `didNB`、`!isECombined || !didNB`）。这是基础消耗本身的缺口，不在这三条里。
- 核对时看到、没有改：2026-04-23 那次更新的五项里，中破可发动（25% 门槛）、三回、弹药 1.1 已经对上；
  基础倍率仍是 2.6／2.7，来源 `kongouSpecialBuff5` 一档是 2.76／2.86（来源自注 estimate，对应公告的
  「3)火力微up」，zekamashi 的页面仍写 2.6／2.7）；发动率式子仍是更新前 wikiwiki 的那条，「2)発生確率微up」
  之后没有人给出新式（kankorekore：検証データがあまりありません；来源的发动率是用户填的）。

## Implementation Units

1. `special_attack_ammo`：103 的前三艘 ×1.5。
2. `special_attack_ammo`：104 的两艘 ×1.1（原 1.2）。
3. `check_qe_attack`：旗舰或二番舰弹药为 0 时不成立。

## Verification

- `the_ammunition_of_a_special_attack_is_its_ships_alone`（两次看过红：103 得到 `[1.0; 6]`、104 得到 1.2）。
- `who_makes_each_attack_and_what_her_hits_are_multiplied_by` 新增两条弹药为 0 的断言（看过红：得到
  `Some((106, …))`）。
- 报文不变，没有重跑无头场景。
