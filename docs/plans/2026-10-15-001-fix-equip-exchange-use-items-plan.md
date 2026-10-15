---
title: "Equipment-Exchange Quests Consume Their Use Items - Plan"
type: fix
date: 2026-10-15
status: implemented
execution: code
---

# Equipment-Exchange Quests Consume Their Use Items - Plan

Issue #142。647（中部海域「基地航空隊」展開！）领奖时不要求、也不扣設営隊。分诊时查到根因在解析器，受影响的不止 647。

**范围**：修 tsunkit 解析器 `equipexchange` 分类漏掉的道具消耗；修领奖时道具 1～4 走错表（658、693 因上一条会撞上）；改依赖旧行为的测试。
**基线**：`main` `27e7a88d`，分支 `fix/equip-exchange-use-items`。

## Evidence

- **上游数据**：`.data/temp/tsunkit_quests.json` 里 647 的 `requirements` 是 `category: equipexchange`，带 `consume: [{category: inventory, id: 73, amount: 1}]`（73 是設営隊）、`scrap` 两个ドラム缶、`resources: [1200, 0, 0, 3000]`。
  码典的 `detail` 也写着「さらに「設営隊」一つを準備せよ！」。
- **码典现状**：647 的条件只有资源消耗和废弃两个ドラム缶。上游带 `inventory` 消耗的 73 条任务里，62 条 `conversion` 在码典里保留了 `UseItemConsumption`，11 条 `equipexchange` 丢了：646、647、648、650、652、653、658、688、689、690、693（2026-10-15 对照 `.data/codex/quest.json`）。
- **根因**：`requirement/exchange.rs` 的 `extract_requirements_equip_exchange` 只取资源、废弃和 `extract_slotitem_consume`；`conversion.rs` 另外调用了 `extract_useitem_consume`。
- **领奖侧已就绪**：`quest/consume.rs` 的 `handle_consumption` 对 `UseItemConsumption` 调 `deduct_use_item_impl`，道具不足时返回 `Insufficient`，领奖事务整体回滚。
  奖励组装（`get_quest_rewards`）不读条件，响应不变。
- **连带问题**：658、693 消耗的是道具 3（開発資材）。道具 1～4 记在 material 表（`use_item.rs` 的 `MATERIAL_BACKED_MST_IDS`），
  `deduct_use_item_impl` 却查 `use_item` 表，报 `EntryNotFound("use item: 3")`。码典里有 45 条任务（只数顶层 `And`）消耗道具 2～4，修前同样领不了。

## Decision

1. `extract_requirements_equip_exchange` 照 `conversion` 的写法，追加 `extract_useitem_consume` 的结果作为 `UseItemConsumption`。
2. `quest/consume.rs` 的 `handle_consumption` 把道具 1～4 换成对应的 `MaterialCategory` 走 `deduct_material_impl`。只改任务消耗这一处：其余调用方传的是固定的非资材道具，或改修数据里的道具。

## Implementation Units

- **U1 解析器（先写测试）**：在 `requirement/mod.rs` 的测试里，构造 `equipexchange` 加一条 `inventory` 消耗，断言产出 `UseItemConsumption`。然后改 `exchange.rs`。
- **U2a 资材道具（先写测试）**：`consume.rs` 单元测试，消耗道具 3 × 20 从 material 表扣。
- **U2 领奖**：`tests/gameplay_tests/map/unlock.rs` 的 `claim_647` 补上設営隊；新增测试：没有設営隊领 647 失败，有则领奖后設営隊被扣掉。
- **U3 码典**：`cargo run -- bootstrap --codex-only` 重建后，11 条任务都带上游的 `UseItemConsumption`，62 条 `conversion` 不变。

U1 → U2 → U3。
