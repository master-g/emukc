---
title: "Opening ASW Rules - Plan"
type: feat
date: 2026-10-22
status: implemented
execution: code
---

# Opening ASW Rules - Plan

Issue #271。来源 `kancolle-replay@69097abc`：`kcships.js:425`（`alwaysOASW`）、`:1368-1369`（`canASW`/`canOASW`）、`:1858`、`:1899-1906`、`:1932-1943`、`:2092-2116`、`:2184-2226`；`kcSHIPDATA.js`（敌方 `alwaysOASW`、646 的覆盖、`isASWlast`）；`shared.js:220`（`isPlayable`）。

**基线**：`main`（#270 之后）。
**非目标**：白天炮击的「能否攻击潜艇」仍走 `can_attack_submarine_day_shelling`（#285 合并）；深海舰缺对潜值（#284），名单上的敌舰补齐数据前多数过不了 `canASW`。

## 事实

- 玩家舰 `ASWbase ≥ 50`（554 除外）与 codex `ship_extra.tais[0] ≥ 50` 筛出的 26 艘完全一致（2026-10-22 对拍）；实现仍取来源的并集（U1）。
- 敌方名单在来源 1501–3000 区间共 71 艘、ID 和 144657；`slot_items` 含补强增设槽（`setup.rs:427`）。
- `damage::base_asw` 对敌舰也成立（`targeting::bare_asw` 对敌舰返回 0），作 `statsBase.ASW` 用。

## Implementation Units

### U1 名单

- `scripts/gen_always_oasw.py` → `crates/emukc_battle/src/simulation/always_oasw.rs`（`PLAYER_ALWAYS_OASW_IDS` 24 艘、`ABYSSAL_ALWAYS_OASW_IDS` 71 艘），壳照 `gen_abyssal_night_carriers.py`，打印条数与 ID 和；测试断言二者。CLAUDE.md「禁止文件」加一行。来源对玩家舰是「明写」与「1 级对潜 ≥ 50（554 除外）」的并集，另有测试钉住 codex 判出的 26 艘（ID 和 17208），防 kcwiki 的 1 级数值与来源 `ASWbase` 漂移。

### U2 `can_asw`

- `targeting.rs` 移植来源 `canASW`，只给 OASW 用：DD/CL/CLT/CT/AO/DE 看 `base_asw > 0`；CAV/AV 敌舰看 `base_asw > 0`，我方看 7/8/11/25/26/41 型、对潜 > 0、有机数的机体；BBV 411/412 带 15 型装备即可，否则同 CAV；AS/AR 带 15 型；LHA 727/945 带 15 型，否则同 CAV；其余 false（CVL、646、554、DE 走 U3 各自的规则，不问它）。

### U3 `can_opening_asw`

- 先按 ID：554（326/327 任一，或 69/324/325 两件）、646 走 CVL；再按舰种：DE（声纳且 60，或 75 且装备对潜和 ≥ 4，不经 `canASW`）、CVL（来源 `:2100-2116`，第一步看装备存在不看机数；`isASWlast` 508/509）；其余 `can_asw && (always || ASW ≥ 100 && 声纳 && 玩家舰)`。`always`：玩家舰为明写名单或 `tais[0] ≥ 50`（554 除外），敌舰读 U1。

### U4 测试

- 按具体舰 ID：五十鈴改二无声纳、DE 75/4、CVL 50（有声纳）与 65、鈴谷航改二有声纳仍要 65、日向改二两件 S-51J 以外的组合、扶桑改二带爆雷、敌舰在名单内外、codex 判出的玩家舰与来源 26 艘一致。现有 `oasw_*` 测试在新规则下仍成立，未改。

### U5 文档

- 新开 `docs/solutions/architecture-patterns/opening-asw.md` 写规则；`docs/plan.md` Gap #4 标已解决。

### 验证

- fmt、严格 clippy、`cargo test --workspace`；golden 若变，说明原因。
