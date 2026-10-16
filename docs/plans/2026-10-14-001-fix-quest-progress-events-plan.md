---
title: "Quest Progress Events - Plan"
type: fix
date: 2026-10-14
status: implemented
execution: code
---

# Quest Progress Events - Plan

Issues #110、#111、#115、#121。四处都是「游戏里发生了某件事，任务没有记上，或记错了」：改修不产出事件、
出击事件不带海图阶段、Bw1 的出击次数按战斗算、远征任务的条件列表匹配不上代号且把「每个都要」当成「任选」。
每个 issue 的 Agent Brief 是验收合同，这份计划只补代码证据、实施顺序和 brief 里没写的坑。

## Evidence

- **#110** `Ctx::remodel_slot`（`game/remodel_slot.rs:189`）开事务、扣材料、掷成功、`tx.commit()`，中间没有
  `GameplayOutcome`。`GameplayOutcome::SlotItemImproved` 带 `#[allow(dead_code)]`（`quest/observe.rs`），
  matcher 的 `Factory::SlotItemImprovement` 早已处理。成功与失败都走到同一个 `commit`，事件放在它之前即可。
- **#111** `build_sortie_battle_outcome`（`game/sortie_result.rs:460`）只填海域、图号、是否 boss、评价、舰队。
  结算里 `apply_sortie_map_result`（约 210 行）**先于**它执行（234 行），会推进地图记录的 `gauge_index`。
  `sortie_map_info_matches`（`thirdparty/quest/matcher.rs` 约 388 行）要求 `phase.is_none()`。7-2 在码典里
  `gauge_count: 2`、变体 `phase1`/`phase2`、`reset_policy: Monthly`。
- **#115** `Kc3rdQuestConditionSortie` 只有 `defeat_boss`、`fleet_id`、`map`、`result`、`times`。tsunkit 解析的
  `extract_requirements_sortie`（`parser/tsunkit_quest/requirement/sortie.rs`）拿不到任务号；任务号 `game_id` 在
  `parser/tsunkit_quest/mod.rs` 那一层。`Ctx::start_sortie`（`game/sortie/mod.rs:218`）有事务，没有 `observe`。
  `apply_sortie_event` 已经收 `master_condition`（`reset_sortie_cycle` 就从 master 拷 `map`）。
- **#121** `Kc3rdQuestCondition::Expedition(Vec<Kc3rdQuestConditionExpedition { list, times }>)`；matcher 对内层
  Vec 找第一个匹配且 `times > 0` 的项减一。列表项与 `mission_id` 比字符串或整数，`"A1"` 永远不等于 100。
  `apply_event_with_context` 已有 `Option<&Codex>`，`observe` 传的是 `Some(codex)`；`codex.expedition_conditions`
  按 id 存着 `code`。tsunkit 原始数据里 410（任选）与 426（每个）形状相同，区分只能按任务号。

## Decision

1. **#110** 每次 `remodel_slot` 产出一条 `SlotItemImproved { item_mst_id, stars }`（尝试之后的星级），
   在 `commit` 前 `observe`。失败也产出（KC3Kai 无条件计数，见 brief）。`remodel_slot_recover` 不产出。
2. **#111** 出击事件加 `gauge_phase: i64`。取值用**出击开始时**的 `gauge_index`：`start_sortie` 已经调了
   `find_map_record_impl`，把它存进 `ActiveSortieState`，结算时直接读。比「在 `apply_sortie_map_result` 之前再读
   一次记录」好：不依赖结算函数内部的语句顺序，以后调整顺序不会静默出错。旧的进行中出击（`ActiveSortieState`
   反序列化缺字段）按 serde 默认 1 处理。matcher：`phase: Some(n)` 只匹配 `gauge_phase == n`，`None` 匹配任意。
3. **#115** `Kc3rdQuestConditionSortie` 加 `count_at_sortie_start: bool`，`#[serde(default)]`：码典重建前的旧
   json 与旧进度行里都没有它。matcher 判断时**读 master 的字段**，不读进度行里的，旧行因此不必迁移。
   解析器在 `mod.rs` 按 `game_id == 214` 给第一个 `Sortie` 条件打标记。新增 `GameplayOutcome::SortieStarted
   { maparea_id, mapinfo_no, fleet_id }` 与对应的 `QuestActionEvent`，`start_sortie` 在 `commit` 前 `observe`。
   带标记的条件只认 `SortieStarted`，不认 `SortieBattleCompleted`；其余条件反之。
