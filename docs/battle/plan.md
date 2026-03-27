# EmuKC Battle System Plan

> 战斗系统按可落地的增量阶段推进，而不是一次性实现完整 KC 全战斗域。

## Current Baseline

仓库当前真实状态：

- 已有一套**演习昼战**模拟，入口在 `practice_battle`。
- `battle` 模块已抽出通用昼战核心，可被演习和未来 sortie 复用。
- 已新增最小 `sortie` battle session 脚手架，用于承载正式出击接线。
- 尚未实现夜战、对潜、联合舰队、基地航空队、PT 小鬼群和完整地图战斗流。

研究文档 `research.md` 仍作为公式与机制来源，但只在进入当前阶段的范围内落地。

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

## Phase 2

目标：补全单舰队战斗中的主要昼夜分支。

- 夜战入口与 `api_midnight_flag`
- 夜战普通攻击与 CI 基础实现
- 对潜攻击与 OASW 触发
- 航空战阶段从“占位实现”升级为制空与阶段伤害模型
- 更完整的胜败判定与战果结算

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
