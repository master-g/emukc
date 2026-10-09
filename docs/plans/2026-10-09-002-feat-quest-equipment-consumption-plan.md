---
title: "Quest Equipment Consumption - Plan"
type: feat
date: 2026-10-09
status: draft
execution: code
---

# Quest Equipment Consumption - Plan

## Problem

任务领奖时该交出的装备一件都不扣。`game/quest/consume.rs` 里 `handle_slotitem_consumption` 是空函数，
`Kc3rdQuestConditionConsumption::SlotItemConsumption` 分支是空的；同时
`thirdparty/quest/progress.rs:61` 把这两类条件一律算作已满足。结果是：机种转换任务不用持有原装备就能领到
新装备，原装备也留在手里；要求「准备」装备的任务白拿奖励。

## Evidence

codex（`.data/codex/quest.json`，2026-10-09 统计）：

| 形状 | 数量 | 细节 |
|---|---|---|
| `ModelConversion` 条件的任务 | 63 | 70 个槽位条件，全部是单个装备 id、数量 1；`EquipType` 一次都没出现 |
| 其中指定秘书舰 | 54 | 另有 1 个带禁用秘书舰 |
| 槽位位置 `pos` | 0：21，1：38，2：7，3：3，4：1 | 0 的含义要在 U1 里从解析器确认（推测是不限槽位） |
| 要求改修值 | ★0：40，★10：24，其余 6 | |
| 要求熟练度 max | 13 | |
| `keep_stars` | 12 | 改修值带到奖励装备上 |
| 奖励里的装备数 | 1 件：55，2 件：2，0 件：6 | 4 个任务没有槽位条件 |
| `SlotItemConsumption` 条件的任务 | 42 | 66 个装备条件，全部是单个装备 id；★0：62，★10：3，★4：1；都不要求熟练度 |
| 两类都有的任务 | 11 | |

规则出处是 wikiwiki 任務页（2026-10-09 读）：

- 「準備」的装备在达成后消费，**低改修值的优先**（F48「任務達成後…必要装備(低改修値のもの優先)は消費します」）。
- 机种转换要求**第一舰队旗舰**在指定槽位搭载原装备，且原装备**没有上锁**（F13 等各条「装備ロック解除済」）；
  部分任务还要求熟练度 max、改修 max。
- 原装备上锁时任务停在 80%，不必重新做其余条件（F61「装備のロックで達成できなくても達成率80%となり再廃棄の
  必要はない」）。
- 改修值是否带到新装备按任务而定（F62「TBFの改修(★の数)は継承されない」；Fm2 等带）。F40 带的是熟练度。
- 「廃棄」类条件是另一种条件（`Scrap`），已经按计数实现，不在本计划内。

客户端（`main-decoder/out/main.decoded.js`）：任务模型的 `isValid()`（:102713）读 `api_invalid_flag`，
为 1 时 `alert`（:102738）按任务 id 给出 2–5 号提示，玩家点领奖时看到的是提示而不是奖励。我们现在恒发 0
（`questlist.rs:81`）。

仓库里已有同类做法：编成条件不靠计数，在读任务列表时由 `validate_composition_quests`
（`game/quest/update.rs:343`）现场判定。

## Decision

- **持有条件在读任务列表时判定，与编成条件同一处。** 计数条件都完成、但装备条件不满足的任务进度记 80%、
  不进入达成状态；满足后进入达成。这样不持有装备的任务根本点不了领奖。
- **`api_invalid_flag`**：机种转换的原装备在位但上了锁时发 1，其余发 0。
- **领奖时在同一事务里再判一次并扣除**；不满足时返回错误、不发奖励（防止列表读取与领奖之间装备被动过）。
- **「準備」装备的挑选**：只取没上锁、没装在舰上、没在基地航空隊也没在配置転換中的；改修值达到要求的里面
  按改修值从低到高、再按熟练度从低到高、再按实例 id 取。上锁的不计入持有数。
- **机种转换**：取第一舰队旗舰指定槽位上的那一件（`pos` 为 0 时取该舰上第一件符合的），从舰上卸下并删除；
  `keep_stars` 为真且奖励恰好是一件装备时，把改修值写到奖励装备上。熟练度不继承（F40 是唯一写明继承熟练度的，
  单独列在范围外）。
- 扣除走现有的 `destroy_items_impl`，但不返还废弃资源。

需要你定的两处（下面先按推荐写，改了不影响其余单元）：

1. **上锁的「準備」装备算不算持有。** 推荐不算（上面的写法）：出处只对机种转换写明了要解锁，但锁的含义就是
   不被消耗。另一种是算持有并直接消耗。
2. **`pos` 为 0 的 21 个槽位条件**若确认是数据缺位而不是「不限槽位」，是按不限槽位处理（推荐），还是先修解析器。

## Scope

范围内：上面的全部，加 `TODO.md` 两条（装备消耗、`api_invalid_flag`）勾掉。

不在范围内：

- `api_voice_id`、`api_c_list`（`questlist.rs` 的另外两条欠账，与消耗无关）。
- F40 的熟练度继承；`EquipType` 分支（数据里没有，保留现有的告警）。
- `Scrap` 条件（已实现）；资源与道具消耗（已实现）。
- 6 个带 `ModelConversion` 但奖励里没有装备的任务的特殊处理：照常扣除原装备，奖励按现有流程发。

## Implementation Units

### U1 读清数据与现有判定

- 从 `emukc_bootstrap` 的任务解析器确认 `pos` 的取值含义（0 与 1–4），以及 4 个没有槽位条件的任务是什么。
- 读 `validate_composition_quests` 的调用点与它如何写回进度，确定持有判定接在哪里、80% 怎么表示。
- 结论与计划不符时先停下改计划。

### U2 持有判定

- `game/quest/` 下新增判定函数：给定任务与玩家当前的舰队、装备，返回满足 / 不满足 / 因上锁不满足。
- `progress.rs:61` 不再恒真；读任务列表时对含这两类条件的已接任务调用判定并写回状态。
- `questlist.rs` 的 `api_invalid_flag` 取判定结果。

### U3 领奖扣除

- `consume.rs`：实现两条分支，按 Decision 的顺序挑选并删除；机种转换先卸下再删。
- `claim_rewards`：`keep_stars` 时把改修值写到新装备上。
- `quest_clear_and_claim_reward` 在扣除前再判一次，不满足返回错误。

### U4 验证与沉淀

- 集成测试（`tests/gameplay_tests/`）：不持有时任务不达成；持有后达成并在领奖后少了对应装备；低改修值优先；
  上锁的不被消耗；机种转换卸下旗舰槽位并带上改修值；原装备上锁时 `api_invalid_flag` 为 1 且领奖被拒；
  列表读取后装备被废弃再领奖被拒。
- 无头场景不新增：现有三个场景都不经过工厂任务，页面提示由 Rust 测试断言的字段值加客户端读取处核对。
- 更新 `docs/solutions/architecture-patterns/quest.md`、`TODO.md`、`PROJECT_MEMORY.md`。

## Stop Conditions

- `pos` 的含义从解析器读不出来时停下问。
- 判定接入后有现存任务测试因为「恒真」被改掉而大面积失败（超过 10 个）时停下，先核对是测试夹具缺装备
  还是判定写错。

## Verification

`cargo fmt --all --check`；`cargo clippy --workspace --all-targets -- -W warnings`（基线 17）；
`cargo test --workspace --exclude emukc_time --no-fail-fast`。战斗 golden 不应变化。
