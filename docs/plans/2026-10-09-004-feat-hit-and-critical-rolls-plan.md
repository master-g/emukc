---
title: "Hit and Critical Rolls for Every Battle Phase - Plan"
type: feat
date: 2026-10-09
status: draft
execution: code
---

# Hit and Critical Rolls for Every Battle Phase - Plan

## Problem

战斗引擎里没有命中判定，也没有暴击判定。炮击、雷击、夜战、对潜、航空战、基地航空队和基地空袭全部必中，
`api_cl_list` 一律写 1。结果是回避、运、命中装备、阵形的命中补正、疲劳对战斗都没有作用，伤害只取决于火力和装甲。

## Evidence

**我方现状（2026-10-09 读代码）：**

| 阶段 | 位置 | 现在怎么写命中 |
|---|---|---|
| 昼战炮击、对潜 | `simulation/day_attack.rs:108` | `api_cl_list.push(vec![1; n])` |
| 夜战 | `simulation/night.rs:839`、`:917` | `hit_cls.push(1)` |
| 开幕雷击 | `types/packet.rs:272`、`:279` | `api_fcl_list_items[i] = Some(vec![1])` |
| 闭幕雷击 | `types/packet.rs:344`、`:350` | `api_fcl[i] = 1` |
| 舰队航空战 | `simulation/kouku.rs` | 每个中队必定造成伤害；`cl_flag` 恒 0（PR #28） |
| 基地航空队出击 | `simulation/air_base.rs` | 同上 |
| 基地空袭 | `simulation/air_raid.rs` | 同上 |

整个 crate 没有任何地方读 `api_kaihi` 或 `api_lucky`。`combined.rs` 的头注释写明联合舰队的命中 / 回避补正
因上游全是未验证值而没有建模。

**需要的数据都在。** 我方舰的回避、运、等级、疲劳值在 `KcApiShip` 上；敌舰走 `codex/ship.rs:173`
（`api_kaihi: [basic.evasion, basic.evasion]`），只有 manifest-only 兜底路径（`enemy_ship.rs:203`）回避为 0。
装备命中是 `ApiMstSlotitem.api_houm`。

**客户端怎么读（`main-decoder/out/main.decoded.js`，2026-10-09）：**

- 炮击 / 夜战的 `api_cl_list`、雷击的 `api_fcl` / `api_ecl`：0 落空，1 命中，2 暴击（`transcript.rs:112` 已按此渲染）。
- 航空战 stage3：`AirWarStage3Model.getHitType` 返回 `cl_flag + 1`，所以 `cl_flag` 0 是普通命中、1 是暴击；
  落空没有单独的标记，表现为 `rai_flag` / `bak_flag` 置位而伤害为 0。现在 `kouku.rs` 只在伤害大于 0 时置这两个
  标记，落空会什么都不演——实现时要改成选定目标就置位，并在无头客户端里确认。

**公式来源：`KC3Kai/kancolle-replay` 的 `kcsim.js`**（空袭实现用的同一份，行号对应本地副本）：

- `hitRate`（2139）：`(基础 + 2√等级 + 1.5√运 + 命中修正) × 倍率 ÷ 100`。
- `accuracyAndCrit`（2144）：回避值 `floor((回避 + √(2×运)) × 阵形回避)`；40 以下原值，40–65 为 `40 + 3√(v−40)`，
  65 以上为 `55 + 2√(v−65)`，都向下取整；燃料不足 75% 时再减。命中率 `max(命中 − 回避, 10)`，乘目标疲劳倍率
  （≥50 为 0.7，≥30 为 1，≥20 为 1.2，否则 1.4），上限 96。暴击率 `√命中率 × 暴击系数`。
- `rollHit`（2191）：一次 0–99 的随机数，`≤ 暴击率` 为暴击，`≤ 命中率` 为命中，否则落空。
- `damageCommon`（2242）：传进来的攻击力已经过上限（`shell()` 602 行先 `softCap` 再调用），暴击在这之后、
  装甲计算之前乘 `CRITMOD = 1.5` 并向下取整。顺序是：上限 → `floor(×1.5)` → 装甲。
- 各阶段的基础值和暴击系数：

