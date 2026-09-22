# KanColle Map / Route Research

> 这份文档记录当前 map 子系统在 EmuKC 里的实际数据链路，以及它和 battle 子系统之间还剩哪些 fidelity gap。

## TL;DR

- 当前 map runtime 主要依赖 repo-tracked `wikiwiki_map_catalog.json`，并已经恢复了显式起点语义。
- `kc_data` 现在主要承担两类职责：
	- 补 wikiwiki 没覆盖到的结构化地图元数据
	- 在 wikiwiki 起点规则缺失时，为 `cell_0` 提供结构化 start fallback
- 成品 catalog 状态（2026-09-22 实测）：**37 maps / 38 variants / 2091 条路由规则**，`Unknown` 与 `SourceUnknown` 均为 **0**。
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

以 `build_final_map_catalog_from_repo_assets(".data/temp", &manifest)` 实测，2026-09-22：

- maps: **37**（1-1…7-5 共 36 张，加上只有 kcdata 拓扑的 5-6）
- 路由规则: **2091**
- `Unknown` / `SourceUnknown` predicates: **0**
- topology warnings: **2**

数字要这样读：

- **规则条数不等于覆盖率。** wikiwiki 的路由是 label 键的 overlay，套到 kcdata 拓扑上时一个 label
  可能对应多个 cell，规则会扇出。资产里 7-3 是 116 条，成品里是 186 条，差额全来自扇出。
- **5-6 ラバウル方面海域 有拓扑但零路由规则**，因为 wikiwiki 没有这张图的页面。这是数据缺口，
  不是管线缺陷——它和下面那条 7-3 的性质完全不同。

### 分歧条件的真实覆盖

按 `.data/temp/wikiwiki_map/extracted/*.txt` 的 `ROUTE TABLE` 段统计（36 张图），再与成品 catalog 对照：

| 条件族 | 原文出现 | 涉及图 | catalog 状态 |
| --- | --- | --- | --- |
| 索敵 (LoS) | 73 | 14 | 已覆盖 |
| 高速/低速 (Speed) | 73 | 19 | 已覆盖 |
| 経由/通過 (VisitedNode) | 6 | 4-5, 5-5, 7-4 | 已覆盖（11 条，逐图对得上） |
| 電探 (EquipmentCount) | 3 | 3-2 | 已覆盖（3 条） |
| **ドラム缶** | **5** | **2-5, 5-3, 5-4, 5-5** | **缺，全局 0 条** |
| **大発動艇系** | **4** | **5-3, 5-4, 5-5** | **缺** |

注意：直接 grep 整份文本会把 ドラム缶 的命中放大到 25 张图——那些几乎全在 ENEMY TABLE 的
搬运加成里（如 2-4 的「燃料+25～60:ドラム缶(+2)」），不是分歧条件。只能在 `ROUTE TABLE` 段内统计。

补这批数据卡在一个结构问题上：`wikiwiki_map_catalog.json` 的生成器（曾经的
`parser/wikiwiki_map/html.rs`）已经从仓库里移除，`.data/temp/wikiwiki_map/` 下没有任何一份
agent JSON 能再生出当前资产（md5 全不匹配），而管线也没有「人工修正」这种输入位
（`load_repo_source_set` 里 `wikiwiki_overlay: None`，overlay 全靠从大资产自动推导）。

### 数据源与 Single Source of Truth

按数据种类（拓扑 / 路由规则 / 敌方编成 / 敌舰属性 / 掉落 / 开放条件）的完整依赖梳理，
包括每一类的唯一来源、生成器是否还在、断链后果，见
[data-dependencies.md](./data-dependencies.md)。

已修掉的两处 SSOT 破坏：`wikiwiki-map normalize` 曾把合并后的成品写回 wikiwiki 源槽位
（`8d0376a8`），掉落曾只存在于那份资产因而锁死了它的再生（`ea40629b`）。

### 地图开放条件是公式，不是数据

`build_regular_prerequisites()` 按两条结构规则生成 62 条前置关系。**没有任何上游来源**：
`api_mst_mapinfo` 只有 `api_level` / `api_required_defeat_count` / `api_sally_flag`，
wikiwiki 抽取文本 36 份里 0 份含开放条件。它对每个海域生成 `2..=9` 号图，所以约一半条目指向
不存在的地图——无害（级联按 profile 已有的 `map_record` 行查），但**条目数不能当覆盖率读**。
详见 `codex/map.rs::build_regular_prerequisites` 的文档注释。

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
- strict immediate-arrival-sensitive routing 尚未进入 IR。成品 catalog 里有 11 条 `VisitedNode`
  规则（4-5 / 5-5 / 7-4），现有 `visited_cell_ids` 已可表达；缺的是「只看这一次从哪条边进来」——
  需要 `FleetRouteContext` 带 `arrival_from_cell_id`、一个 `ArrivedFrom` predicate，以及只在原文
  明写「Xマスから来た場合」时才 lowering 的规则。原文只写「経由」时不要用它
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
