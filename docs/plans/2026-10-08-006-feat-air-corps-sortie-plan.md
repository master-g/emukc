---
title: "Air Corps Sortie - Plan"
type: feat
date: 2026-10-08
status: draft
execution: code
---

# Air Corps Sortie - Plan

## Problem

基地航空隊的母港侧已经做完（计划 `2026-09-22-001`）：玩家能配属、下达行动指示、补给、扩张。
但航空隊从不出击：`api_req_map/start_air_base` 没有，战斗包里没有 `api_air_base_attack`，
`emukc_battle` 里对基地航空隊零命中。结果是：

- 行动指示设成「出撃」后进 6-4 / 6-5，客户端会让玩家选攻击目标并发 `start_air_base`，服务器 404。
- 没有任何东西让中隊损失機数或疲劳，所以已实现的 `supply`、`cond_recovery` 在游玩中触发不到，
  也没法用真实客户端验证它们的响应。

## Evidence

2026-10-08 读代码与数据得到，实施时不必再查：

- **落点**：常规图只有 6-4（`airbase_count` 1）与 6-5（2）有航空隊可出击。
- **格子距离已经在发**：`real_map_start_data/map_6-4.json`、`map_6-5.json` 的每个格子带 `api_distance`
  （6-4 为 1–8，6-5 为 1–5），已进目录的 `MapCell.distance`，并由 `api_req_map/projection.rs` 发成
  `api_cell_data[].api_distance`。客户端按它给目标格上色：超出半径红，等于半径黄（`main.decoded.js:117247`）。
- **请求形状**：`api_strike_point_1..3`，每个是逗号分隔的格子号（客户端 `:110152`），只在该航空隊为「出撃」时带。
  响应无内容。
- **战斗包字段**：`docs/apilist.txt:2121` 起的 `api_air_base_attack[]`（`api_base_id`、`api_stage_flag`、
  `api_plane_from`、`api_squadron_plane[]`、`api_stage1`、`api_stage2`、`api_stage3` 只含敌方数组）。
  解码器的 `battle_protocol_fields.json` 已有 `api_air_base_attack`、`api_air_base_injection`、
  `api_air_base_rescue_type` 三项（模块 58435）。
- **数值来源存在**：`KC3Kai/kancolle-replay` 的 `js/kcsim.js` 有 `LBASPhase`（2988 行起）与 `airstrikeLBAS`
  （3135 行起），2026-10-08 取 master 分支核对过函数在；内容还没逐条读。
- **敌方对基地的制空值**：KCNav 的敌编成带 `lbasAirpower`，已在 `.data/temp/kcnav/6-4|6-5/` 里。
- **已有的航空战实现**：`crates/emukc_battle/src/simulation/kouku.rs`（825 行，`simulate_kouku`、
  `calculate_fighter_power` 等），基地航空攻击的三个 stage 与它同构。
- **golden 的代价**：`crates/emukc_battle/tests/golden/` 有 40 份 `{:#?}` dump，加字段就全量失配
  （见 `PROJECT_MEMORY.md` 失败尝试）；`tests/gameplay_tests/battle_golden.rs` 渲染 transcript，加字段不动它。

## Decision

- **没有航空隊参战时，战斗的随机数消耗一位都不变。** 基地航空阶段只在这次出击带了攻击该格的航空隊时才运行并抽随机数。
  这样现有 golden 的差异只会是新增字段那一行，可以逐份确认；任何别的差异都是回归。
- **数值来源按可信度排序**：客户端代码（字段是否存在、動画需要什么）＞ `KC3Kai/kancolle-replay` 的 `js/kcsim.js`
  （已知的第二条数据链，含基地航空的伤害与制空）＞ wikiwiki 基地航空隊页。两条来源都给不出的数，不实现，写进 Known gaps。
- **出击侧状态放在出击会话里**：攻击目标记在 `ActiveSortieState` 上，不落库；损失的機数与疲劳在战斗结算时写回
  `plane_info`，与舰船的结算走同一个事务。