| 阶段 | 基础 | 命中修正 | 倍率 | 目标阵形回避 | 暴击系数 |
|---|---|---|---|---|---|
| 昼战炮击（444、529、599） | 90 | 装备命中合计 | 疲劳；阵形未被克制时再乘 `shellacc` | `shellev` | 1.3 |
| 夜战（705–727、821、874） | 69，照明弹 +5；夜间触接时 ×1.1 / 1.15 / 1.2 | 装备命中合计 | 阵形 `NBacc` × 疲劳，夜战 CI 另有倍率 | `NBev` | 1.5；夜间触接时 1.57 / 1.64 / 1.7 |
| 雷击（1878、1921） | 85 | 装备命中合计 | 阵形 `torpacc` × 疲劳 | `torpev` | 1.5 |
| 对潜（989、1000） | 80 | 声纳命中 | 疲劳；阵形未被克制时再乘 `ASWacc`（缺省用 `shellacc`）| `ASWev` | 1.3 |
| 舰队航空战（1968、2013） | 固定 95% | 无 | 无 | 1 | 0（只来自熟练度） |
| 基地航空队（3135、3236） | 固定 90% + 0.07 × 装备命中 | 无 | 无 | 1 | 0（只来自熟练度） |
| 基地空袭（4409） | 同舰队航空战 | — | — | — | — |

  基地航空队那一行取的是源码默认开关下的值：`enableLBASFormula2: true` 时基础是 `lbasAccBase = 0.9`，
  `LBASBuff: true` 时才加 `0.07 × 装备命中`；两个开关关掉时是固定 95%。
  同一开关下目标回避值最后再乘一个系数（3233）：`lbasEvaModSingle: .86`，对联合舰队 `lbasEvaModCombined: .68`。
- 阵形克制 `formationCountered`（431）：攻击方複縦対敌単横、梯形対単縦、単横対梯形时返回真，这时炮击和对潜
  不乘攻击方阵形的命中倍率。雷击（1866）和夜战（707）没有这个条件，直接乘 `torpacc` / `NBacc`。
- 攻击方疲劳倍率 `moraleMod`（`kcships.js` 1798）：炮击、夜战、对潜用 ≥50 为 1.2、≥30 为 1、≥20 为 0.8、否则 0.5；
  雷击用 `moraleMod(true)`：≥50 为 1.3、≥30 为 1、≥20 为 0.7、否则 0.35。
- 阵形表在 `kcsim.js` 开头（`LINEAHEAD` … `COMBINEDCF4`），每个阵形给 `shellacc`、`torpacc`、`NBacc`、
  `shellev`、`torpev`、`NBev`、`ASWev`。

**对结果的影响估计：** 航空战和空袭的命中率约 95% 减回避，变化小；这不会减轻「6-5 空袭把基地打到只剩 1」，
那是伤害公式的结果。炮击和雷击变化大：对高回避的驱逐舰会有明显落空，暴击会让伤害上限提高 1.5 倍。

## Decision

1. **一个共用的判定函数，放在 `emukc_battle/src/damage.rs` 旁的新模块 `accuracy.rs`。** 输入是攻击方命中率
   （各阶段自己算）、目标、阵形回避倍率、暴击系数；输出 `Miss | Hit | Critical`。照 `accuracyAndCrit` 与
   `rollHit` 用整数百分比计算，比较用 `<=`，每次攻击只抽一次随机数。
2. **七个阶段全部接上，一次 PR 交付。** 分开做要把战斗基线冻结两次以上，且中间状态下各阶段行为不一致。
3. **暴击在上限之后、装甲计算之前乘 1.5 并向下取整。** 插入点是各 `calculate_*_damage` 里 `apply_cap` 与
   `calculate_defense_power` 之间；加一个倍率参数，不另起一条伤害路径。
4. **落空的伤害是 0，不是擦伤。** 擦伤（`calculate_scratch_damage`）只属于命中但打不穿装甲的情况，保持现状。
5. **第一版不做的补正，各留一条 `ponytail:` 注释写明上限：** 舰载机熟练度的命中与暴击加成、主炮口径适重
   （`ACCfit`）、改修的命中加成、联合舰队命中 / 回避补正（上游未验证）、警戒阵按位置的回避、烟幕、阻塞气球、
   PT 小鬼群、历史加成。没有熟练度时航空战不出暴击，这是照源码的结果。
