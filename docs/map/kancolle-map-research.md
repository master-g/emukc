# Map / Enemy Data Research

这份文档只记录**当前仓库代码和当前 repo asset 已经证实的事实**，不再保留早期调研里已经过时的设计草案。

## TL;DR

- 地图主语义源已经是 `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`。
- `bootstrap` 会优先加载这份 repo asset，再用 `kc_data` 补 wikiwiki 未覆盖的结构。
- 当前 runtime 已能按 cell/route/enemy_fleet/ship_drop 走完整个常规图主链路。
- 当前地图敌舰**编成 ID** 已经可用，但敌舰**完整属性与敌装**仍然不完整。

## 当前地图数据链路

当前代码的实际链路在这些文件里：

- `crates/emukc_bootstrap/src/parser/mod.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/resolver.rs`
- `crates/emukc_model/src/codex/map.rs`
- `crates/emukc_gameplay/src/game/sortie.rs`

按执行顺序看：

1. `parse_partial_codex()` 读取 `start2.json`、`kcwiki_*`、`kc_data` 等输入。
2. `load_map_catalog()` 优先加载 repo-tracked 的 `wikiwiki_map_catalog.json`。
3. 如果 wikiwiki asset 缺图，再用 `kc_data` 做结构补全，而不是反过来。
4. `Codex.maps` 在运行时提供 `MapCatalog`。
5. `sortie` 逻辑直接消费 `MapVariantDefinition` 的 `cells`、`routing_rules`、`enemy_fleets`、`ship_drops`。

## 当前 repo asset 已覆盖的内容

直接检查 `crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json`，当前状态是：

- `36` 张地图
- `37` 个 variant
- `Unknown = 0`
- `SourceUnknown = 0`
- `parse_warnings = 0`

也就是说，当前 asset 已经覆盖：

- 地图 cell 结构
- route rule
- variant / gauge / clear transition
- enemy fleet composition
- ship drop

这也是当前 `sortie` 链路可以直接依赖 repo asset 的原因。

## 敌舰编成数据现在是怎么来的

### 1. wikiwiki 敌编成表已经能落到 `ship_ids`

`crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs` 会解析 wikiwiki 的敌编成表，产出：

- `EnemyFleetDefinition`
- `EnemyComposition`
- `EnemyComposition.ship_ids`

也就是说，地图层已经不再只是“某格有敌人”，而是已经有“这一格可能出现哪些敌舰 ID 组合”。

### 2. 敌舰名称到 ship id 的映射来自 manifest

`crates/emukc_bootstrap/src/parser/wikiwiki_map/resolver.rs` 的 `ShipResolver::new()` 会从 `ApiManifest.api_mst_ship` 建立名字索引。

它不是硬编码敌舰名表，而是直接基于：

- `api_name`
- `api_yomi`

生成可匹配 label，所以 wikiwiki 敌舰名能被解析到 master ship id。

### 3. runtime 已经会用这些 `ship_ids` 生成敌舰

`crates/emukc_gameplay/src/game/sortie.rs` 的关键路径是：

- `resolve_sortie_enemy_fleet()`
- `select_random_enemy_composition()`
- `build_sortie_enemy_ships()`
- `build_sortie_enemy_ship()`

也就是说，现在战斗里敌舰已经不是纯粹按 map/world 范围硬编码数量，而是会优先使用地图资产里的真实 `EnemyComposition.ship_ids`。

## 当前敌舰属性链路的真实状态

### 1. runtime 会优先尝试 `codex.new_ship(ship_id)`

`build_sortie_enemy_ship()` 先调用 `codex.new_ship(ship_id)`。

这条路径依赖：

- `manifest.find_ship(ship_id)`
- `ship_extra.get(&ship_id)`

也就是说，想走“完整 ship 实例”这条路，敌舰不仅要在 `api_mst_ship` 里存在，还要在 `ship_extra` 里有额外属性和初始装备信息。

### 2. 如果 `ship_extra` 不存在，会退化到 manifest-only fallback

当 `codex.new_ship(ship_id)` 失败时，代码会退化成只用 `manifest.find_ship(ship_id)` 构建一个最小 `KcApiShip`：

- `api_taik` 不存在时，HP 退成 `1`
- `api_houg` / `api_raig` / `api_tyku` / `api_souk` 不存在时，全部退成 `0`
- `slot_items` 为空，所以 `api_eSlot` 最终会是全 `-1`
- `api_onslot` 只有 `api_maxeq` 有值时才会带上

所以当前链路虽然“能生成敌舰对象”，但不等于“已经拿到了完整敌舰属性”。

## 基于当前本地数据的核验结果

我对当前 repo asset 和本地 `.data/codex` 做了实际核验，结论如下。

### 1. 当前地图资产里一共用到了 83 个敌舰 ID

样例来自 `wikiwiki_map_catalog.json`：

