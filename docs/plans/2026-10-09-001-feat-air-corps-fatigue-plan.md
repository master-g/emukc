---
title: "Air Corps Fatigue and Relocation Timer - Plan"
type: feat
date: 2026-10-09
status: implemented
execution: code
---

# Air Corps Fatigue and Relocation Timer - Plan

## Problem

基地航空隊已经能配属、出击、损失機数、补给（计划 `2026-09-22-001` 与 `2026-10-08-006`），但中隊从不疲劳：
`plane_info.condition` 永远是 1，`cond_recovery` 白扣一个 航空特別増加食，
`api_port/airCorpsCondRecoveryWithTimer` 永远回空。配置転換也没有等待：摘下的中隊在下一次读取航空隊时
立即解除，而上游要等 12 分钟并回一次母港。两件事缺的是同一样东西：`plane_info` 上的时间戳。

## Evidence

客户端（`main-decoder/out/main.decoded.js`）：

- 中隊的 `api_cond` 只有三档：`AirUnitPanelItemFatigueIcon.update`（:86938）3 画红脸、2 画橙脸、其余不显示。
- `airCorpsCondRecoveryWithTimer`（:83159）：进出击画面时每个航空隊每次会话只问一次（`SallySceneMemory`，
  :87308），且只在有 `state == 1 && fatigue != 0` 的中隊时才问；回应有数据就 `updateSquadronData(api_plane_info,
  api_distance)`，没有数据就什么都不做。
- `cond_recovery`（:83111）：客户端自己把 useitem 102 减 1，再 `updateSquadronData`。
- 配置転換：`api_port/port` 的 `api_plane_info.api_base_convert_slot`（:2828）是一组装备实例 id；
  整个 `api_plane_info` 缺席时客户端把列表清空。`set_plane` 摘下中隊时客户端自己把该装备加进列表（:14692）。
  `isRelocation()`（:14785）据此把装备标成配置転換中、不让再配属。

真实账号快照 `z/snapshot/2026-09-22/api_port_port.json`：`"api_plane_info": {"api_base_convert_slot": [21991]}`。
我们的 `api_port/port` 现在不发这个字段（`port.rs:23` 注释掉了）。

规则出处是 wikiwiki 基地航空隊页（2026-10-09 读）「疲労」「整備Lv強化による効果」「配置転換」：

| 规则 | 数值 | 出处的把握 |
|---|---|---|
| 内部コンディション范围 | 0–46；30–46 无标记，20–29 橙，0–19 红 | 页面自己标「推測される値」 |
| 配备直后 | 40；从配置転換重新配备也回到 40 | 推测值 |
| 出击一次的减少 | 两次指向同一格 −6，分散到两格 −8；与距离、战果、是否打到无关；中途撤退也减 | 推测值；减少时机写的是「帰投時」 |
| 回复节拍 | 每 3 分钟一次，按行动指示加；到 40 为止，之后不论指示每次 +1 到 46 | 推测值 |
| 回复量（整備Lv 0 / 1 / 2 / 3） | 出撃 1/1/1/2，防空 2/2/3/3，退避 3/3/4/4，待機 4/4/5/5，休息 8/10/12/12 | 页面表格，注 43 |
| 配置転換等待（整備Lv 0 / 1 / 2 / 3） | 12 / 10 / 8 / 6 分钟，之后回一次母港才解除 | 页面表格与「よくある質問」 |
| 疲劳对战斗的影响 | 橙色时命中下降，幅度「要検証」 | 无数值 |
| 休息时ボーキ自然回复减半 | 有 | 无精确数值 |
| 航空特別増加食的效果 | 页面没有写恢复到多少 | 无 |

## Decision

- **`plane_info.condition` 改存内部值（0–46），线上的 `api_cond` 在 `PlaneInfo → KcApiPlaneInfo` 一处换算**
  （≥30 → 1，20–29 → 2，其余 → 3）。客户端只看得到三档，内部值不出服务器。
- **加一列 `since`（可空时间戳），两种状态共用**：配属中的中隊记「上一次回复节拍的时刻」，配置転換中的中隊记
  「开始转换的时刻」。两者不会同时需要：转换中的中隊不回复，重新配备时状态与内部值一起重置。
