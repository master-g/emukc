# Battle Data Fidelity Fix Plan

> 这份文档记录当前 battle 系统的**数据保真修复**工作，目标不是继续扩大战斗机制覆盖面，而是先消除会让客户端崩溃的非法响应数据。

## Problem

当前 battle 系统已经能产出昼战、夜战、对潜和部分 sortie battle 响应，但仍会生成一类客户端无法安全消费的数据：

- `api_si_list` 中的装备 ID 与攻击类型不匹配
- `api_eSlot` / `api_si_list` 中包含 `api_mst_slotitem` 中不存在的装备 ID
- manifest-only fallback 产出的敌舰出现 `api_onslot` 非零但 `slot_items` 为空
- 敌舰 bootstrap 数据没有和 manifest 做严格交叉验证

这类问题不是“数值偏差”，而是**协议级错误**。客户端会按攻击类型和装备 ID 去加载对应资源，一旦 ID 不存在或组合不合法，就可能直接崩溃。

## Root Causes

### 1. `weapon_display_ids()` is attack-agnostic

`crates/emukc_gameplay/src/game/battle/core.rs`

当前 `weapon_display_ids()` 固定返回前 2 个装备 master ID，所有攻击阶段共用。这对客户端来说是不安全的：

- `api_at_type = 7` 的 ASW 攻击，客户端期望看到声纳 / 爆雷 / 对潜机
- 夜战 CI (`api_sp_list = 2~5`) 需要返回和 CI 类型对应的触发装备组合
- 现在的实现会把“前两个槽位”错误地当成“本次攻击所用装备”

结果是：客户端可能按 ASW/CI 动画路径去加载主炮、鱼雷或其他无关装备资源。

### 2. Enemy equipment IDs are not validated against manifest

`crates/emukc_model/src/codex/ship.rs`

`new_enemy_ship()` 直接把 bootstrap 产出的 `slot_info.item_id` 装配进 `KcApiSlotItem`，没有先检查该 ID 是否存在于 `api_mst_slotitem`。这意味着：

- 外部数据源里出现的未来装备 / 活动装备 / 解析错误装备
- 但本地 manifest 里并不存在

依然会一路流入 `api_eSlot` 和 `api_si_list`。

### 3. Manifest-only enemy fallback is internally inconsistent

`crates/emukc_gameplay/src/game/sortie.rs`

`build_manifest_only_sortie_enemy_ship()` 会用 manifest 的 `api_maxeq` 构建 `api_onslot`，但 `slot_items` 始终为空。这样会出现：

- `api_onslot` 声称该敌舰有舰载机
- `api_eSlot` 却全是 `-1`

这属于响应内部自相矛盾，客户端行为不可预期。

## Fix Scope

这轮修复只处理**客户端安全性**和**协议数据正确性**，不在本轮直接扩展：

- 联合舰队
- 基地航空队
- 更完整的航空战动画语义
- 更精细的命中 / 暴击 / 特攻公式

先把 battle payload 收到“客户端可稳定消费”的状态，再继续推进 fidelity。

## Current Status

已完成：

- `api_si_list` 不再统一回退到“前两个槽位”，而是按昼战炮击 / 对潜 / 夜战上下文选择展示装备
- `102 -> slot/btxt_flat` 这类事故已被固定为回归样例，并可通过 battle incident analyzer 解释
- `new_enemy_ship()` 会过滤 manifest 中不存在的敌舰装备 ID
- manifest-only fallback 敌舰现在返回 `api_onslot = [0; 5]`

仍待继续收紧：

- 更多昼战 / 夜战 cutin 的展示装备细则
- 更正式的 battle display rule table
- 敌舰 stat source 的完整化

## Fix Tracks

### Track 1. Context-aware `api_si_list`

目标：让 `api_si_list` 返回“本次攻击真正对应的装备 ID”，而不是固定取前两个槽位。

实现方向：

1. 用新的 helper 替代 `weapon_display_ids()`
2. 按攻击上下文构建装备列表
3. 在 night battle 中把 `NightAttackType` 明确传入 `si_list` 生成逻辑

最低要求：

- Day shelling (`api_at_type = 0`)：返回可用于昼战砲撃展示的武器 ID
- ASW (`api_at_type = 7`)：只返回 ASW 装备 ID
- Night normal / DoubleAttack：返回普通夜战武器 ID
- Night CI：
  - MainMainMain -> 3 main gun IDs
  - MainMainSec -> 2 main guns + 1 secondary gun
  - TorpTorpTorp -> torpedo IDs
  - MainTorpRadar -> main gun + torpedo + radar

关键调用点：

- `simulate_shelling_side()`
- `simulate_oasw()`
- `simulate_night_hougeki()`

### Track 2. Enemy equipment manifest validation

目标：保证进入 runtime 的敌舰装备 ID 全都存在于当前 manifest。

实现方向：

1. 在 `new_enemy_ship()` 中过滤不存在于 manifest 的装备 ID
2. 对被过滤的 ID 打 warning，方便追 bootstrap 数据问题
3. 对 `new_ship()` 保持相同防线，避免玩家船和敌舰走出两套校验标准

Acceptance:

- `api_eSlot` 中不再出现 manifest 不存在的装备 ID
- `api_si_list` 中不再出现 manifest 不存在的装备 ID

### Track 3. Manifest-only fallback consistency

目标：fallback 敌舰至少要做到“响应内部自洽”。

实现方向：

1. 当 `slot_items` 为空时，强制 `api_onslot = [0; 5]`
2. 避免返回“有搭载量但没有任何装备”的敌舰
3. 保持 `api_eSlot` / `api_onslot` / `api_ship_ke` 三者之间语义一致

这不会让 fallback 敌舰变得真实，但能保证它**不再生成会把客户端打崩的无效数据**。

### Track 4. Verification

目标：修复后不只“能编译”，还要证明 battle payload 已经回到安全区间。

验证内容：

1. Gameplay tests
2. Sortie route tests
3. Clippy 无新增 warning
4. 对关键 battle packet 做断言：
   - `api_si_list` 只包含 manifest 已知装备 ID
   - `api_eSlot` 只包含 manifest 已知装备 ID 或 `-1`
   - ASW attack 的 `api_si_list` 不含明显非 ASW 装备
   - Night CI 的 `api_si_list` 与 `api_sp_list` 语义一致

## Recommended Order

1. `battle-si-list-fix`
2. `enemy-equip-validation`
3. `manifest-fallback-consistency`
4. end-to-end verification

前三项彼此独立，但建议先做 Track 1，因为它直接修掉当前最危险的“攻击类型 / 装备资源错配”。

## Guardrails

- 不要继续用“前两个槽位”近似表示所有攻击类型
- 不要把 manifest 不认识的装备 ID 继续透传给客户端
- 不要让 fallback 敌舰返回内部不一致的数据结构
- 修复目标是“客户端稳定 + 协议自洽”，不是在这一轮追求公式完全真实

## Validation Commands

```bash
cargo check --workspace
cargo test -p emukc_gameplay --lib
cargo test --bin emukcd -- "api_req_sortie::tests" "api_req_battle_midnight::tests"
cargo clippy --workspace
```