- `1-1:A:パターン1 -> [1501]`
- `1-1:A:パターン2 -> [1502]`
- `1-1:A:パターン3 -> [1503]`
- `1-1:B:パターン1 -> [1501, 1501]`

### 2. 这 83 个敌舰 ID 全都存在于 `start2.api_mst_ship`

这一点很重要：**当前 bootstrap 已经足够支撑“地图敌编成 -> 敌舰 master id”**。

换句话说，当前并不存在“wikiwiki 解析出了 ship id，但 runtime manifest 根本找不到”的问题。

### 3. 但这 83 个敌舰 ID 在 `ship_extra.json` 里是 `0/83`

核验结果是：

- `83/83` 存在于 `.data/codex/start2.json`
- `0/83` 存在于 `.data/codex/ship_extra.json`

这意味着当前 enemy ship 基本走不到 `codex.new_ship()` 的完整路径。

### 4. 当前 `start2` 对这批敌舰只给了“身份字段”，没给战斗主属性

对当前 asset 里出现的这 83 个敌舰 ID，`start2.api_mst_ship` 的字段覆盖是：

- 始终存在：`api_name`、`api_yomi`、`api_stype`、`api_ctype`、`api_soku`、`api_slot_num`、`api_sort_id`
- 当前本地数据里全部缺失：`api_taik`、`api_houg`、`api_raig`、`api_tyku`、`api_souk`、`api_maxeq`

这意味着当前 integrated bootstrap 数据**足够知道“这是谁”**，但**不够知道“它具体有多少血、火力、装甲、搭载、敌装”**。

## 现有 bootstrap 输入为什么还不够

### 1. `kcwiki_ship.json` 这条链路今天没有补上敌舰 `ship_extra`

当前 `kcwiki` parser 在：

- `crates/emukc_bootstrap/src/parser/kcwiki/mod.rs`
- `crates/emukc_bootstrap/src/parser/kcwiki/ship.rs`

它确实会生成 `ship_extra`，但当前结果里没有覆盖地图使用到的敌舰 ID。

因此，对“敌舰完整属性”这个问题来说，`kcwiki_ship.json` 在**当前实现**下还不能满足需求。

### 2. `kc_data` 明确把非 ally ship 排除在 ship info 之外

`crates/emukc_bootstrap/src/parser/kcwikizh_kcdata.rs` 在解析 `_ship` 时会跳过 `api_aftershipid.is_none()` 的条目。

对 manifest 来说，这基本就是把敌舰排除掉了。

所以 `kc_data` 当前只补：

- ally ship picturebook / class name 一类信息

它不是敌舰属性源。

### 3. `ships.nedb` 虽然下载了，但当前没有进入解析链

`crates/emukc_bootstrap/src/res.rs` 会下载 `ships.nedb`，但当前代码里没有 parser 消费它。

因此它目前只是“下载到了磁盘”，不是“已经进入 Codex 的敌舰属性输入”。

## 结论：当前 bootstrap 数据源能否满足需求

### 如果需求只是“拿到敌舰编成”

**可以。**

当前链路已经满足：

- 每格敌编成
- 编成里的敌舰 master id
- 基本身份字段（名字、舰种、舰型、速度、slot 数）

### 如果需求是“拿到敌舰完整属性并真实用于战斗”

**还不可以。**

当前链路缺的是：

- HP
- 火力 / 雷装 / 对空 / 装甲
- 搭载量
- 敌方装备列表
- 能稳定生成非空 `api_eParam`
- 能稳定生成真实 `api_eSlot`

所以更准确的说法是：

> 当前 bootstrap 已经能满足“地图敌编成”和“敌舰 ID 解析”，但还不能满足“真实敌舰属性/敌装”的需求。

## 建议的下一步

如果下一阶段目标是把 sortie battle 的敌舰从“manifest-only fallback”推进到“真实属性 + 真实敌装”，建议按这个顺序做：

1. 明确新增一条**敌舰专用 master data** 输入，而不是继续复用 ally `ship_extra`。
2. 优先考虑把 `kcwiki/kancolle-data` 的 `wiki/enemy.json` / `wiki/enemyEquipment.json` 接进 bootstrap。
3. 在 `emukc_model` 里新增敌舰额外数据模型，不和 ally `Kc3rdShip` 强绑。
4. 让 `build_sortie_enemy_ship()` 优先消费这份 enemy extra，再回填 `api_eParam` / `api_eSlot`。

## 代码参考

- `crates/emukc_bootstrap/src/parser/mod.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs`
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/resolver.rs`
- `crates/emukc_bootstrap/src/parser/kcwiki/mod.rs`
- `crates/emukc_bootstrap/src/parser/kcwiki/ship.rs`
- `crates/emukc_bootstrap/src/parser/kcwikizh_kcdata.rs`
- `crates/emukc_bootstrap/src/res.rs`
- `crates/emukc_model/src/codex/map.rs`
- `crates/emukc_model/src/codex/ship.rs`
- `crates/emukc_gameplay/src/game/sortie.rs`
