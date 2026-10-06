---
date: 2026-10-06
topic: equipment-bonus-sources
---

# 装備ボーナス的数据来源（调研）

## 结论

装備ボーナス（特定舰装特定装备时的属性加成）在真实游戏里由服务端算进 `api_karyoku`、`api_sakuteki` 等显示值。
本项目没有这一层。要补，推荐的做法与路由规则同构：**用 KC3Kai 的声明式表作数据，用客户端 `main.js` 里的同一套逻辑对拍**。
两个来源都是现成的，不需要抓任何网页。

本文只做调研，不是计划。实施前另立 `docs/plans/` 计划。

## 为什么是服务端的事

客户端从 2020-03-03 起在装备界面显示加成明细，这段逻辑硬编码在 `main.js` 的 `SlotItemEffectUtil` 里；
但舰船的属性值本身仍由服务端下发，已经含了加成（KC3Kai `GearBonus.js` 头部注释：
"Explicit stats visible bonuses ... are added to API result by server-side"）。所以落点是
`crates/emukc_model/src/codex/ship.rs` 的属性组装（`apply slotitem boost` 那一段之后），不是路由。
路由的索敌得分用 `los_now − Σ装备索敌` 当素索敌，属性里有了加成，它就自动落在根号内，
`crates/emukc_gameplay/src/game/sortie/route.rs` 那条 `ponytail:` 注释可以删。

## 来源一：客户端 main.js（权威，可作对拍）

- 位置：本仓库已解码，`main-decoder/out/modules/module-82692-slot-item-effect-util.js` 是分发入口，
  按装备类别（`get_type3_nums`）和装备 id（`get_slotnums`）调 291 个效果函数，分布在约 280 个
  `module-*-get-slot*-personal-effect.js` 里。解码版本 6.3.5.0。
- 输入模型 `module-73785-slot-item-effect-param-model.js`：`ship_id`、`yomi`、`ctype`、`getCountryName()`、
  `get_slotnums(id)`、`get_type3_nums(type3)`、`get_each_level_nums` / `get_each_level_over_nums`（改修）、`get_have_rader_nums`。
- 输出模型 `module-74496-slot-item-effect-model.js`：`houg`、`raig`、`tyku`、`souk`、`kaih`、`tais`、`saku` 等，带 `add` / `multiply`。
- 其中 128 个模块写到了 `saku`。
- 形态是任意 JS（嵌套三元、局部变量、提前 return），不像羅針盤源码那样可以白名单转换。
  它适合**被执行**而不是被转换：在 Bun 里加载这些模块，喂一艘舰和一组装备，得到加成。
- 权威性：这就是游戏显示加成用的代码。局限：只含「可见」加成；隐藏的命中 / 回避补正不在其中。
- 国籍来自 `getCountryName()`，客户端按 `ctype` 推出，不需要另找国籍表。

样例（Fairey Seafox改，id 371）：ゴトランド `houg+4 tais+2 kaih+3 saku+6`，改二（630）另加一次性 `houg+2 kaih+2 saku+3`；
`ctype` 70 `saku+4`；`ctype` 79 `saku+3`；イギリス舰 `saku+3`，其中 `ctype` 88 另加一次性 `saku+2`；按装备数量倍乘。

## 来源二：KC3Kai GearBonus.js（声明式，可作数据）

- `KC3Kai/KC3Kai`，`src/library/objects/GearBonus.js`，MIT。2026-10-06 时最新提交 `ee4d7dfb`（2026-09-11，
  "Update visible bonuses"），仓库 2026-09-29 仍有推送。348 KB，356 个装备条目，122 处 `saku`。
- 结构：装备 id → `byClass` / `byNation` / `byShip` → `multiple`（按数量倍乘）或 `single`（一次性），
  限定词有 `remodel`、`minStars`（990 处）、`minCount`、`excludes`、`distinctGears`，另有 `synergy`（237 处，
  与电探、鱼雷等其他装备的组合加成），组合用的装备 id 列表在文件头的 `synergyGears`。
- 与来源一的样例逐项一致（上面 371 的五条规则在两边数值相同）。
- 它是 JS 对象字面量加注释，不是 JSON；用 Babel 取 `explicitStatsBonusGears` 的返回值即可，`main-decoder` 已有这套工具。
- 本仓库已经在用同一组织的数据（`res.rs` 拉 `KC3Kai/kc3-translations`）。

## 其余来源（不推荐作主数据）

| 来源 | 状态 | 评价 |
| --- | --- | --- |
| 羅針盤シミュ `src/data/equipBonus.ts`（MIT，已钉住） | 28 种装备、60 条规则 | 只有索敌，且只收作者关心的装备；可作索敌一项的第三方对照 |
| `noro6/kc-web` `src/classes/item/ItemBonus.ts` | 2026-09 仍在更新 | 仓库没有许可证，不能取用 |
| `TeamFleet/WhoCallsTheFleet-DB`（MIT） | 最后推送 2024-03 | 停更 |
| en.kancollewiki.net / Fandom 的 Equipment Bonuses 页 | 人读表格 | 前者被 Cloudflare 挡，后者是散文表 |
| npm `equipment-bonus` | 2021-03 | 停更 |

## 建议的实施形态（留给计划）

1. `main-decoder` 加一个提取器：钉住 KC3Kai 的提交，取出表，转成中性 JSON 资产（装备 id、条件、各属性加成）。
2. Rust 侧在属性组装处套用，七项属性一起做，不单做索敌。
3. 对拍：Bun 里加载解码后的 `SlotItemEffectUtil`，对「每个有规则的装备 × 它点名的舰 + 若干不相干的舰 × 数量 1–3 × 改修 0 / 满」
   逐项比较七项属性。差异要么是 KC3Kai 落后于客户端，要么是提取器的错。
4. 行为变化会波及战斗数值（火力、对空进了伤害公式），`battle_golden.rs` 的舰队若带这类装备需要有意重冻。

## 未决问题

- 对拍要在 Bun 里构造 `SlotItemEffectParamModel`，它的构造函数读客户端的舰船 / 装备模型；需要看清最小桩有多大。
- `main.js` 每次发版都会变，KC3Kai 通常滞后几天；`make update` 之后对拍会报出这段时间差里的新规则，算预期内的差异还是失败要定。
- 只做可见加成。隐藏补正（命中、回避、对特定敌的特效）没有客户端代码可对拍，不在此列。

## 量级参考

单就索敌：ボーナス在根号内，单舰得分增量 `√(s+b) − √s`，通常一支舰队偏差 0.3～1.5 分，
极端约 4 分，方向固定为本项目偏低。算式与样例见计划 `docs/plans/2026-10-06-002-fix-map-los-formula-33-plan.md` 的实施记录。