4. **#121**
   - 代号在**匹配时**解析：列表项等于 `mission_id` 的字符串，或等于 `codex.expedition_conditions[mission_id].code`。
     旧进度行里的 `"A1"` 因此也能匹配。
   - 「每个都要」在**解析时**展开：`{ list: [a, b, c], times: n }` 拆成三项单元素的 `{ list: [a], times: n }`，
     放进同一个 `Expedition(Vec)`。顶层条件个数不变，进度行与 master 按下标对齐不受影响；matcher 现有的
     「第一个匹配且未完成的项减一」对单元素项就是逐个计数，matcher 不用改语义。
   - 410、411 在解析器按 `game_id` 保留原列表（任选、共用计数）。
   - 旧进度行保持旧形状，在重置前仍是「任选」。宽松方向，不迁移。

## Implementation Units

- **U1 #110** `remodel_slot` 产出并观察事件；去掉 `dead_code` 与 `observe.rs` 模块注释里「没有生产者」的说法；
  改 `docs/solutions/architecture-patterns/quest-observation.md`（Variants without a producer stay 一节）与
  `quest.md`（Slot item improvement 一条）。测试放 `tests/gameplay_tests/quest/`：619 成功一次完成、强制失败一次
  也完成（`certain=false` 时要能控制掷点；若没有现成的注入点，用 ★ 等级使成功率为 0 或改用可注入的随机源，
  选现有代码里已有的做法）、1167 需三次、未接取的不动、`remodel_slot_recover` 不推进。
- **U2 #111** `ActiveSortieState.gauge_phase`、事件字段、matcher 比较。单元测试改
  `build_sortie_battle_outcome_*` 两个用例补字段；matcher 测试：7-2 phase 2 的 S 胜移除 872/893 的 phase 2 项、
  phase 1 的战斗只移除 893 的 phase 1 项、打空第一条血的那场算 phase 1。
- **U3 #115** 模型字段、解析器标记、`SortieStarted` 事件与 `observe`。`matches_event` 的表要加
  `(Sortie, SortieStarted)`，对所有 `Sortie` 条件放行（它拿不到 master），由 `apply_sortie_event` 按 master 的
  标记过滤：带标记的只认 `SortieStarted`，不带的只认 `SortieBattleCompleted`。重建码典
  `cargo run -- bootstrap --codex-only`，确认 `quest.json` 里只有 214 的第一个条件带标记。测试：一次出击三场战斗
  只加 1；进图后不打就撤退也加 1；216、210 仍按战斗计。
- **U4 #121** matcher 代号解析、解析器展开与 410/411 例外，重建码典。测试一律走 `apply_event_with_context`
  并传 codex（`apply_event` 没有 codex，解析不了代号）。集成测试覆盖 427、426、428、410、411 与失败不计数，
  见 brief 的验收条件。`tests/gameplay_tests/quest/progress.rs` 里用 `"A1"` 的 `is_satisfied` 用例不受影响。
- **U5** `docs/solutions/architecture-patterns/quest.md` 的 progress tracking 一节补三条规则：阶段匹配、
  出击开始计数只给 Bw1 首项、远征列表逐个计数（410/411 例外）。

U1–U4 互不依赖，按编号顺序做，每个 U 单独可测。U2 与 U3 都改 `SortieBattleCompleted` 相关代码，同一分支上
先 U2 后 U3 可以少解一次冲突。

## Risks

- **U3 的事件名与计划 004 相关**：#114 要给 `SortieBattleCompleted` 加舰队快照。计划 004 在本计划之后做。
- **U4 的展开依赖码典重建**：没重建码典就跑测试，426 仍按旧形状走。实施记录里写明重建过。
- **展开的范围**：只作用于 `Expedition` 条件，其他条件里的列表（如 `Sortie.map.All`）不受影响。
- **7-2 跨月重置**：出击途中跨过月初、地图被重置回 phase 1，事件仍报出击开始时的阶段。边角，不处理。
- **U1 的强制失败**：如果 `roll_success` 没有可控入口，不要为了测试引入新的随机抽象；用成功率为 0 的配方阶段
  或已有的测试种子机制。找不到就在实施记录里说明用的是哪种。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test --workspace --no-fail-fast`，报告 ignored 数。
- 重建码典后 `quest.json` 的抽查：214 首项带标记；426 的 `Expedition` 有 4 项；410 仍是 1 项双元素列表。
- 不需要客户端演示：任务列表的报文形状不变。
