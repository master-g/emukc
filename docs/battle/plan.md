# EmuKC Battle System Plan

> 战斗系统按可落地的增量阶段推进，而不是一次性实现完整 KC 全战斗域。

## Immediate Repair Track

当前最紧急的 battle 工作不是继续扩机制覆盖面，而是先修复会让客户端崩溃的非法 payload：

- 错误的 `api_si_list` / 攻击类型组合
- manifest 中不存在的敌舰装备 ID
- fallback 敌舰的 `api_onslot` / `api_eSlot` 不一致

详见：[`data-fidelity-fix.md`](./data-fidelity-fix.md)

## Current Baseline

仓库当前真实状态：

- 已有一套**演习昼战**模拟，入口在 `practice_battle`。
- `battle` 模块已抽出通用昼战核心，可被演习和未来 sortie 复用。
- 已新增最小 `sortie` battle session 脚手架，用于承载正式出击接线。
- 尚未实现夜战、对潜、联合舰队、基地航空队、PT 小鬼群和完整地图战斗流。
- sortie 敌编成来源目前只有“节点敌舰 `ship_ids`”，敌舰属性源并不完整。
- 早期 abyssal ID 如 `1501`、`1502`、`1503`、`1505` 在当前 `start2` manifest 中缺少 `api_taik` / `api_houg` / `api_souk` / `api_maxeq`，同时也没有对应 `ship_extra`，因此 battle fallback 会把它们构造成 `HP=1` 的敌舰。

研究文档 `research.md` 仍作为公式与机制来源，但只在进入当前阶段的范围内落地。

## Current Risks

- 现有 sortie battle 还不能视为“可玩”。地图推进和战斗接线已经贯通，但敌舰 stat source 缺失会让实际战斗结果严重失真。
- 当前问题不是单一公式误差，而是数据源缺口。只要敌舰属性仍来自 `[1, 1]` fallback，命中、伤害、胜败判定、经验都不可信。
- `build_sortie_enemy_ship()` 当前优先调用 `codex.new_ship()`，查不到时退回 manifest，并使用 `mst.api_taik.unwrap_or([1, 1])`。这条退路对 abyssal 早期 ID 会稳定地产生 `HP=1`。

## Phase 1

目标：建立**正式出击可复用**的单舰队昼战骨架。

已完成：

- `battle/core.rs`
  - 通用 `BattleContext`
  - `BattlePacket` / `BattleOutcome`
  - 单舰队昼战 `simulate_day_battle_v1`
  - 昼战炮击 cap 220 / 雷击 cap 180
  - 阵形与交战形态修正的 v1 实现
- `battle/practice.rs`
  - 演习改为调用通用 battle core
  - 保持原有演习 API 包结构
- `battle/sortie.rs`
  - 最小 sortie battle session 存储与读取脚手架

剩余：

- 将 sortie battle session 接到实际 `map` / `api_req_sortie` 流程
- 细化昼战命中、暴击、目标选择与更准确的 armor/random 行为
- 为正式出击设计 battle/result 两步式 API 映射
- 引入敌舰有效 stat source，替换当前对 abyssal 的 `HP=1` fallback

## Phase 2

目标：补全单舰队战斗中的主要昼夜分支。

- 夜战入口与 `api_midnight_flag`
- 夜战普通攻击与 CI 基础实现
- 对潜攻击与 OASW 触发
- 航空战阶段从“占位实现”升级为制空与阶段伤害模型
- 更完整的胜败判定与战果结算
- 将敌舰装备、火力、装甲、耐久与等级统一挂到可复用 enemy master 数据源，而不是临时从 manifest 拼接

## Phase 3

目标：覆盖高复杂度机制与特殊敌我交互。

- 联合舰队
- 基地航空队
- PT 小鬼群
- 支援舰队
- 特殊装备协同与舰种特攻

## Guardrails

- 演习 API 兼容性优先，battle core 重构不能破坏现有 `practice` 客户端流程。
- 新机制优先落在通用 `battle` 层，再由 `practice` / `sortie` 各自映射到响应。
- 只有进入当前 phase 的机制才进入 DoD，其余内容保留在研究文档中，避免范围蔓延。
- 在敌舰 stat source 完成之前，不应把当前 sortie battle 结果当成“接近真实 KC 战斗”的基线。
