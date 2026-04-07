# KanColle Map / Route Research

> 这份文档记录当前 map 子系统在 EmuKC 里的实际数据链路，以及它和 battle 子系统之间还剩哪些 fidelity gap。

## TL;DR

- 当前 map runtime 主要依赖 repo-tracked `wikiwiki_map_catalog.json`，并已经恢复了显式起点语义。
- `kc_data` 现在主要承担两类职责：
	- 补 wikiwiki 没覆盖到的结构化地图元数据
	- 在 wikiwiki 起点规则缺失时，为 `cell_0` 提供结构化 start fallback
- 当前 asset 状态是 **130 maps / 131 variants**；其中 **7** 个 variant 仍带 warning，`Unknown = 4`、`SourceUnknown = 0`。
- map 侧最明显的 “起点后直接飞到 A” 问题已经修复；battle 侧的主要 fidelity 风险已转向**敌方属性/装备数据源不足**。

## Current Data Path

### 1. Bootstrap builds the semantic map catalog

当前主链路：

1. 解析 wikiwiki route / enemy / drop 资料
2. 生成 repo-tracked `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`
3. runtime 加载该 catalog 作为常规图 map semantics 的主来源
4. 用 `kc_data` 和 public overlay 补结构信息与缺失信息

这意味着：

- wikiwiki 负责**分歧语义、敌舰编成、掉落等“玩法语义”**
- `kc_data` 负责**结构化 cell 元数据与 start-edge 兜底**

### 2. Start routing is now first-class data

起点处理已经不再靠 graph-root 猜测，而是：

- 统一把 `出撃` / `出撃ポイント` / `スタート` / `Start` 规范化为 `Start`
- 把显式起点规则编译成 `routing_rules[0]`
- 把起点 label 保留到 runtime `MapCellDefinition.node_label`
- wikiwiki 起点缺失时，由 `kc_data` start edge 做 `structural_start_fallback`

当前 repo asset 中：

- “多起点但没有任何 start rule，只能 runtime 硬猜”的残留数已经是 **0**

### 3. Runtime uses the compiled catalog directly

sortie runtime 现在直接消费：

- `cells`
- `routing_rules`
- `enemy_fleets`
- `ship_drops`

`start_sortie()` 与 `next_sortie()` 都走同一套 route evaluator，而不是对 `cell_0` 做特殊“取第一个 next cell”的旧逻辑。

## Current Asset State

### Coverage snapshot

- maps: **130**
- variants: **131**
- variants with warnings: **7**
- `Unknown` predicates: **4**
- `SourceUnknown` predicates: **0**
- `structural_start_fallback`: **3**

结论：

- 常规图的**起点 fidelity 问题已经从系统性缺陷降到少量 warning / prose 覆盖问题**
- 剩余 map fidelity 的主要工作已经不是“补 start routing”，而是“继续减少 unsupported route prose / predicate”

## Enemy Data and Battle Integration

### What map data already gives battle

当前 map catalog 已经能稳定提供：

- encounter cell 结构
- enemy fleet compositions（以 ship ID 为主）
- drop candidates
- 分歧语义与到达路径

这对 sortie / battle 对接已经足够支撑：

- 进入哪一格
- 这一格刷哪组敌编成
- 这一战结束后可能出现哪些掉落

### What map data does **not** solve

map catalog 并不会自动补齐 battle 所需的完整敌舰属性。当前 `build_sortie_enemy_ship()` 仍然是：

1. 先尝试 `codex.new_enemy_ship(ship_id)`
2. 再尝试 `codex.new_ship(ship_id)`
3. 失败时退回 manifest-only fallback

这意味着 battle 侧仍然受限于：

- 当前 repo-tracked normal map 中出现的敌舰 ID 已经被 `enemy_ship_extra` 全覆盖
- 但这条覆盖还需要被持续守住，并扩展到未来新增 map / enemy corpus
- 一旦落回 manifest-only fallback，敌方装备 / slot / onslot 细节仍会退化
- 某些 battle payload 因此仍只能做到“自洽”，还做不到“完全像线上”

因此，**map fidelity 的主问题已经不是 route graph；battle fidelity 的主问题则变成“如何稳定守住并扩展当前 enemy bootstrap coverage，同时继续压缩 fallback 退化面”**。

## What Is Solved vs. What Remains

### Already solved

- 显式起点 `出撃` / `出撃ポイント` 已进入 AST
- `cell_0` 不再默认跳到 alphabetically / numerically first node
- `MapCellDefinition` 保留 `node_label`
- repo asset 中 “inferred multi-root start without rule” 已清零
- boss-route 类测试已经按 runtime-valid path 运行

### Still open

- `Unknown = 4`：仍有少量 wikiwiki 路由文本没有结构化
- strict immediate-arrival-sensitive routing 尚未进入 IR；当前 repo asset 里只有 4 条 `VisitedNode` route-history 规则（4-5 / 5-5 / 7-4），现有 `visited_cell_ids` 已可表达
- 通用 cross-source merge 仍主要依赖 `cell_no`，`node_label` 只是保留下来，还不是权威 join key
- battle 侧仍缺少稳定、完整的 enemy master-data source

## Practical Reading of the Current System

如果要理解当前 map/battle 边界，可以按下面的方式看：

- **Map subsystem**：已经能较忠实地决定“舰队会去哪里、会遇到谁”
- **Battle subsystem**：已经能在现有单舰队框架下较稳定地结算“这一战怎么打”，并修掉了沉船保护、torpedo payload direction 等重大错误
- **Remaining fidelity gap**：主要集中在“敌方完整战斗属性从哪里来”以及“更复杂 route prose / arrival context 规则如何结构化”

## Recommended Next Work

1. 继续消化 `Unknown` route predicates，降低 `variants_with_warnings`
2. 把 `node_label` 从“保留信息”推进到“更稳定的 merge identity”
3. 继续扩展 battle-ready 的敌方属性/装备数据源覆盖面，并缩小 manifest-only fallback
4. 再考虑 combined / event / arrival-context 这类高阶 fidelity 议题
