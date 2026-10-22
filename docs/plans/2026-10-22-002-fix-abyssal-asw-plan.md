---
title: "Abyssal ASW - Plan"
type: fix
date: 2026-10-22
status: implemented
execution: code
---

# Abyssal ASW - Plan

Issue #284。来源 `kancolle-replay@69097abc`：`kcSHIPDATA.js` 各深海舰的 `ASW`；`kcsim.js:4132-4147`（`createDefaultShip` 以 `loadEquips(…, true)` 把装备的各项数值加到舰上）、`kcships.js:536-546`。

**基线**：`main`（#271 之后）。
**非目标**：深海舰火力、雷装、对空、装甲、回避、索敌同样没算装备（`new_enemy_ship` 只取本体值）。这会让敌方打得轻、扛得少，影响面大，golden 也会变，另开 #287。白天、夜战「能否攻击潜艇」用总对潜 `> 0`，与来源 `canASW` 看本体不同，归 #285。

## 事实

- codex `enemy_ship_extra.json` 889 艘，对潜大于 0 的 44 艘；来源 1501–3000 区间 889 艘全有 `ASW`，大于 0 的 271 艘（ID 和 525470，对潜和 20073）。codex 有值的 44 艘全在其中，有 2 艘与来源不同：駆逐イ級flagship（1564）kcwiki 60 / 来源 82，駆逐ロ級後期型（1576）kcwiki 72 / 来源 80（2026-10-22 对拍）。
- codex 敌舰其余数值是来源的本体值：火力 889 艘中 880 艘相同。
- 来源给敌舰的 `this.ASW` 是本体加装备，`statsBase.ASW` 是本体；emukc 的 `api_taisen` 对我方是含装备的总值，`damage::base_asw` 拿它减去装备与装備ボーナス得本体。敌舰现在 `api_taisen` 只有本体，减装备后偏低。
- 深海舰只带深海装备，装備ボーナス表里没有这些装备，`bonus_asw` 对敌舰为 0。

## Implementation Units

### U1 资产

- `scripts/gen_abyssal_asw.py`，壳照 `gen_abyssal_night_carriers.py`，从钉住的 `kcSHIPDATA.js` 写出 `crates/emukc_bootstrap/assets/abyssal_asw.json`：1501–3000 中 `ASW > 0` 的每艘。脚本打印条数、ID 和、对潜和。`assets.rs` 登记，随 crate 嵌入；drift-check 基线收下新资产（`.sync-fingerprint.json`，资产数测试 25 → 26）。CLAUDE.md「禁止文件」加一行。

### U2 bootstrap

- kcwiki 敌舰解析：资产有值就用资产的值，没有就用 kcwiki 的值。来源与 kcwiki 不同时以来源为准，这也定了 1564、1576 两艘。之后 `cargo run -- bootstrap --codex-only` 重建 codex。

### U3 敌舰总对潜

- `Codex::new_enemy_ship` 的 `api_taisen` 取本体加装备 `api_tais` 之和，与来源 `this.ASW`、我方 `api_taisen` 同义。其余数值不动（见非目标）。

### U4 撤 `ABYSSAL_CARRIER_ASW`

- `base_asw` 不再特判敌方空母：总值减装备就是本体。`bare_asw` 对深海舰读 codex 的 `enemy_ship_extra.asw`。`gen_abyssal_night_carriers.py` 不再生成这张表，重新生成 `abyssal_night.rs`，`the_abyssal_tables_follow_the_source` 去掉 ASW 一行；`targeting::abyssal_carrier_asw` 删除。

### U5 测试

- 资产：条数、ID 和、对潜和（271、525470、20073）。
- codex：深海舰对潜大于 0 的条数等于资产条数，駆逐ラ級（2264）100、駆逐イ級flagship（1564）82。旧 codex 下会失败，与 #182 的测试一样。
- `new_enemy_ship`：带声纳的敌舰 `api_taisen` 为本体加声纳对潜，`base_asw` 得回本体。
- 现有测试按新数据有变化时，按来源改。golden 若变，说明原因。

### U6 文档

- `opening-asw.md` 的 Known gaps、`carrier-night-air-attack.md` 里 `ABYSSAL_CARRIER_ASW` 一段改写。敌舰其余数值没算装备的问题开成 #287。

### 验证

- fmt、严格 clippy、`cargo test --workspace`。
