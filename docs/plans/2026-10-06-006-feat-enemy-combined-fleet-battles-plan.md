---
title: "Enemy Combined Fleet Battles - Plan"
type: feat
date: 2026-10-06
status: implemented
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Enemy Combined Fleet Battles - Plan

## Goal Capsule

让单舰队打敌方联合舰队的战斗能走通（`ec_battle`、`ec_midnight_battle`），用 KCNav 的实测编成驱动 6-5 的 M 格。
做完之后 `wikiwiki_map_catalog.json` 没有任何消费者，可以连同其代码一起删除。

## Product Contract

### Problem Frame

- 6-5 的 boss 格 M 在真实游戏里是敌方联合舰队：KCNav 的 6 组编成全是主力 6 艘 + 护卫 6 艘、阵形 13。
- 本项目给它的是 wikiwiki 抄来的 6 艘单舰队、阵形 6（把联合舰队当单舰队抄了），客户端按普通战斗打。
- `kcnav normalize` 目前跳过 `escortFleet` 非空的记录，所以 M 格只能继续读 wikiwiki 资产；
  计划 `2026-10-06-001` 的 U5 / U6 和 `2026-10-06-003` 的 U6 第 3–5 步因此挂起。

### 已核实的前提

- `docs/api_coverage.md` 写着「常规图里没有一个超过 6 艘的敌方编成，敌方联合舰队只在活动图出现，没有数据可驱动」。
  **这句已经不成立**：6-5 M 就是，数据在 `.data/temp/kcnav/6-5/edge_13_enemycomps.json`、`edge_18_enemycomps.json`。本计划收口时订正。
- 敌方联合舰队在常规图里只此一处（全量同步的 37 张图里，带 `escortFleet` 的记录只在 6-5，共 6 条）。
- 我方联合舰队对敌方单舰队的 9 个端点已在 2026-09-21 交付；剩下 5 个都是敌方联合：
  `ec_battle`、`ec_midnight_battle`、`ec_night_to_day`、`each_battle`、`each_battle_water`。
- 倍率与修正表在 `crates/emukc_battle/src/combined.rs`，出处 `docs/battle/combined-fleet-reference.md`；
  其中「連合 vs 連合」的修正表上游标为要検証，夜战选哪一队作对手有一套评分规则（同文档 §Night battle opponent selection）。
- 协议字段在 `docs/apilist.txt`：`ec_battle` 第 2501 行起，`ec_midnight_battle` 第 2653 行起。
- 客户端按什么决定调 `ec_battle` 而不是 `battle`，要在 `main.js` 里确认（预期是 `api_req_map/next` 返回里敌方为联合的标记）。

### Key Decisions

- **KD1 只做常规图用得到的两个端点**：`ec_battle` 与 `ec_midnight_battle`（单舰队 vs 敌联合）。6-5 不能用联合舰队出击，
  `each_battle*` 没有常规图可以触发，留给活动图；`ec_night_to_day` 同理。
- **KD2 编成模型加护卫队**：`EnemyComposition` 加 `escort_ship_ids` 与 `escort_levels`，空即单舰队。
  `kcnav normalize` 不再跳过联合记录，阵形原样保留（11–14 是联合阵形）。
- **KD3 战斗模拟按参考文档的阶段顺序实现**，没有可靠数值的修正项（上游标 `?` 的命中 / 回避）不建模，与现有 `combined.rs` 的取舍一致。
- **KD4 验证靠协议校验而不是数值对拍**：没有可对拍的参考实现；用 `battle validate` 的客户端规则确认包结构，
  再用固定种子冻结一份 6-5 M 的 transcript。

### Requirements

- R1 6-5 的 M 格出现 12 艘的敌方联合舰队，客户端能正常演出昼战与夜战并结算。
- R2 `kcnav_enemy_fleets.json` 覆盖全部战斗格，组装不再读 wikiwiki 的敌方编成。
- R3 `wikiwiki_map_catalog.json`、`wikiwiki_map_asset.rs`、`wikiwiki_map_download.rs`、`wikiwiki-map sync`、
  `label_overlay.rs` 里贴 wikiwiki 编成的分支、`EnemyComposition.raw_ship_names` 全部删除；`wikiwiki-map build-overlays` 挪到 `map build-overlays`。
- R4 `docs/api_coverage.md` 的端点计数与那段「没有数据」的说明更新。

### Scope Boundaries

- 不做 `each_battle`、`each_battle_water`、`ec_night_to_day`。
- 不做基地航空隊对敌联合的部分，除非 6-5 的现有基地航空逻辑因为敌方变成 12 艘而出错（U1 检查）。

## Implementation Units

- **U1 协议与触发条件**：读 `apilist.txt` 与 `main.js`，列出 `ec_battle` / `ec_midnight_battle` 相对普通战斗多出和改名的字段，
  以及 `next` 响应里让客户端走 `ec_` 路径的字段。产出写回本计划。
- **U2 数据**：模型加护卫队字段；`kcnav normalize` 收联合记录；重生成资产。此时 M 格已有 12 艘的编成但战斗还不能打，
  所以 U2 与 U3 同一个 PR 交付。
- **U3 昼战 `ec_battle`**：敌方分主力 / 护卫的阶段顺序、目标选择、修正表；handler 与路由注册。
- **U4 夜战 `ec_midnight_battle`**：对手队伍的评分选择。
- **U5 结算**：`api_req_combined_battle/battleresult` 对「我方单舰队、敌方联合」的分支；MVP、掉落、血条照常。
- **U6 wikiwiki 退役**（R2、R3）：前提是覆盖报告显示没有格子还依赖 wikiwiki 编成。
- **U7 收口**：文档、`PROJECT_MEMORY.md`、golden（6-5 不在现有 transcript 里，新增一份）。