- **迁移沿用 `preset_deck.locked` 的做法**：建表后执行 `ALTER TABLE plane_info ADD COLUMN since ...`，失败忽略。
  这条语句只会成功一次（旧库第一次启动），成功时顺带 `UPDATE plane_info SET condition = 40`，把旧行里的
  `api_cond`（全是 1）换成内部值。新库建表时已有该列，语句失败，不动数据。这是第二次加列；`preset/mod.rs`
  的注释说第二次就该写迁移步骤——这里把「加列成功才回填」做成一个小函数给两处用，仍然不引入迁移表。
- **回复按读取时结算**：`ticks = floor((now − since) / 3 分钟)`，逐拍应用上表，`since` 前移 `ticks × 3 分钟`。
  凡是会读到或改变回复速度的操作（读航空隊、改行动指示、升整備Lv、增加食、计时恢复、派航空隊出击）都先结算。
  改行动指示时先按旧指示结算，再记新指示。
- **疲劳在 `start_air_base` 时扣**，不等帰投。出处写的是帰投时，但它同时写明中途撤退、没打到也照扣，
  所以玩家看到的结果一样；而出击结束的路径有好几条（撤退、打完、断线后回母港），在出发时扣只有一处。
  一个航空隊只指向一格也按集中（−6）算。
- **配置転換只在 `api_port/port` 结算**：超过等待时间的行在母港读取时删除，没超过的装备 id 放进
  `api_plane_info.api_base_convert_slot`。读航空隊和读装备清单时不再结算（现有的两处调用移走）。
  等待时间按该海域的整備Lv 取 12 / 10 / 8 / 6 分钟。
- **航空特別増加食**：把航空隊里低于 40 的中隊恢复到 40。数值是本项目自己定的（出处没有），延续现有
  「恢复到通常」的含义。

## Scope

范围内：上面的全部，加 `airCorpsCondRecoveryWithTimer` 在有中隊档位变化时回数据。

不在范围内：

- 疲劳对战斗的影响。出处没有数值，而且我们的基地航空攻击不做命中判定（计划 006 的「Level of detail」）。
- 休息时ボーキ自然回复减半。出处没有精确数值。
- 配置転換中的装备被服务器拒绝再配属。客户端已经拦住；服务器现在允许（`airbase/mod.rs:186`），不动。
- 基地防空、空襲、噴式強襲（同计划 006）。

## Implementation Units

### U1 数据：`since` 列、内部值、线上换算

- `plane_info` 加 `since: Option<DateTime>`；建表后的 `ALTER TABLE` 与回填；回归测试仿
  `an_older_preset_deck_table_gains_the_lock_column`，并断言旧行的 `condition` 变成 40。
- `emukc_model::profile::airbase`：常量（40、46、30、20）、`cond_tier(internal) -> i64`、
  `recovery_per_tick(action, level)`、`relocation_minutes(level)`；`KcApiPlaneInfo.api_cond` 用 `cond_tier`。
- 配备写 40；增加食改成「低于 40 的恢复到 40」；`COND_NORMAL` 删除。

### U2 回复与疲劳

- `settle_conditions_impl(c, profile_id, now)`：对配属中的中隊按节拍结算。在 Decision 列出的操作开头调用。
- `start_air_base`：扣出击消耗的同一事务里给每个出击的航空隊的中隊扣 6 或 8，下限 0。
- `check_airbase` 改为返回结算后有档位变化时的中隊与半径，否则无数据；处理器照此回应。

### U3 配置転換计时

- `relocate_squadron` 记 `since = now`。
- `settle_relocations_impl` 加时间条件，改由 `port_view` 调用；`get_airbases` 与 `slot_item.rs` 的调用移走。
- `api_port/port` 发 `api_plane_info: { api_base_convert_slot: [...] }`。

### U4 验证与沉淀

- 单元测试：节拍结算（含 40 封顶后 +1、改指示先按旧速度结算）、出击扣减、三档换算、转换未到时间不解除、
  到时间在母港解除且 `api_base_convert_slot` 相应变化。
- 无头场景 `air_corps_6_4` 重跑；它出击一次只从 40 掉到 34，看不到图标。给场景预设加一个可选的
  「中隊初始コンディション」，让场景以橙色起步，确认真实客户端画出疲劳图标且不报错。