6. **疲劳和燃料：** 攻击方疲劳倍率与目标疲劳倍率都做（数据在手，公式是一张小表）。目标燃料不足时的回避惩罚
   也做。敌舰的疲劳按 49、燃料按满算。
7. **敌舰没有回避数据时回避按 0。** 只发生在 manifest-only 兜底路径，那里已经有 warn 日志。

## Scope

范围内：上表七个阶段的命中与暴击判定，对应的 `api_cl_list` / `api_fcl` / `api_ecl` / `cl_flag` 取值，
特殊攻击（昼战 CI、夜战 CI、舰队特殊攻击）每一击各判一次。

范围外：支援舰队（还没实现）；第 5 条列出的补正；伤害公式本身；演习与出击共用同一套判定，不单独处理。

不涉及 `crates/emukc_model/src/codex/` 的 `Default`，Balance Defaults Policy 不适用。

## Implementation Units

### U1 判定函数

`accuracy.rs`：`evasion_term`（三段式换算）、`hit_chance`（上下限、疲劳）、`roll`（一次抽取，返回三态）。
阵形命中 / 回避表与现有 `formation_modifier` 放在一起。单元测试：三段式的边界值（40、41、65、66）、
下限 10 与上限 96、固定随机数下三种结果各出一次。

### U2 炮击与对潜

`day_attack.rs`、`shelling.rs`、`asw.rs`、开幕对潜。`api_cl_list` 写真实结果；落空伤害 0。昼战 CI 的每一击各判
一次，CI 的命中倍率按 `getSpecialAttackMod` 取；取不到来源的 CI 倍率按 1 并注释。

### U3 雷击

`torpedo.rs` 与 `types/packet.rs` 里记雷击命中的四处（开幕写 `api_fcl_list_items` / `api_ecl_list_items`，闭幕写 `api_fcl` / `api_ecl`）。

### U4 夜战

`night.rs` 两处 `hit_cls.push(1)`。连击两击各判一次；夜战 CI 的命中倍率同 U2 的处理。

### U5 三种航空战

`kouku.rs`、`air_base.rs`、`air_raid.rs`。选定目标就置 `rai_flag` / `bak_flag`，落空伤害 0。删掉
`air_raid.rs` 与 `air_base.rs` 头注释里「every strike hits」的说明。

### U6 基线与真实客户端

- 重新冻结 `crates/emukc_battle/tests/golden/*.txt` 与 `tests/gameplay_tests/battle_golden.rs`，PR 里解释：
  每次攻击多抽一次随机数，整条随机数序列都会移位，差异是全量的。
- `make battle-sim` 把 `PRESETS` 里每个场景各跑一遍，确认仍能打完。
- 无头客户端跑 `fresh_1_1`（炮击、雷击落空的画面）和 `air_raid_6_5`（航空战落空的画面），确认落空不会卡住客户端。

### U7 沉淀

`docs/solutions/architecture-patterns/` 新增一篇命中与暴击判定的说明（公式、来源、没做的补正）；
更新 `air-corps.md` 的 Known gaps、`TODO.md`、`PROJECT_MEMORY.md`。

## Stop Conditions

- 某个阶段的公式在 `kcsim.js` 里依赖的输入我方运行时状态拿不到，且不在第 5 条的清单里：停下来问，不自己补数值。
- 无头客户端在落空的攻击上卡住或报错：停下来查客户端读法，不靠把落空改成擦伤绕过去。
- 断言「被打中了」的带种子测试因为落空而失败：换种子或改成断言判定结果，不放宽断言。
- 加上判定后 `clearing_1_1_unlocks_1_2` 一类依赖通关的测试通过率明显下降：记录下来并报告，不调公式。

## Verification

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -W warnings`（基线 17 条）
- `cargo test --workspace --exclude emukc_time --no-fail-fast`
- `make battle-sim SCENARIO=<每个预设> SEED=1`
- `make headless-check SCENARIO=fresh_1_1` 与 `SCENARIO=air_raid_6_5`
