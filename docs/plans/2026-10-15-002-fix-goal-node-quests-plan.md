---
title: "Quest Sortie Entries Keep Their Node - Plan"
type: fix
date: 2026-10-15
status: implemented
execution: code
---

# Quest Sortie Entries Keep Their Node - Plan

Issue #149。任务的出击条件里写了格子的，码典把格子丢了：以 1-6 终点 N（ゴール地点到達）计数的任务永远完成不了。
修的时候查到同一类问题的另一种写法：`7-3P`、`7-2M` 这种「海域 + 格子字母」，解析器把整个海域丢掉。用户 2026-10-15 定为一并修。

**范围**：码典保留格子；出击结算和到达不打仗的格子时按格子计数；改依赖旧行为的测试。
**基线**：`main` `34cdd584`，分支 `fix/goal-node-quests`。

## Evidence

- **上游写法两种**（`.data/temp/tsunkit_quests.json`，2026-10-15）：
  - 出击条目带 `node`：23 条任务，`1-6` + `N`（1051 写成小写 `n`），以及 966 的 `7-4O` + `O`。
  - 海域字符串末尾带格子字母：`7-2M`、`7-3E`、`7-3P`、`7-4O`、`7-4P`、`7-5T`，共 36 条任务。另有 `7-2-P1`/`7-2-P2`，这是血条阶段，已能解析。
- **码典现状**：`Sortie::parse_single_map_info` 按 `-` 切两段转整数，`3P` 转不了，整个海域被丢掉。
  33 条任务在多海域列表里少了 7 海域（如 847 要求 2-2、3-2、7-3P，码典只剩 2-2、3-2），任务变容易；
  927、966、967 的某个条目没有海域了，而 `sortie_map_matches(None)` 返回 `true`，在任何海域打一仗都算。
  `Sortie.node` 被读进来，但 `to_kc3rd_sortie` 不用它：807 成了「1-6，result Clear」。
- **匹配**：出击条件只在 `QuestActionEvent::SortieBattleCompleted` 上计数，`Clear` 要求在 boss 格以 B 以上胜利。
  1-6 N 是船団護衛成功（`event_id` 8），7-4 O 是资源格（`event_id` 2），都不打仗，所以永远不会计数。
- **格子字母**：码典每个格子有 `node_label`。代码里「战斗格」一律是 `event_id` 4 或 5（`sortie/setup.rs`、`sortie/mod.rs`）。
- **旧进度行**：进度行里存的是条件的副本。`count_at_sortie_start` 已有先例：由码典里的原条件（master）决定，旧行跟着走。

## Decision

1. **格子放在 `Kc3rdQuestConditionMapInfo` 上**：`node: Option<String>`，序列化时缺省省略。
   多海域列表里每个海域可以各有格子（847 的 7-3P），所以格子属于海域条目，不属于整个出击条件。
2. **解析**：`7-3P` 拆成海域 7-3、格子 `P`；第三段 `P2` 仍是血条阶段。出击条目的 `node` 补给没有格子的海域条目，统一转成大写。
3. **事件**：
   - `SortieBattleCompleted` 加 `node: Option<String>`，取当前战斗格的 `node_label`。
   - 新增 `SortieNodeReached { maparea_id, mapinfo_no, gauge_phase, node, fleet_id, fleet_ships }`。在推进到下一格时，只要那一格不是战斗格（`event_id` 不是 4 或 5）就发出，和该格的效果写在同一个事务里。
4. **匹配**：
   - 海域条目带格子时，只认同一格子。
   - 战斗事件照旧按 boss 格和战果判断。
   - 到达事件只给带格子的海域条目计数，并且条件不能要求击败 boss，也不能要求某个评级（`Clear`、`Any` 或不写都算到达即达成）。
   - 编成检查与战斗事件相同。
5. **旧进度行**：行里的海域条目没有格子、而 master 里同一海域（海域号、图号、阶段相同）有格子时，用 master 的格子。旧行里已经被丢掉的 7 海域条目补不回来，不迁移。

## Implementation Units

- **U1 解析（先写测试）**：`7-3P` → 7-3 加格子 P；`7-2-P2` → 血条阶段 2；`1-6` 加条目的 `node: "n"` → 格子 `N`。
- **U2 匹配（先写测试）**：在 `matcher.rs` 的测试里覆盖以下几点：
  - 到达 1-6 N 给 807 那种条目计数，不给不带格子的条目计数；
  - 战斗事件的格子必须对得上；
  - 旧进度行从 master 补上格子。
- **U3 事件**：战斗结算带上格子；推进到不打仗的格子时，发出到达事件并交给 `quest::observe`。
- **U4 集成测试**：用 807 的编成出击 1-6，到达 N 后 807 完成；855 这种两个条目的任务，两个都满足才算完成。
- **U5 码典与文档**：重建码典后对照 `quest.json`：带格子的任务都有格子，927、966、967 不再出现没有海域的出击条目。在 `quest.md` 记下格子的规则。

U1 → U2 → U3 → U4 → U5。
