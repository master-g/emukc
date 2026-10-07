---
title: "Equipment Stat Bonuses - Plan"
type: feat
date: 2026-10-06
status: implemented
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Equipment Stat Bonuses - Plan

## Goal Capsule

特定舰装特定装备时的属性加成（装備ボーナス）进舰船属性：数据取 KC3Kai 的声明式表，用游戏客户端 `main.js` 自己的逻辑逐项对拍。
调研与来源比较见 `docs/brainstorms/2026-10-06-equipment-bonus-sources.md`，本计划不重复。

## Product Contract

### Problem Frame

真实游戏的服务端把这类加成算进 `api_karyoku`、`api_sakuteki` 等显示值；本项目的 `codex/ship.rs` 属性组装没有这一层。
后果：客户端显示的数值偏低；战斗用的火力、对空等偏低；路由的索敌得分偏低 0.3～1.5 分（量级见计划 `2026-10-06-002` 的实施记录）。

### 已核实的前提

- 数据：`KC3Kai/KC3Kai` 的 `src/library/objects/GearBonus.js`（MIT），356 个装备条目；结构是装备 id →
  `byClass` / `byNation` / `byShip` → `multiple`（按数量倍乘）或 `single`（一次性），限定词 `remodel`、`minStars`、`minCount`、
  `excludes`、`distinctGears`，另有与其他装备的组合加成 `synergy`。
- 对拍用的客户端代码已在本仓库解码：`SlotItemEffectUtil` 分发到约 290 个效果函数。输入模型 `SlotItemEffectParamModel`
  的构造函数只从舰船对象读 `mstID`、`yomi`、`shipTypeID`、`getClassType()`，再加装备列表——桩很小。
  国籍由 `shipCountryModule` 按舰级推出。（模块编号每版都变，6.3.5.0 是 96115 / 98672 / 32903 / 34170；
  本计划初稿写的 82692 等来自 `out/modules` 里的旧版残留，对拍工具按导出名找模块，不依赖编号。）
- 输出模型 `SlotItemEffectModel` 的字段：`houg`、`raig`、`tyku`、`souk`、`kaih`、`tais`、`saku` 等。
- 抽查 Fairey Seafox改（id 371）的五条规则，两个来源数值一致。
- 落点：`crates/emukc_model/src/codex/ship.rs` 里 `apply slotitem boost` 之后。路由的素索敌是 `los_now − Σ装备索敌`，
  属性里有了加成它自动落在根号内。

### Key Decisions

- **KD1 七项属性一起做**，不单做索敌：只补一项会让舰船数值半新半旧。
- **KD2 数据走转换，客户端代码只用来对拍，不进运行时。** 客户端那段是任意 JS，不能白名单转换；KC3Kai 的表是声明式的。
  转换器放 `main-decoder`，钉住 KC3Kai 的提交，产出中性 JSON 资产；遇到不认识的限定词即失败（与路由规则转换器同一原则）。
- **KD3 对拍差异的处理**：KC3Kai 通常比客户端晚几天。差异逐条登记为「KC3Kai 落后」或修转换器；登记表入库，
  `make update` 后对拍出现未登记差异即失败。
- **KD4 只做可见加成。** 隐藏的命中 / 回避补正、对特定敌的特效没有客户端代码可对拍，不在此列。
- **KD5 属于数值行为变化**：新增规则不改 `codex/` 下的 `Default`，不触发 Balance Defaults Policy 的独立提交要求；
  但会改变战斗结果，golden 若受影响要有意重冻并说明。

### Requirements

- R1 带加成装备的舰，七项属性与客户端 `SlotItemEffectUtil` 的结果一致。
- R2 不带这类装备的舰，属性与改动前逐值相同。
- R3 转换是确定性的：同一个 KC3Kai 提交转两次，资产逐字节相同。
- R4 对拍覆盖「每个有规则的装备 × 它点名的舰与若干无关的舰 × 数量 1–3 × 改修 0 / 满」，未登记差异为 0。
- R5 `route.rs` 里那条说明装備ボーナス未建模的 `ponytail:` 注释删除，计划 002 的 KD6 标为已解决。

### Scope Boundaries

- 不做敌舰的加成（敌舰属性来自 `enemy_ship_extra.json`，是成品值）。
- 不做演习对手的属性修正以外的任何联动；不做装备界面的加成明细显示（客户端自己算）。

## Implementation Units

- **U1 取源与转换器**：`route-rules sync` 的同款命令取 KC3Kai 钉住提交的单个文件；`main-decoder/src/gear-bonus.ts` 用 Babel 取
  `explicitStatsBonusGears` 的返回值，展开 `synergyGears` 的 id 列表，产出中性 JSON。测试：371 的五条规则。
- **U2 对拍工具先行**：`main-decoder/src/gear-bonus-oracle.ts` 加载解码后的客户端模块，给舰和装备算加成。
  先用它量一遍 KC3Kai 表与客户端的差异规模，再决定 U3 的资产里要不要带登记表。
