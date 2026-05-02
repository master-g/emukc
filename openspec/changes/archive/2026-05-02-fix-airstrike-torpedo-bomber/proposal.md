## Why

CV opening airstrike (航空戦/kouku) stage3 的数据结构与本物 KC API 不一致。当前实现使用 per-defender 的 0/1 标记数组，但本物 API 使用 per-attacker 的目标索引数组。客户端依赖 per-attacker 数据渲染鱼雷机动画，导致舰攻攻击偶尔不可见。

## What Changes

- **修改 `BattleKoukuStage3` 结构**: 将 `api_frai_flag`/`api_fbak_flag`/`api_erai_flag`/`api_ebak_flag` 从 per-defender 0/1 标记改为 per-attacker 目标索引（值 = 目标 position，-1 = 未攻击）
- **修改 `AirstrikeOutput`**: 从单数组 `damage[]`/`bak_flag[]`/`rai_flag[]` 改为同时追踪 attacker→target 映射
- **修改 `execute_airstrike_phase`**: 每个 bomber slot 记录攻击者索引和目标索引，而非仅设置 defender 级别的 flag
- **修改 `simulate_kouku`**: 构造 stage3 时填充 per-attacker 数组

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `battle-kouku-stage3`: kouku stage3 输出结构从 per-defender flags 改为 per-attacker target indices，匹配本物 KC API 格式

## Impact

- `crates/emukc_model/src/kc2/` — BattleKoukuStage3 struct 定义
- `crates/emukc_gameplay/src/game/battle/core.rs` — simulate_kouku, execute_airstrike_phase, AirstrikeOutput
- 下游所有读取 stage3 rai/bak 字段的代码（测试、battle diagnostics）
- 不影响 damage 计算逻辑，仅影响输出数据结构

## Non-goals

- 不修改制空战/对空射击的飞机损失逻辑
- 不修改 airstrike damage 计算公式
- 不处理航空战/长距离空袭以外的战斗类型
