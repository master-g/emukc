---
title: "Anti-Air Cut-In - Plan"
type: feat
date: 2026-10-10
status: implemented
execution: code
---

# Anti-Air Cut-In - Plan

## Problem

开幕航空战的対空射撃没有対空カットイン：`fly_through_anti_air` 的注释里一直列着它。带高射砲与電探的舰队
击坠数偏低，客户端也从不演出切入。

## Evidence

**我方现状：**

- `simulation/kouku.rs` `fly_through_anti_air` 每槽：我方防空固定多击坠 1 機，另有按比例与固定两发各
  半数命中。`BattleKoukuStage2` 没有 `api_air_fire`。
- 客户端（`main.decoded.js` 124573）读 `api_stage2.api_air_fire` 的 `api_idx` 与 `api_use_items`，用
  `deck_f.ships[api_idx]` 取舰，按 `api_use_items` 加载最多三张 `slot/btxt_flat` 名牌
  （`CutinAntiAircraft`，147063）；不读 `api_kind`，也没有 `api_air_fire_e`。
- 素材 `kcs2/img/battle/battle_cutin_anti_air.{json,png}` 在缓存里。按分类规则会被点名的装备
  （高角砲 36、機銃 16、高射装置 2、三式弾 3、対空電探 20、大口径主砲 48）在缓存里只有 574、575 两件
  新装备没有名牌。
- `docs/apilist.txt` 2194：`api_air_fire` 仅发动时存在，`api_idx` 0 基点。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- `kcsim.js` `AACIDATA`（59）：每个种别的固定击坠数 `num`、发动率 `rate`、固定射撃倍率 `mod`、展示
  装备的字母串；`orderKnown`（112）给出 53 个种别的优先顺序，下标即优先级。
- `getAACI`（2417）默认走 `aaciMultiRoll`：按舰、按该舰的种别顺序逐个看，只有优先级能胜过当前种别时
  才抽签。`toggleAACIRework` 没有调用点，所以发动率用 `AACIDATA` 字面量。
- 2555：攻击方没有攻击机（`!hasbomber`）时在判定之前返回。
- 2604：`shotFlat = floor(getAAShotFlat × AACImod)`，`shotFix = (我方或有切入 ? 1 : 0) + AACInum`。
- `kcships.js` `getAACItype`（1654）给出每舰的种别及其顺序；装备分类见 2364（图标 16 → 高角砲，
  対空 ≥ 8 → 自带高射装置，機銃対空 ≥ 9 → 集中配備，電探対空 ≥ 2 → 対空電探）。
- 航空队攻击与基地空袭里 `AACInum = 0`，不发动。

## Decision

1. 一张种别表加一次判定：只做由装备与舰型决定的种别 1、2、3（秋月型）、4、6（戦艦）、5、7、8、9、
   12、13。判定顺序、优先级、短路抽签都照来源。
2. 只有我方防空且敌方有攻击机槽时判定一次；结果作用于该阶段的每一槽。
3. 包里加 `api_air_fire`，没发动时不出现；联合舰队下 `api_idx` 与其他我方舰位一样换算。
4. 没有相应装备的舰队不多抽一个随机数。

## Scope

**不做：** 点名舰的种别（10、11、14–53）；敌方的切入；来源里摩耶改二等对种别 13 的排除（它因 10/11
而存在）；`aaResist`（部分敌机对固定击坠的抗性）；陣形補正与改修。

## Implementation Units

- **U1** `simulation/aaci.rs`：种别表、装备分类、`kinds_of`、`roll_air_fire`。测试：表对来源、各装备组合
  的种别序列、固定抽签下胜出的种别与展示装备、无装备时随机流不变。
- **U2** `kouku.rs`、`types/packet.rs`、`combined_packet.rs`：击坠计算、`api_air_fire`、舰位换算。测试：
  秋月发动时每槽至少多损失该种别的固定数，敌方防空与只有戦闘機来袭时不发动。
- **U3** 场景 `anti_air_cut_in`（2-5，首战必遇空母）进 sim→validate 门与无头检查。
- **U4** `docs/solutions/architecture-patterns/anti-air-cut-in.md`；基线若有变化则重新冻结并说明。

## 实施记录（2026-10-10）

- U1–U4 按计划做完。
- 基线：`crates/emukc_battle/tests/golden/` 的 20 份 `{:#?}` 转储各多一行 `api_air_fire: None,`
  （结构体加了字段），已用 `EMUKC_BLESS_GOLDEN=1` 重新冻结并核对差异只有这 20 行；
  `tests/gameplay_tests/battle_golden.rs` 没有变化。
- 首战有敌方空母的通常海域只有 2-5（C，100%）与 3-2（A，40%），所以场景放在 2-5。
- 无头检查实测：秋月发动种别 1 与 2 各一次，客户端演出切入与三张名牌，无页面错误、无缺失资源。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、`cargo test --workspace`。
- `make headless-check SCENARIO=anti_air_cut_in`。