- **U3 资产与 Rust 套用**：`gear_bonus.json` 入库并登记；`codex/ship.rs` 在装备属性相加之后套用。
  需要舰的舰级、国籍、改造阶数——国籍表从客户端的 `shipCountryModule` 转出，不手写。
- **U4 对拍进 Makefile**：`make gear-bonus-oracle`，Rust 侧出一个不落库的「给舰 + 装备，出七项加成」的探针命令（参照 `route-rules dist`）。
- **U5 行为验证**：R2 的回归（现有舰船属性测试不变）；带 SG レーダー的美国舰索敌 +4 等三五个手算用例；golden。
- **U6 收口**：删 `ponytail:` 注释，更新计划 002、`docs/solutions/`、`PROJECT_MEMORY.md`、`THIRD_PARTY_NOTICES.md`（KC3Kai 的 MIT 声明）。

## 实施记录（2026-10-07）

**与计划的出入**

- **实施顺序**：U2 的开放问题（桩有多大）先用一个探针脚本回答了——客户端函数只需要替换 `model.slot.getMst`
  一处全局读取。随后按 U1 → U3 → U4 → U2 做，因为量差异需要先有 Rust 求值器。
- **国籍表取自 KC3Kai 的 `Meta.js`，不取客户端的 `shipCountryModule`**（U3 原文相反）。表里的 `byNation` 用的是
  KC3Kai 的国名和它自己的舰级划分，换成客户端的划分就不再是同一张表；两边划分的差异由对拍报出。
- **转换器直接写资产**，没有路由规则那样的 Rust 归一化一步：表本身已经是声明式的，转换只做别名展开和
  「单个或列表」的归一。
- **Rust 侧是 KC3Kai 读取函数的移植**（`KC3Gear.equipmentTotalStatsOnShipBonus`）。用 KC3Kai 原函数对 9.2 万次
  抽样探测逐项核对过，66 次不同，全部落在下面这处有意的偏离上。偏离：KC3Kai 只对声明了 `starsDist` 的条目看改修星数，
  其余条目的 `minStars` 会把每一件都算上；对拍显示客户端从不这样，改为始终看星数后 4 个条目（279、467、557、558）
  与客户端一致，没有条目变差；`byStars` 读没有声明 `starsDist` 的装备的星数（503、530 读 174）也照此处理。`byStars.isMultiple` 在 KC3Kai 里声明了但读取函数不读，这里照读取函数。
- **五个按类别计数的组合标志**（水上电探、对空电探、高精度电探、回转翼机、对空机枪）按主数据判定而不是按表里的
  id 列表，与 KC3Kai 读取函数一致；按列表会把 573 这类数值变过的装备算错。

**途中发现并修掉的既有问题**

- `Codex::ships_before_and_after` 对可往返改造的舰死循环（Fletcher Mk.II、Fletcher改 Mod.2、宗谷三形态、
  Glorious改 两个，共 7 艘）。此前没有调用路径碰到这些舰；加成计算要改造阶数，会让服务器卡死。
  改为先收集全部前级再取「没有任何舰改造成它」的那一艘，整条链成环时取最小 id。

**对拍结果（KC3Kai `ee4d7dfb` 对客户端 6.3.5.0）**

- 探测：每个条目 × 865 艘舰 ×（1–3 件、改修 0 / 满、与规则点名的装备搭配），只取舰装得上的搭配，共 1,193,174 次。
- 363 个条目里 82 个有差异，21,561 次探测不一致（1.8%）。全部登记在 `main-decoder/gear-bonus-known-diffs.json`，
  每条记差异次数；新增条目或次数变化即失败，`bun run gear-bonus-oracle -- --accept` 重新登记。
- 82 条的原因没有逐条看，登记为 `not looked into`。抽看的几条两类都有：新舰的规则 KC3Kai 还没有
  （三隈改二、深雪改二、吹雪改三、北上改三等），以及 KC3Kai 自己的数据错（天山村田隊在翔鶴改二上，
  它的注释写 +4，表算出来是 +3，客户端是 +4）。**R1 因此只对其余 281 个条目成立。**
- 只对拍 KD1 的七项。表里还有命中、射程、速力、爆装，资产里保留，服务端不套用。

**Open Questions 的结论**

- 桩：见上，一处。
- `make update` 后对拍失败算阻断还是告警：对拍没有接进 `make update`，是单独的 `make gear-bonus-oracle`；
  它失败不挡任何流程，只表示登记表要重看。

## Verification Contract

三道质量门与 `cd main-decoder && bun run check && bun test` 以退出码为准；`make gear-bonus-oracle` 未登记差异为 0；
同一提交连转两次 `git diff` 为空。
对拍命令：`make gear-bonus-oracle`（需要先 `make decode-main` 并重建 codex）。

## Open Questions

- 客户端模块里有没有读全局状态（如 `require(18622)`）到桩不住的程度——U2 第一步确认。
- `main.js` 发版后、KC3Kai 跟上之前的那几天，`make update` 的对拍失败算阻断还是告警。