- 不做（各有原因，见 Scope）：噴式強襲、基地防空与空襲、超重爆迎撃、カタリナ救助、联合舰队下的基地航空。

## Scope

范围内：`start_air_base`；6-4 / 6-5 上「出撃」航空隊对目标格的航空攻击；機数损失、出击消耗与疲劳；
用真实客户端走一遍。

范围外：

- `api_air_base_injection`（噴式強襲）：要噴式機，另有独立的動画与消耗规则，等基本攻击稳定后单列。
- 基地防空（「防空」指示）与 `api_destruction_battle`（基地空襲）：是敌方打基地的另一条链，
  需要敌方空襲编成的数据来源，单列计划。
- `api_req_map/air_raid`（超重爆迎撃）与活动图的位置ギミック：只在活动海域。
- 联合舰队 + 基地航空：6-4 / 6-5 不能用联合舰队出击。

## Implementation Units

### U1 出击目标：`start_air_base`

接上端点。校验：出击会话存在且在出发点；每个带目标的航空隊在该海域、行动指示是「出撃」；
带目标的航空隊数不超过地图的 `airbase_count`；每队恰好两个目标格（可重复），格子属于这张图且
`distance` 不超过该队的半径（`api_base + api_bonus`）。通过后把目标记到 `ActiveSortieState`。

先读客户端目标选择画面（`main.decoded.js:117100` 附近）确认「两个目标」「可否选同一格」「半径为 0 的队能否出击」。

完成标志：端到端测试覆盖接受与四种拒绝；出击会话里能读到各队的目标。

### U2 战斗核心：基地航空攻击阶段

`emukc_battle` 新增基地航空阶段，输入是「攻击本格的航空隊列表（每队的四个中隊：装备、機数、熟練度）」，
在既有航空战之前运行，每队按它指向本格的目标次数攻击一到两次。每次三个 stage：
制空（我方只算该队的中隊，敌方用舰队制空）、对空射击（敌舰击落我方攻击机）、对舰攻击（陸攻 / 艦攻 / 艦爆）。
敌方 HP 的削减进入后续阶段。输出对应 `api_air_base_attack[]` 的结构，并报告每个中隊损失的機数。

公式从 `kcsim.js` 的基地航空部分读，逐条写出处；实施第一步是把要用的公式列成表，
缺来源的条目停下来报告，不自己定值。

完成标志：单元测试覆盖制空五档、击落、伤害上限与陸攻的对舰倍率；不带航空隊的战斗，
`cargo test -p emukc_battle` 的 golden 只有新增字段的差异，逐份确认后重冻结并在 PR 说明。

**公式表（2026-10-08，`KC3Kai/kancolle-replay` master 的 `js/kcsim.js` 与 `js/kcships.js`）**

