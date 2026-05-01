## Context

KanColle 航空战 kouku stage3 的本物 API 返回 per-attacker 目标索引数组：

```
api_frai: [2, -1, 1, -1, -1, -1]  // friendly ship i → target enemy index (-1 = no attack)
api_erai: [0,  0, 0,  0,  0,  0]  // enemy ship i → target friendly index
```

当前 emukc 实现使用 per-defender 0/1 flag 数组：

```rust
// crates/emukc_gameplay/src/game/battle/core.rs:1380
let mut api_frai_flag = vec![0i64; friendly.len()]; // 索引 = defender position, 值 = 0/1
```

客户端依赖 `api_frai[ship_idx]` 的语义（"ship_idx 的鱼雷机攻击了哪个目标"）来渲染动画。当收到 per-defender flag 格式时，客户端无法确定哪个 ship 的鱼雷机参与了攻击，导致动画缺失。

### 当前代码流程

```
simulate_kouku (line 1333)
  └── execute_airstrike_phase (line 1233)
       ├── Phase 1: 舰爆 — 遍历 attacker slots, 随机选 target
       │    └── output.bak_flag[target_idx] = 1  ← per-defender
       └── Phase 2: 舰攻 — 遍历 torpedo bomber slots, 随机选 target
            └── output.rai_flag[target_idx] = 1  ← per-defender
  └── 构建 BattleKoukuStage3
       └── api_frai_flag = [0,1,0,0,0,0]  ← per-defender flags
```

### 本物 API 期望格式

```
api_frai: [3, -1, 2, -1, -1, -1]
           ↑ship0 的舰攻攻击了 enemy3
                    ↑ship2 的舰攻攻击了 enemy2
```

## Goals / Non-Goals

**Goals:**
- BattleKoukuStage3 的 rai/bak 字段改为 per-attacker 目标索引格式
- 每个 bomber slot 记录攻击者 ship index → 目标 defender index 映射
- 同一 ship 有多个 bomber slot 时，取最后一个（与本物行为一致）

**Non-Goals:**
- 不修改 damage 计算逻辑
- 不修改 Stage 1/2 飞机损失逻辑
- 不修改 `api_fdam`/`api_edam`（这些本就是 per-defender，保持不变）
- 不处理 `api_f_sp_list`/`api_e_sp_list`（特殊攻击标记，与本次无关）

## Decisions

### Decision 1: AirstrikeOutput 结构改为 per-attacker 映射

**选择**: 将 `AirstrikeOutput` 从三个 per-defender 数组改为同时追踪 attacker→target 映射

**方案**:
```rust
struct AirstrikeOutput<'a> {
    damage: &'a mut [i64],        // 保持不变，per-defender 累积伤害
    bak_targets: &'a mut [i64],   // 改为 per-attacker, 值 = target index (-1 = 未攻击)
    rai_targets: &'a mut [i64],   // 改为 per-attacker, 值 = target index (-1 = 未攻击)
}
```

**替代方案**: 使用 `HashMap<usize, usize>` 追踪映射 → 拒绝，因为需要有序数组直接输出到 API

**理由**: 直接用 `Vec<i64>` 长度 = attacker count，初始化为 -1，命中时写入 target index。与最终 API 格式完全一致。

### Decision 2: execute_airstrike_phase 签名变更

需要传入 attacker 的起始索引（用于多 ship 场景）：

```rust
fn execute_airstrike_phase(
    codex: &Codex,
    random: &BattleRandom,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    is_enemy_side: bool,
    output: &mut AirstripeOutput,  // 改为 per-attacker targets
)
```

每个 bomber slot 处理时，`(ship_idx, slot_idx)` 唯一标识攻击来源。对于 `bak_targets[ship_idx]` 和 `rai_targets[ship_idx]`，同 ship 多个 slot 的情况取最后一个覆盖。

### Decision 3: BattleKoukuStage3 字段重命名

```rust
// 旧
pub api_frai_flag: Vec<i64>,  // per-defender 0/1
// 新
pub api_frai: Vec<i64>,       // per-attacker target index (-1 = no attack)
```

## Risks / Trade-offs

- [Risk] 同 ship 多 bomber slot 覆盖问题 → Mitigation: 本物 KC 也是取最后攻击的目标，行为一致
- [Risk] 下游代码依赖旧字段名 → Mitigation: 全局搜索 `api_frai_flag`/`api_fbak_flag`，确保全部更新
- [Risk] enemy 侧同理需要修改 → Mitigation: `api_erai`/`api_ebak` 同步改为 per-attacker