## U1 的结果（2026-10-06）

- **触发条件是格子的 `event_kind`。** 客户端 `map_info.isVS12()` 是 `5 == type || 7 == type`（`main.decoded.js:21206`），
  为真时昼战请求 `ec_battle`（我方联合则是 `each_battle*`），夜战请求 `ec_midnight_battle`。5 是敌联合，7 是对敌联合的夜昼戦。
- **codex 里 6-5 的 M 有两个格子**：13 号是 `(5, 5, 5)`，18 号是 `(5, 5, 1)`。也就是说从 13 号边进 M 时客户端会请求 `ec_battle`，
  而服务端没有这个接口，且 `sortie/setup.rs` 与 `select_locked_enemy_composition` 都以 `event_kind != 1` 拒绝——**这条路现在走不通**；
  从 18 号边进 M 则被当成普通战斗打。两个格子都应是 5。`event_kind` 的来源是真实起点抓包，KCNav 的路线文档没有这一列，
  所以这一处用「该节点的实测编成全是联合舰队」来定：节点有联合编成，进它的每个格子 `event_kind` 都是 5。
- **阶段顺序、修正值、夜战对手的选择规则都已在 `docs/battle/combined-fleet-reference.md`**（§Friendly single vs enemy combined、
  §Night battle opponent selection），不需要再查。昼战是：航空 → 先制对潜 → 开幕雷击（打敌两队）→ 对敌护卫队炮击一轮 →
  雷击 → 对敌主力炮击一轮 → 任一方有戦艦級时再对全体一轮。
- **协议**（`docs/apilist.txt:2501` 起）：敌方多出 `api_ship_ke_combined`、`api_ship_lv_combined`、`api_eSlot_combined`、
  `api_eParam_combined`，HP 多出 `api_nowhps_combined` / `api_maxhps_combined`，航空与基地航空多出 `api_stage3_combined`，
  攻击目标下标 1–6 是主力、7–12 是护卫。
- **实现形态**：我方联合已有先例——`BattleState` 把两队放在一个连续向量里，用 `CombinedLayout.escort_start` 分界，各阶段在这个空间里算，
  `finalize_day` 再经 `combined_packet` 翻译成客户端的下标。敌方照此加一个 `enemy_escort_start`，不另起一套状态。

## 实施记录（2026-10-07）

U1–U7 全部完成。

- **数据**：`kcnav_enemy_fleets.json` 收了 6-5 M 的 6 组联合编成（两个格子各 3 组，主力 6 + 护卫 6，阵形 13）。
  组装末尾有一步 `mark_enemy_combined_cells`：某格的实测编成全是联合舰队，就把它的 `event_kind` 记为 5。
- **昼战**：`simulate_day_enemy_combined`。敌方两队放在一个连续向量里，各阶段在这个空间里算，
  `finalize_day` 再把敌方下标翻成客户端的空间（护卫队固定从 6 开始）。炮击三轮分别对护卫队、主力、全体，
  落在 `hougeki1` → `raigeki` → `hougeki2` → `hougeki3`。
- **修正**：炮击与雷击走 `combined_correction_vs_enemy_combined`；航空攻撃的 −10 / −20 加在 `kouku.rs` 的基本攻撃力上。
- **夜战**：`night_enemy_deck` 按护卫队的状态评分选对手，只打选中的那一队，响应里用 `api_active_deck` 告诉客户端。
- **结算**：复用 `battleresult`，敌方 12 艘一起计。
- **校验器**：`api_ship_ke_combined` 等六个数组并入逐舰检查。
- **wikiwiki 退役**：见计划 `2026-10-06-003` 的收口一节。

### 与计划不同或计划没写到的地方

- **开幕雷击敌方两队都参加，闭幕雷击只有护卫队。** 参考文档只对闭幕雷击点名了护卫队，开幕没有限定，按字面实现。
- **开幕对潜没有限制敌方哪一队。** 参考文档只写了我方开幕对潜。
- **夜战结算修了一个既有问题**：会话在夜战后保存的敌方 HP 取自夜战包，而夜战包报的是入夜时的 HP，
  所以夜战击沉的敌舰不计入结算（击沉数、旗舰击沉、血条）。现在改为取夜战结束后各舰的实际 HP。单舰队也受这次修正影响。
- **验证方式**：`battle sim` 只打出击后的第一场战斗，到不了 boss 格。改为测试
  `enemy_combined_boss_runs_day_night_and_result` 把出击状态直接放在 6-5 的 18 号格上跑完昼战、夜战、结算，
  昼夜两个包各过一遍客户端规则校验；设 `EMUKC_DUMP_DIR` 可把两个包写出来，昼战包可再用 `battle validate` 看。
  `battle validate` 命令本身只认昼战包。没有冻结 transcript：出击入口用的是不可注入的生产随机源。
- **没有在浏览器客户端里实际打过。** 校验器的规则和战斗包出自同一轮工作，两边若对协议有同样的误解，校验照样通过。

### 遗留

- `each_battle`、`each_battle_water`、`ec_night_to_day` 仍未实现，常规图触发不到。
- 夜战对手评分里上游标为未验证的三条（旗舰中破 / 大破的分值、护卫 5 艘以上、PT 与潜水艦）按文档主规则实现。
- 敌方护卫队的旗舰不享受旗艦援護。

## Verification Contract

三道质量门以退出码为准；`cargo run -- battle validate` 对 6-5 M 的昼战与夜战包无 finding；
仓库内 grep `wikiwiki_map`、`enemy_nodes` 无命中（`docs/plans/`、`docs/solutions/` 的历史叙述除外）。