| 项 | 取值 | 出处 | 实现 |
| --- | --- | --- | --- |
| 基地制空值 | 各中隊 `floor((対空 + 迎撃×1.5[局戦]) × √機数)` 之和 | `kcships.js:2241` `airPower` | 照做；陸偵乘数（1.15 / 1.18）不做 |
| 制空状态 | 我方 ≥ 敌方×3 确保，≥×1.5 优势，敌方同理，否则均衡 | `kcsim.js:2274` `compareAP` | 用现有 `AirState::from_power` |
| stage 1 损失 | 我方按制空状态的比例区间逐中隊扣 | `kcsim.js:2330` `AADefenceFighters` | 同 kouku 的简化：用现有 `stage1_*_loss_ratio` |
| stage 2 击落 | 逐攻击中隊，随机一艘敌舰的割合 / 固定击落 | `kcsim.js:3053`、`getAAShotProp/Flat` | 同 kouku 的简化（敌方対空合计 / 400） |
| 参加攻击的机种 | 艦爆、艦攻、陸攻等能对舰的；纯战斗机不攻击 | `kcsim.js:3053` | 照做 |
| 基准值 | 陸攻：对陆上型用爆装，否则雷装；其余：艦爆用爆装，艦攻用雷装（对陆上型减半） | `kcsim.js:3240` | 照做，陆上型用现有 `is_installation_target_name` |
| 基本攻击力 | `25 + 基准值 × √(1.8 × 機数)` | `kcsim.js:3277` | 照做 |
| 上限前补正 | 陸攻 ×0.8 | `kcsim.js:3278` | 照做 |
| 上限 | 220，超出部分开方 | `kcsim.js:201`、`:3346` | 照做，用现有 `apply_cap` |
| 上限后补正 | 陸上攻撃機(47) ×1.8 | `kcsim.js:3283`、`:3357` | 照做 |
| 命中与暴击 | 基础命中 0.9、熟練度与装备命中、回避 ×0.86 | `kcsim.js:3138`–`3236` | **不做**：同 kouku 的简化，必中、无暴击 |
| 目标选择 | `choiceWProtect`（旗艦援護） | `kcsim.js:3098` | 同 kouku 的简化：存活敌舰里均匀随机 |
| 触接 | `getContact`，倍率进上限后 | `kcsim.js:3028` | **不做**，`api_touch_plane` 发 `[-1, -1]` |
| 装备专属补正 | 444 / 453 / 454 / 459 / 484 / 562 等的命中与威力 | `kcsim.js:3180`–`3270` | **不做** |
| 特定深海舰的特效 | 按舰 ID 的 ×1.2–×3.5 | `kcsim.js:3300`–`3325` | **不做**（多为活动敌舰） |
| 对潜 | 対潜值 ≥ 7 的机种打潜艇 | `kcsim.js:3051`、`:3279` | **不做**，基地航空不打潜艇 |

**保真度的取舍**：现有舰载机航空战（`kouku.rs`）本身就是简化模型——没有命中判定，stage 2 是
`対空合计 / 400` 的线性近似，目标均匀随机。基地航空与它同档：结构和核心威力式按 `kcsim.js`，
命中、触接、装备与敌舰专属补正不做。把基地航空单独做到 `kcsim.js` 的档次，要连带引入回避、阵形、
熟練度一整串，而舰载机一侧仍是简化的，两边不一致。要提高保真度应两边一起做，另立计划。

**与来源不同的一处**：`kcsim.js` 每一波都从战斗前的機数重新开始（`_currentSlots`，`:3704`）。
这里让损失延续到下一波，并在战后写回——否则「写回哪一波的损失」没有定义。

**包形状**：客户端的 `AirUnitData` 继承 `AirWarDataBase`（`main.decoded.js:124366`–`124415`），与普通航空战同一个基类，
多读 `api_base_id` 与 `api_squadron_plane[]`（`api_mst_id`、`api_count`）；制空显示取 `api_stage1.api_disp_seiku`。

### U3 接进出击：到达目标格时带上航空隊，结算时写回

`next_sortie` 到达战斗格时，从会话取出攻击该格的航空隊交给战斗；战斗包带 `api_air_base_attack`；
`battleresult` 结算时把損失機数写回 `plane_info.count`。出击开始时扣出击消耗（各机种的燃料与弾薬，
来源 wikiwiki 基地航空隊页「出撃コスト」，实施时核对并写进计划），并按来源的规则加疲劳。

完成标志：6-4 的集成测试——配属陸攻的航空隊指向 boss 格出击，boss 战的包里有 `api_air_base_attack`，
敌方起始 HP 已被削减，战后 `plane_info.count` 减少，随后 `supply` 能补回并按每機 3 / 5 扣资源。

**实施记录（2026-10-08）**

- 攻击本格的航空隊在战斗准备时取出（每指向一次算一波），战斗包带 `api_air_base_attack`，
  战斗结算时把每队最后一波剩下的機数写回 `plane_info.count`，与舰船结算同一事务。
