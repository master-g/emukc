---
title: "Enemy Combined Round 3: Main Deck Fires - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# Enemy Combined Round 3: Main Deck Fires - Plan

Issue #218。来源 `kancolle-replay@69097abc`：`kcsimcombined.js` 单舰队对敌联合的第三轮炮击，敌方顺序 `order2` 只取 `ships2`（主力舰队），传给 `shellPhaseC` 的目标含 `alive2` 与 `alive2C`（两支舰队）。

**范围**：U1–U2，一个 PR。
**非目标**：友方联合舰队的各轮（#219）；联合舰队的炮击火力与命中常数；12 对 12；`hasInstall` 出场门槛（见 `target_candidates` 的 `ponytail:` 注释）。

**基线**：`main` `c2d0cd57`。

## Implementation Units

### U1 开火与挨打分开

- `ShellingRound`：`enemy_deck` 只表示开火的敌舰（顺序、索敌值）；新增 `enemy_targets` 表示我方打的敌舰。两者相同的轮次传同一个范围。
- `simulate_shelling_round`：我方的目标、下标换算与回合内的终止判断用 `enemy_targets`；敌方的攻击方下标用 `enemy_deck` 的起点。
- `shelling_round` 的提前返回看 `enemy_targets` 是否有活舰：第三轮主力全灭而护卫尚存时，我方照打，与改前在 `0..enemy_len` 下一致。源码在这种情况下的判断未核对。
- `simulate_day_enemy_combined`：三轮依次为（护卫，护卫）、（主力，主力）、（主力，全体）；#203 的 0.39 抽舰队留在 `deck_candidates`，不动。

### U2 测试与文档

- 单测：40 个种子的单舰队（含战舰）对敌联合，`hougeki3` 里 `api_at_eflag == 1` 的攻击方都在主力舰队，护卫舰出现在我方目标里。
- #203 的抽舰队测试不改。
- `ShellingRound` 字段注释与 `day-shelling-order.md` 写明谁开火、谁挨打并注明来源；golden 若动，按流程重冻并说明。
