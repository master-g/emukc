---
title: "Air Corps against an Enemy Combined Fleet - Plan"
type: fix
date: 2026-10-19
status: implemented
execution: code
---

# Air Corps against an Enemy Combined Fleet - Plan

Issue #242。来源 `kancolle-replay@69097abc`：`kcsimcombined.js:816-830` `sim6vs12`：基地航空队在航空战之前出击，目标为 `alive2.concat(alive2C)`，即敌方两层舰队。

**范围**：U1–U2，一个 PR。
**非目标**：我方连合舰队带基地航空队（6-4、6-5 不能以连合出击）；喷气机强袭（`api_air_base_injection`）；源码对连合舰队目标的 ×1.1（`airstrikeLBAS` `postMod`，记作目标外问题）。

**基线**：`fix/asw-synergy`（#243）之上。

## Implementation Units

### U1 出击与封包

- `simulation/mod.rs`：`simulate_day_enemy_combined` 在航空战前，按 `simulate_day` 的同一闸门（有基地航空队才进）调用 `execute_air_base_attacks`；没有基地航空队的战斗随机数序列不变。
- `combined_packet.rs`：`remap_day_packet_enemy` 对每个 `air_base_attack` 的 `kouku` 调 `split_enemy_kouku_stage3`。

### U2 测试

- battle crate：敌连合 3+2 艘、4 中队一式陸攻，20 个种子下主力与护卫各被打中过，stage3 两段长度 3 与 2。
- gameplay：6-5 M（cell 18）带基地航空队的 `ec_battle` 响应含 `api_air_base_attack`，两段 `api_edam` 各 6，`validate_day_battle_response` 无错误；转储后 `battle validate` 报 0 错误 0 警告。
- golden：180 份与 `battle_golden.rs` 未变（敌连合 golden 无基地航空队）。