- **出击消耗已做**，出处 wikiwiki 基地航空隊「出撃コスト」（2026-10-08 读）：陸攻每機 燃料 1.5（切上）、
  弾薬 0.7（切捨）；大型陸上機 2 / 2；其余 燃料 1、弾薬 0.6（切上）。18 機陸攻 = 27 / 12，与同页算例一致。
  在 `start_air_base` 时扣；资源不够时有多少扣多少仍然出击，同页写明。一次出击只能派一次。
- **疲劳没做，单列后续。** 同一页给的规则是：内部コンディション 0–46（30 以上无标记、20–29 橙、19 以下红），
  配备时 40，每次出击帰投时集中 −6、分散 −8，每 3 分钟按行动指示回复（出撃 +1、防空 +2、退避 +3、待機 +4、
  休息 +8，到 40 后每次 +1 至 46）。但该页自己标注这些是「推測される値」；实现它要把 `plane_info.condition`
  从现在存的 `api_cond`（1 / 2 / 3）改成内部值、加一个时间戳列做按时间回复（又是一次加列），并让
  `airCorpsCondRecoveryWithTimer` 与 `cond_recovery` 跟着改。量与本计划其余部分相当，且数值未经验证。
- 顺带读到：配置転換的等待是 12 分钟，之后要回一次母港才解除（同页「よくある質問」）。
  现有实现是下次读取航空隊时立即解除（`settle_relocations_impl` 的注释说没有出处），现在有了，
  可以与疲劳的时间戳一起做。
- 目标格不必是战斗格：客户端对任何有距离的格子都给选（`main.decoded.js:117161`），指向非战斗格的航空隊
  付了出击消耗但不攻击。

### U4 用真实客户端验证

用客户端派生的战斗规则校验 U3 产出的包（`battle validate`）；扩展无头场景 `air_corps_6_4`：
配属 → 指示出撃 → 出击 6-4 → 选目标 → 打到目标格，要求无页面异常、无失败请求、战斗包带基地航空阶段，
回港后补给。这一步顺带实测 `supply` 响应里的 `api_distance`。

完成标志：`make headless-check SCENARIO=air_corps_6_4` 无人值守跑通；`battle validate` 无错误。

### U5 质量门与沉淀

三道质量门；`apilist.md`、`TODO.md`、`docs/api_coverage.md` 按路由重推；
`docs/solutions/architecture-patterns/` 记基地航空阶段的输入输出与数值出处；`PROJECT_MEMORY.md` 回写。

## Open Questions

- 出击消耗与疲劳的确切规则（哪些机种多少燃料弾薬、疲劳何时加、何时自然恢复）。恢复若按时间，
  `airCorpsCondRecoveryWithTimer` 就有了内容，是否纳入本计划由 U3 读完来源后定。
- 敌方对空射击对基地航空机的击落规则与舰载机是否相同（`kcsim.js` 里如何处理）。
- 基地航空的触接、陸攻的熟練度与クリティカル补正是否有可用来源。
- 6-4 路线上的陆上型敌舰对各机种的特效（陸攻对地无效等）在来源里是否完整。
- 目标格是否必须是战斗格；客户端会不会让玩家选非战斗格。

## Stop Conditions

- U2 的公式表里，对舰伤害或制空任何一项两条来源都给不出：停在 U2，不编数。
- 不带航空隊的战斗出现「新增字段」以外的 golden 差异：停下查随机数消耗顺序，不重冻结。
- 客户端播放基地航空阶段需要的字段超出 apilist 所列且含义读不出来：停在 U4 报告。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`（基线 17）、
  `cargo test --workspace --exclude emukc_time --no-fail-fast`。
- `cargo test -p emukc_battle` 的 golden 重冻结差异逐份说明。
- `cargo run -- battle validate` 对带基地航空的包；`make headless-check SCENARIO=air_corps_6_4`。
