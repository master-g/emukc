---
title: "The Kongou Class's Night Assault - Plan"
type: feat
date: 2026-10-12
status: implemented
execution: code
---

# The Kongou Class's Night Assault - Plan

金剛型改二丙的特殊攻击【僚艦夜戦突撃】（`api_sp_list` 104）。用户直接提出，没有 issue。沉淀文档是
`docs/solutions/architecture-patterns/night-attack-types.md` 的「The Kongou class's night assault」，
规则都写在那里；这里记证据、两处来源之间的分歧、实施单元与结果。

## Evidence

**来源（`KC3Kai/kancolle-replay` `69097abc`，`kcsim.js`）：**

- `canSpecialAttackUnique` 的 104 分支：只在夜战（`isNB`）、只有我方（`ship.side == 1` 直接返回）、旗舰、
  活着的非潜水艦 5 艘以上、陣形 `[1,4,12,14]`、旗舰与二番舰血量都超过四分之一、各旗舰的僚舰名单。
- `getSpecialAttackShips`：`[ships[0], ships[1]]`，两发。`nightPhase` 里两艘各自 `nightPhaseTarget(…, [], …)` 选水面舰，
  再由 `apiAdjustHougekiSpecial` 并成一行；二番舰之后照常轮到自己。
- `getSpecialAttackMod`：旗舰与僚舰的倍率、T有利 1.25 / T不利 0.8、砲的倍率、命中 1.4；乘在 cap 之前（`preMod`）。
- **没有发动率**：`SIMCONSTS.kongouSpecialRate` 是 `null`，算出来是 0。

**wikiwiki 金剛改二丙 页（2026-10-12 取）：** 概要表（僚舰、编成、损伤、陣形、次数、倍率、装备）与发动率推定式
`⌊3.5(√旗艦Lv+√僚艦Lv) + 1.1(√旗艦運+√僚艦運) + A + B − 33⌋`，A 是旗舰的水上電探（索敵 8 以上），B 是大型探照灯，
按旗舰是谁取值：金剛 30/10、比叡 10/30、榛名乙 15/0、榛名丙 20/0、霧島 20/20。页面的验算：金剛 Lv99 运 18 +
榛名乙 Lv99 运 48 得 48。

**客户端（`main.decoded.js`）：** 夜战 `PhaseHougeki._kongo_cutin`（种别 `0x68`）把一行里至多三个目标交给
`PhaseKongoAttack`；攻击方不读 `api_at_list`，直接取 `deck_f.ships[0..2]`（联合舰队取 `ships_sub`），不读装备。
`CutinKongoAttack._getVoiceId` 按旗舰与僚舰播旗舰的语音 900、990–993。源站探测：591、592、593、954 各有 990–992，
694 有 990–993，下一个编号都是 404，与客户端代码一致。

## 两处来源的分歧，以及取哪边

| 项 | 来源 | wikiwiki | 取 |
| --- | --- | --- | --- |
| 基础倍率 | 2.76 / 2.86，自己标了 `estimate`（上一档是 2.6 / 2.7） | 2.6 / 2.7 | wiki |
| 砲 A 的 id | `[502, 529]`，529 是 12.7cm連装砲C型改三H（驱逐砲） | 35.6cm連装砲改二 / 改三(ダズル迷彩仕様)，即 329、502 | wiki，来源多半是笔误 |
| 发动率 | 没有 | 推定式 | wiki |
| 僚舰名单、陣形、5 艘、大破不发动、砲 B、交战形态 | 两边一致 | | |

## Decision

- 放在夜战路径（`night.rs`）：`kongou_assault_rate`（资格与发动率，不满足返回 `None`）、`kongou_assault_multiplier`、
  `kongou_night_assault`（在我方旗舰的回合里先试，发动就发一行并跳过旗舰自己的攻击）。伤害复用
  `calculate_night_damage` 的 cap 前倍率，命中 `with_modifier(1.4)`。
- **不满足资格时不抽随机数**；满足且敌方有水面舰时先抽一次发动率，发动后两舰各自：选目标 → かばう → 命中 → 伤害。
- 陣形集合直接写 `[1, 4, 12, 14]`，没有引用 `special_attack.rs` 的常量（见 Risks 的第一条）。
- 缓存清单：`voice.rs` 的 `SPECIAL` 加 16 个语音。

## Scope

**不做，记为已知缺口：**

- **每次出击最多三回。** 战斗模块没有跨战斗的状态；一场夜战旗舰只有一个回合，所以是每场一次。
  昼战的特殊砲撃同样只做到「每场一次」，而实机是「每次出击一次」，#68 把它们的发动率提到六成多之后这个差别变大了
  （2026-10-12 用户转来的提醒）。出击级的次数限制要在 sortie 的状态里记，单独定。
- 发动的那场战斗弹药消耗 ×1.2。现有的特殊砲撃也都没有做弹药。
- 只有 PT 小鬼群的格子不发动（来源的 `isNoSpecialNode`）。

## Implementation Units

- **U1** 资格与发动率。测试 `the_kongou_night_assault_rate_is_wikiwikis`（48、带電探 78、再带大型探照灯 88；
  索敵 5 的電探与普通探照灯不算；比叡是 58 与 78）、`…_needs_its_pair_five_surface_ships_and_a_formation`。
- **U2** 倍率。测试 `…_multiplier_follows_the_place_the_engagement_and_the_guns`。
- **U3** 夜战里的一行。测试 `…_is_one_row_of_two_hits_and_spares_the_partner_her_turn`：种别 104、两发、旗舰只有这一行、
  僚舰另有自己的一行、敌方不发动、换掉僚舰不发动。
- **U4** 预设 `kongou_night_assault`、无头场景与检查、`voice.rs` 的 16 个语音。
- **U5** 文档：`night-attack-types.md`、`headless-client-check.md`、`CLAUDE.md` 的场景名单、`PROJECT_MEMORY.md`。

## 实施记录（2026-10-12）

- U1–U5 做完，分支 `feat/kongou-night-assault`。
- **基线没有变：** 两类夜战基线原样通过。基线里的旗舰是榛名改二（151），不在名单里，所以一个随机数也没有多抽。
- `make headless-check SCENARIO=kongou_night_assault` 跑了七次：四次发动（一次两发，三次因为旗舰击沉了最后一艘敌舰只有一发），
  都没有页面错误；其余三次没有发动或僚舰已大破，检查按设计报「再跑一次」。
- 第一次两发的那次查出缓存清单缺切入的语音 `991.mp3`；补进 `SPECIAL` 并重新生成清单后，清单外与回源都是空。
- 测试是和实现一起写的，没有逐条先看失败；发动率与倍率的期望值取自 wikiwiki 的验算与表。

## Risks

- **`special_attack.rs` 的 `FORMATION_ECHELON = 3` 是既有的错。** 梯形陣是 4（`targeting.rs::escort_shield_rate`、
  来源的 `formation.id != 4`），3 是輪形陣；所以長門型、Colorado、Queen Elizabeth 的特殊砲撃现在在輪形陣发动、
  梯形陣不发动，联合舰队的警戒航行序列也不认。这次没有改，单独处理。
- 带这对舰的舰队夜战明显变强：旗舰的回合变成两发 2.6 / 2.7 倍的攻击。
- 发动率的推定式只有 wikiwiki 一个出处。
