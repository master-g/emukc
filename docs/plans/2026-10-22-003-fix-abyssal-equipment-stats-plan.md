---
title: "Abyssal Equipment Stats - Plan"
type: fix
date: 2026-10-22
status: implemented
execution: code
---

# Abyssal Equipment Stats - Plan

Issue #287。来源 `kancolle-replay@69097abc`：`kcsim.js:4132-4147`（`createDefaultShip` 以 `loadEquips(…, true)` 加装备）、`kcships.js:536-546`（加 FP/TP/AA/AR/EV/ASW/LOS/LUK）、`kcships.js:1000`（`hasTorpStat`）、`kcships.js:1033-1038`（`statsBase` = 总值减装备与装備ボーナス）。

**基线**：`main`（#284 之后，codex 已含 `abyssal_asw.json`）。
**非目标**：
- 码典深海舰本体值与来源的差异（如駆逐ラ級 装甲 158/169、回避与索敌 0/140、0/100，kcwiki 缺）：889 艘中回避 706、索敌 635、运 750 艘码典为 0 而来源有值（部分是 `unknownstats` 推定），HP/火力/雷装/对空/装甲各差 7–12 艘。另开工单。
- `fleet_los`（`shelling.rs`，弾着観測的舰队索敌）敌我共用，取总索敌；来源 `fleetLoS` 取本体索敌加侦察机「索敌 × floor(√机数)」。改后敌舰计入全部装备索敌（駆逐ラ級的 深海SG +12 来源不计），改前只有本体，而码典本体索敌 635 艘为 0。两者都偏，正解是敌我一起移植 `fleetLoS` 并补本体数据，会动我方，另开工单（审查时提出）。
- 演习对手的总值不含装备：`build_practice_enemy_ships` 用 `new_ship`（`api_slot` 全 -1）后直接 `cal_ship_status`，装备与装備ボーナス都没加，对手打得轻、`base_asw` 偏低。另开工单。
- 演习对手的 `api_eParam`：真实抓包（`z/snapshot/2026-09-22/practice_battle.json`）是不含装备的本体值（霧島改 火力 89，装备另加 47），本项目发总值。另开工单。
- 运。深海装备没有运（157 件全为 0）；我方 `cal_ship_status` 也不加装备运。

## 事实

- codex `start2` 的深海舰条目与 `enemy_ship_extra` 的火力、雷装、对空、装甲 889 艘逐艘相同（2026-10-22 对拍），夜战空母火力（`night_air_power`）读的就是它。
- 来源的雷击参战看本体雷装（`hasTorpStat`）。本项目三处判定读总雷装 `api_raisou[0] > 0`：敌舰加装备后，本体 0 的深海空母会因舰攻（深海棲艦攻 雷装 4）参加闭幕雷击；我方赤城带九七式艦攻（总雷装 5）现在就会参加（实测）。
- 来源的加重対空对敌舰是 `sqrt(this.AA)`，`this.AA` 含装备：`weighted_anti_air` 读总值，改后正确。

## Implementation Units

### U1 雷击判定看本体雷装（单独提交）

- `targeting::has_own_torpedo`：舰体雷装（start2 `api_raig[0]`，深海舰即本体值）加近代化改修，即来源 `TP - tpEquip`。开幕、闭幕、`attack_capability_for_phase` 三处改用它。先试过「总值减装备与装備ボーナス」（`base_asw` 写法），但演习对手由 `new_ship` 构建、`api_slot` 为空，总值本不含装备，千歳甲/千代田甲（本体 15、甲標的 24）会被判成没有雷装，故改读舰体（审查时发现）。
- 我方随来源变化：本体雷装 0、雷装全来自装备的舰（如带舰攻的空母）不再参加雷击。
- 测试：赤城带九七式艦攻不参加闭幕雷击，駆逐ラ級参加。三个拿 `api_raisou[0]` 当开关的测试改用真实舰：BB 例用 Bismarck drei（本体 16），AV 例用千歳甲（本体 15，「无甲標的」一例先清空自带装备），删去与「本体 0 的 BB」重复的 `zero_raisou` 例。
- golden：只改这一处时 1/180 变（`day_seed_04`）。day 组夹具把榛名、山城改二（本体雷装都是 0）的 `api_raisou[0]` 设成 120、10，旧判定让它们打闭幕雷击；该种子的闭幕雷击消失，敌舰存活、`midnight_flag` 变 1，其余种子炮击阶段已分胜负。闭幕雷击仍由各深海组的驱逐、轻巡覆盖。

### U2 敌舰数值加装备

- `Codex::new_enemy_ship`：火力、雷装、对空、装甲、回避、索敌与对潜一样取本体加装备，`[总, 总]`。
- `api_eParam` / `api_eParam_combined` 给本体值：`response::enemy_params` 对深海舰读 `enemy_ship_extra`，其余（演习对手、缺数据的兜底舰）照旧。`build_day_response`、`build_night_response`、`with_enemy_escort`、`with_enemy_decks` 收 `&Codex`。
- 测试：駆逐ラ級（2264）火力 178、雷装 176、对空 180、装甲 158、回避 24、索敌 12（本体加两门 Mk.30+GFCS、一具 5連装魚雷+深海SG、一具 Hedgehog）；日战响应的 `api_eParam` 为 `[158, 158, 148, 158]`；軽母ヌ級（1510）总雷装 4 但不参加闭幕雷击。既有测试按新口径改：kcwiki 覆盖测试删去火力/装甲断言（改成本体加装备就是照抄实现，数值由 2264 的测试钉住），`enemy_ship.rs` 两处对空 40 → 43。
- golden：`crates/emukc_battle/tests/golden` 另有 73/180 份变化，全在敌方为深海舰的六组（ctf 19、ec 19、te 15、stf 13、single 4、sp_midnight 3）；敌方为我方舰的 day/night/practice 不变。`tests/gameplay_tests/battle_golden.rs` 不变：1-1 那场敌舰唯一一次攻击是 miss，我方两击的命中与伤害也未受影响。

### U3 文档

- `battle-attack-type.md` 的雷击判定改写；`opening-asw.md` 的 Known gaps 删除；新增 `abyssal-ship-stats.md`。

### 验证

- fmt、严格 clippy、`cargo test --workspace`。