- 更新 `docs/solutions/architecture-patterns/air-corps.md`（Known gaps 去掉两条、写明哪些数值是推测值）、
  `docs/api_coverage.md`、`PROJECT_MEMORY.md`。

## 实施记录（2026-10-09）

按 Decision 实施，下面是计划里没有写、实施时定下或改掉的地方。

- **同一航空隊内换槽位改成原样互换。** 旧实现把移动的中隊删掉重建：機数回满、目标槽位原来的中隊消失、
  不扣ボーキ，等于一次免费补给。放着不动的话，换一次槽位疲劳就回到 40，所以必须一起改。现在两行互换
  `squadron_id`，機数与コンディション都不变（出处：同页「中隊の配置入れ替えによる影響もない」）。
- **覆盖配属时，被盖掉的中隊进入配置転換。** 旧实现直接删行，被盖掉的装备立刻可用，12 分钟的等待可以用
  「先盖再摘」绕过。出处同页「入れ替えで外されたほうがリスト上で配置転換中になる」，客户端也是这样记的
  （`main.decoded.js:14692`）。
- 由上一条，**一个槽位可以同时有两行**（转换中的旧中隊与配属中的新中隊）。按 `squadron_id` 取中隊的地方
  都改成取配属中的那一行；战斗结算写回機数也加了状态条件。
- **计时恢复总是回当前的中隊**，不判断「这次有没有变化」。客户端手里的档位可能是几次出击之前的，
  服务器无从知道；`updateSquadronData` 重复应用无害。
- 没有 `since` 的旧行：配属中的从现在起算；转换中的在下一次进母港时直接解除（与旧行为一致）。
- 配属中但機数为 0 的中隊同样扣疲劳（出处：与战果无关）。
- `api_port/port` 的 `api_plane_info` 只在有装备转换中时才发，形状是 `{ "api_base_convert_slot": [...] }`。
- 无头场景用新步骤 `tire:22` 直接改工作区数据库，没有给场景预设加字段：中隊是客户端在场景里自己配属的，
  预设里写不到它。
- 顺带修了上一轮留下的不稳定测试 `an_air_corps_attacks_its_cell_and_comes_home_short`：随机种子在出击开始
  之后才固定，而途中遇到哪支敌舰队在那之前已经掷过，所以它在未改动的代码上也时过时不过。现在在出击前固定。

验证结果：fmt 干净；clippy 17（基线）；全量测试 1250 过、0 败、0 忽略；战斗 golden 无变化；
`air_corps_6_4` 无头通过，橙色疲劳图标在真实客户端里画出来了（`home.png`），计时恢复与补给的响应通过检查。

没有用真实客户端验证的：母港带 `api_base_convert_slot` 的情形（无头脚本做不了拖拽摘除中隊）。字段形状
与真实快照一致，客户端读取处已核对，Rust 测试覆盖了服务器一侧。

## 审查后的修正（2026-10-09）

- **母港在解除配置転換时回 `api_unset_slot`。** 客户端在配属时把装备从自己的未装备列表里拿掉
  （`main.decoded.js:14684`），之后只有母港的 `api_plane_info.api_unset_slot`（:2830）或重新拉
  `api_get_member/unsetslot` 能把它放回去。解除改在母港之后，不回这个字段的话装备要到下次出击归来才出现。
  现在解除的那一次母港响应带上受影响装备种类的整份未装备列表。这个字段没有真实样本，形状取自客户端读取处。
- 跨基地移动拒绝配置転換中的中隊（原先只靠客户端拦，伪造请求能让一个槽位出现两行配属中）。
- 加列与回填放进同一个事务。
- 同槽两行都在转换中时，显示后摘下的那一行。

## Stop Conditions

- 加列的回填需要比「成功一次」更复杂的判断时停下，改写正式的迁移步骤并先问用户。
- 无头客户端在母港收到 `api_plane_info` 后报错时停下，按快照核对字段形状。

## Verification

`cargo fmt --all --check`；`cargo clippy --workspace --all-targets -- -W warnings`（基线 17）；
`cargo test --workspace --exclude emukc_time --no-fail-fast`；`make headless-check SCENARIO=air_corps_6_4`。
战斗 golden 不应变化（本计划不碰战斗模拟）。
