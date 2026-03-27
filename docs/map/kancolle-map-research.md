# KanColle Sortie/Map System Research

## 研究目标
为 emukc 实现 battle 系统提供出击/地图相关的技术调研：
1. 数据源 - 从哪里获取最新数据
2. 数据结构 - 如何组织地图、敌人、编队移动路径
3. 系统实现 - 如何实现 map/navigate/enemy generation

---

## 一、数据源

### 权威程度排序

| 优先级 | 来源 | URL | 说明 |
|--------|------|-----|------|
| ⭐⭐⭐ | **KC3Kai** | https://github.com/KC3Kai/KC3Kai | 495 stars，活跃，解析游戏数据 |
| ⭐⭐⭐ | **poi** | https://github.com/poi/poi | 活跃的 KanColle 查看器，map 解析 |
| ⭐⭐⭐ | **KcWiki (日) wikiwiki.jp** | http://wikiwiki.jp/kancolle/ | 日语原文，最权威 |
| ⭐⭐ | **KcWiki (中)** | https://zh.kcwiki.cn | 活跃维护，镜像日wiki |
| ⭐⭐ | **kancolle-shinkai-db** | https://github.com/kcwikizh/kancolle-shinkai-db | 深海棲艦 JSON/Lua 数据，维护较少 |
| ⭐ | **KC2-Assets** | 游戏客户端 `kcs2/` 资源 | 地图图片，声音数据，需要解析 |

### poi 的 map 解析参考
- `poi/lib/map.js` - 地图数据解析逻辑

---

## 二、数据结构

### 2.1 地图层级结构

```
海域 (MapArea)           api_mst_maparea
  └── 地图 (Map)         api_mst_mapinfo
        └── 格子 (Cell)  spots array in *_info.json
```

#### MapArea (海域)
**API**: `api_mst_maparea` (start2.rs:159-168)

```rust
pub struct ApiMstMaparea {
    pub api_id: i64,      // 海域 ID
    pub api_name: String,  // "鎮守府海域", "南西諸島海域" 等
    pub api_type: i64,     // 0=普通, 1=活动
}
```

**海域 ID 映射**:
- 1 = 鎮守府海域 (Guardians of the Port)
- 2 = 南西諸島海域 (Southwestern Islands)
- 3 = 北方海域 (Northern Waters)
- 4 = 西方海域 (Western Waters)
- 5 = 南方海域 (Southern Waters)
- 6 = 中部海域 (Central Waters)
- 7 = 南西海域 (Southwestern Sea)
- 42-60 = 活动海域 (Event maps)

#### MapInfo (单独地图如 1-1)
**API**: `api_mst_mapinfo` (start2.rs:187-214)

```rust
pub struct ApiMstMapinfo {
    pub api_id: i64,                    // 地图 ID
    pub api_maparea_id: i64,             // 所属海域 ID
    pub api_no: i64,                    // 地图编号 (1-1 中的 1)
    pub api_name: String,               // "鎮守府南方海域"
    pub api_level: i64,                 // 进入等级限制
    pub api_opetext: String,            // 出击说明文字
    pub api_infotext: String,           // 地图详情文字
    pub api_item: Vec<i64>,             // 道具奖励
    pub api_sally_flag: Vec<i64>,      // 舰队编成标志
    pub api_max_maphp: Option<serde_json::Value>,  // 活动地图 HP 上限
    pub api_required_defeat_count: Option<i64>,    // 活动地图击破数
}
```

**地图 ID 计算公式**: `map_id = maparea_id * 100 + mapinfo_no`
- 1-1 → `api_id = 11`
- 5-5 → `api_id = 55`
- 活动 E-1 → `api_id = 421`

---

### 2.2 地图格子 (Cell/Spot)

**重要发现**: emukc 当前**没有**解析 `*_info.json` 文件中的 `spots` 数组。格子数据是 stub 的。

格子数据存储在 `kcs2/resources/map/{area}/{map}_info.json` 中：

```json
{
  "spots": [
    {
      "api_no": 1,           // 格子编号
      "api_color_no": 0,      // 颜色 (0=起点, 5=boss)
      "api_next": [2, 3],    // 下一格候选
      "api_type": "boss"      // 格子类型
    }
  ]
}
```

**格子类型**:
- normal - 普通战斗格
- boss - BOSS 格
- resource - 资源格
- air_raid - 空袭格

**颜色标识** (`api_color_no`):
- 0 = 起点
- 5 = BOSS
- 其他 = 普通战斗格

---

### 2.3 舰队编成与移动路径

#### api_sally_flag 编码 (map_record.rs:108-111)

```rust
/// [0, x, x] normal fleet enabled, [1, x, x] combined fleet enabled
/// [x, 1, x] Carrier Task Force, [x, 2, x] Surface Task Force, [x, 4, x] Transport Escort Force
/// [x, x, 0] ?, [x, x, 1] 7 ships enabled
pub sally_flag: [i64; 3],
```

**编成标志**:
- `[0][x][x]` = 普通舰队
- `[1][x][x]` = 联合舰队
- `[x][1][x]` = 機動部隊 (CTF)
- `[x][2][x]` = 水上部隊 (STF)
- `[x][4][x]` = 輸送護衛部隊 (TCF)

---

### 2.4 联合舰队 (連合艦隊)

#### 三种联合舰队类型

| 类型 | 日语 | 适用编成 | sally_flag[1] |
|------|------|----------|----------------|
| 機動部隊 (CTF) | Carrier Task Force | 2CV + 护卫 | 1 |
| 水上部隊 (STF) | Surface Task Force | 主力舰编成 | 2 |
| 輸送護衛 (TCF) | Transport Escort | 输送编成 | 4 |

#### CTF 组成限制
- **主力舰队**: 最少 2 CV/CVB/CVL; 最多 2 FBB/BB/BBV + 4 CV/CVB/CVL
- **护卫舰队**: 最少 1 CL + 2 DD; 最多 2 FBB/BB/BBV + 1 CVL + 2 CA/CAV + 1 CL + 1 AV
- **禁止**: 慢速 BB/BBV，任何 CV/CVB 在护卫舰

#### STF 组成限制
- **主力舰队**: 最少 2 FBB/BB/BBV/CA/CAV/CL/CLT; 最多 4 FBB/BB/BBV + 4 CA/CAV + 1 CV/CVB 或 2 CVL
- **护卫舰队**: 最少 1 CL + 2 DD; 最多 2 FBB/BB/BBV + 1 CVL + 2 CA/CAV + 1 CL + 1 AV
- **禁止**: 慢速 BB/BBV (伊丽莎白级需要高速)，任何 CV/CVB 在护卫舰

#### TCF 组成限制
- **主力舰队**: 最少 4 DD/DE; 最多 2 BBV/CAV/CL/CT/AV/AS/AO/LHA + 1 CVE + 1 AR
- **护卫舰队**: 最少 1 CL/CT + 3 DD/DE; 最多 2 CL/CT + 2 CA/CAV
- **禁止**: 任何 FBB/BB/CV/CVB/CA/CLT/SS/SSV; CVL 不能当 CVE
- **旗舰**: 护卫舰队的旗舰必须是 CL/CT

#### 联合舰队战斗流程
| 阶段 | CTF/TCF | STF | 单舰 |
|------|----------|-----|------|
| 侦查 | 主力舰队 | 全部 | 全部 |
| 第一次炮击 | **护卫舰队** 先攻 | **主力舰队** 先攻 | 标准顺序 |
| 第二次炮击 | 主力舰队 | 主力 vs 两舰队 | — |
| 鱼类发射 | 护卫舰队 | 两舰队 | 标准 |
| 夜战 | 护卫舰队 | 护卫舰队 | 标准 |

#### 联合舰队阵型 (巡航阵型)

| 阵型 | ID | 主力舰队效果 | 护卫舰队效果 |
|------|-----|-------------|-------------|
| 巡航 1 (对潜警戒) | 1 | 0.8伤害, 0.9命中, 1.1-1.2回避 | 0.8伤害, 0.7命中, 0.75回避 |
| 巡航 2 (前方警戒) | 2 | 1.0伤害, 1.0命中, 1.3回避 | 1.0伤害, 0.9命中, 1.0回避 |
| 巡航 3 (警戒阵型) | 3 | 0.7伤害, 0.8命中, 1.1回避 | 0.7伤害, 0.6命中, 0.4回避 |
| 巡航 4 (战斗阵型) | 4 | 1.1伤害, 1.1命中, 1.0回避 | 1.1伤害, 1.0命中, 1.2回避 |

---

### 2.5 活动地图 vs 普通地图

#### api_type 区分
```rust
// start2.rs:166-167
pub api_type: i64, // 0: 普通海域, 1: 活动海域
```

#### 活动地图特有字段

**HP 系统** (`KcApiEventmap`):
```rust
pub struct KcApiEventmap {
    pub api_max_maphp: i64,      // 最大 HP
    pub api_now_maphp: i64,      // 当前 HP
    pub api_selected_rank: i64,  // 难度 (1=丁, 2=丙, 3=乙, 4=甲)
    pub api_state: i64,           // 状态 (1=默认, 2=完成)
}
```

**HP 表类型** (`MapGaugeType`):
- 1 = 撃破 (击破数)
- 2 = HP (击破 BOSS HP)
- 3 = 揚陸 (登陆点数)

**难度选择** (`MapSelectRank`):
```rust
pub enum MapSelectRank {
    NotSet = 0,
    Casual = 1,  // 丁
    Easy = 2,    // 丙
    Normal = 3,   // 乙
    Hard = 4,     // 甲
}
```

#### 活动地图资源结构

**普通地图** (海域 1-7):
```
kcs2/resources/map/{area}/{map}.png
kcs2/resources/map/{area}/{map}_image.png
kcs2/resources/map/{area}/{map}_info.json
```

**活动地图** (海域 42-60):
```
kcs2/resources/map/{area}/{map}.png
kcs2/resources/map/{area}/{map}_image.png
kcs2/resources/map/{area}/{map}_info.json
kcs2/resources/map/{area}/{map}_info{N}.json   # 子格子
kcs2/resources/map/{area}/{map}_image{N}.png
```

活动地图有多个子格子变体 (如 `_info14.json`, `_image22.json`)，表示不同路线/阶段。

---

### 2.6 敌人数据

**重要发现**: emukc 当前使用程序生成的敌人 (ship ID 412)，没有真实的敌人舰队组成数据。

**当前简化实现** (`sortie.rs:395-427`):
```rust
let enemy_count = match active.map_id {
    11..=22 => 1,   // 世界 1-1 到 2-2
    23..=34 => 2,   // 世界 2-3 到 3-4
    35..=54 => 3,   // 世界 3-5 到 5-4
    _ => 4,         // 世界 6+ 和活动
};
let enemy_level = map_level.max(1) * 5 + cell_id;
```

**实际 KanColle 敌人数据**:
- 每个格子有特定的敌人舰队组成
- 每个格子有多个 "comp" 变体
- 舰船 ID 引用 `api_mst_ship` (敌人舰船定义)
- 装备数据定义在每个敌人舰船中

---

### 2.7 BOSS 特殊处理

BOSS 格子通过以下识别:
- `api_bosscell_no` - BOSS 格子编号
- `api_color_no = 5` - BOSS 颜色
- `api_boss_bgm` - BOSS 专用 BGM

BOSS HP 表 (活动地图):
- `api_max_maphp` / `api_now_maphp` - HP 值
- `api_required_defeat_count` - 需要击破次数
- 多个 gauge_num 表示多重 HP 表

---

## 三、当前 emukc 实现状态

### 已实现
- ✅ `ApiMstMaparea` / `ApiMstMapinfo` / `ApiMstMapbgm` 解析
- ✅ `MapRecord` 数据库实体
- ✅ `SortieStartResponse` 处理
- ✅ `BattleContext` 含阵型/交战类型
- ✅ `ActiveSortieState` 在内存 sortie 状态
- ✅ Day battle simulation (`battle/core.rs`) — complete day battle with all phases
- ✅ Practice battle system (`battle/practice.rs`) — full rival system, exp calculation
- ✅ Boss defeat handling, defeat count, first clear logic

### 未实现 (Gap)
- ❌ 地图格子 (`spots`) 解析 - `spots: Vec<serde_json::Value>` 原始 JSON，从未反序列化
- ❌ 路径决定 - 没有基于舰船类型的路由逻辑（`api_soku` 速度、`api_sakuteki` 索敌值存在但未使用）
- ❌ 敌人舰队组成 - `build_sortie_enemy_ships()` 所有敌人都是 ship ID 412
- ❌ 阵型验证 - 直接传递玩家选择的阵型
- ❌ 联合舰队编成验证 - `/api_req_hensei/combined` 是 stub，返回 `{ api_combined: 1 }` 但不存储状态
- ❌ `api_combined_flag` 在 port 响应中硬编码为 `0`
- ❌ 活动地图 HP 表处理
- ❌ 难度选择对战斗的影响
- ❌ 航空基地支援 (LBAS) - 数据结构存在但完全未使用
- ❌ `api_req_map/next` 路由 **不存在** — 出击开始后无法推进到下一格
- ❌ 战斗模拟中所有攻击者锁定同一目标（第一个存活敌人），无目标优先级
- ❌ 反潜作战 (ASW) 未实现
- ❌ 夜战未实现

---

### 三.2 Battle System 实现细节 (`battle/core.rs`)

**完整日战流程** (834 lines):

| 阶段 | 函数 | 说明 |
|------|------|------|
| 航空战 | `simulate_kouku()` | 简化：敌方受损 6，我方受损 3 |
| 开幕雷击 | `simulate_opening_torpedo()` | 仅第一个存活目标受伤 |
| 第一次炮击 | `simulate_shelling_round()` | 全体攻击者 → 第一存活防御者 |
| 第二次炮击 | 同上 | 同上 |
| 雷击 | `simulate_raigeki()` | 顺序攻击，每艘舰船攻击其第一个存活敌人 |

**阵型伤害修正** (`core.rs`):
- 阵型 ID 2: 0.8 damage
- 阵型 ID 3: 0.7 damage
- 阵型 ID 4: 0.85 damage
- 阵型 ID 5: 0.6 damage

**交战形态修正** (`engagement_for_cell()`):
- T 字有利 (T-Advantage): 1.2 damage
- 遭遇战 (Head-On): 0.8 damage
- T 字不利 (T-Disadvantage): 0.6 damage

**伤害上限**:
- 炮击战: 220
- 雷击战: 180

**制空权** (`api_disp_seiku`):
```
api_disp_seiku = 1 (航空优势) if friend_planes >= enemy_planes
api_disp_seiku = 2 (航空劣势) if friend_planes < enemy_planes
```
- 友军损失: `min(4, friend_planes)`
- 敌军损失: 所有敌机

**MVP 计算**: 伤害量最大的舰船

**胜败判定**:
- S: 击沉全部
- A: 伤害 ≥ 70%
- B: 我军伤害 > 敌军伤害
- C: 未全灭
- D/E: 失败

### 三.3 Sortie System 存根细节 (`sortie.rs`)

```rust
// ActiveSortieState (hardcoded)
cell_id: 2           // 所有地图都是第 2 格
boss_cell_id: 2       // 所有地图 boss 都在第 2 格

// build_sortie_enemy_ships() — STUB
enemy_ship_id = 412   // 所有敌人都是夕暮 (Akatsuki)

// enemy_formation_for_cell()
formation_id = cell_id.rem_euclid(5)

// engagement_for_cell()
engagement_id = (map_id + cell_id).rem_euclid(4)
```

### 三.4 Ship Routing Mechanics (未使用)

**速度** (`api_soku`):
- 0 = base
- 5 = slow (低速)
- 10 = fast (高速)
- 15 = fast+ (高速+)
- 20 = max (最速)

**索敌值** (`api_sakuteki`): 每艘舰船的索敌值，含装备加成和等级缩放

这两个字段在 `Codex` 和数据库中已实现，但**完全未用于路径决定**。

### 三.5 Air Base (LBAS) — 未实现

**数据已存在但未使用**:
- `crates/emukc_model/src/profile/airbase.rs` — `AirbaseAction` (Idle/Attack/Defense/Evasion/Resort)
- `crates/emukc_db/src/entity/profile/airbase/` — Airbase 和 Plane 数据库实体

**LBAS 战斗顺序**:
1. Jet Assault (喷式)
2. LBAS Air Combat (基地航空战)
3. Carrier Air Battle (航母航空战)
4. Fleet Support (支援)

**缺失 endpoint**:
- `api_req_map/start_air_base`
- `api_req_map/air_raid`
- `api_get_member/base_air_corps`
- `api_req_air_corps/set_action`

### 三.6 Combined Fleet — Stub

```rust
// combined.rs (stub)
pub async fn handler(...) -> impl IntoResponse {
    // TODO: implement this
    (StatusCode::OK, Json(json!({ "api_combined": 1 })))
}
```

- 接受 `api_combined_type` (0=解散, 1=CTF, 2=STF, 3=TCF)
- 返回 `{ api_combined: 1 }` 但**不存储任何状态**
- `port.rs` 中 `api_combined_flag` 硬编码为 `0`
- 数据库无 `combined_type` 列

---

## 四、实现建议

### 4.1 数据获取

1. **从 poi/KC3Kai 源码** 解析地图数据
2. **从 kcwiki** 获取敌人舰队组成 wiki 表格
3. **从游戏客户端** 解析 `kcs2/resources/map/*_info.json`

### 4.2 数据存储

建议在 `Codex` 中新增:
```rust
pub struct CodexMaps {
    pub map_areas: HashMap<i64, MapArea>,
    pub map_infos: HashMap<i64, MapInfo>,
    pub map_cells: HashMap<i64, Vec<MapCell>>,  // map_id -> cells
    pub cell_routes: HashMap<(i64, i64), CellRoute>, // (map_id, cell_id) -> routes
    pub enemy_compositions: HashMap<(i64, i64), EnemyComp>, // (map_id, cell_id) -> comps
}
```

### 4.3 系统实现优先级

1. **Phase 1**: 解析 `spots` 数组 + 实现 `api_req_map/next`
   - 定义 `MapCell` / `CellSpot` 结构体
   - 实现 `spots: Vec<serde_json::Value>` → `Vec<MapCell>` 反序列化
   - 新建 `api_req_map/next.rs` handler — 推进 cell_id，存储 route 结果
   - 参考: `map.rs:201-309` 的 `MapInfoJson` 需要扩展

2. **Phase 2**: 路径决定逻辑
   - 基于 `api_soku` (速度) 决定是否能进入特定路线
   - 基于 `api_sakuteki` (索敌值) 决定节点分支
   - 特定舰船类型限制 (重巡无法进入某些点, 练巡需要特定速度等)
   - 参考: `codex/ship.rs` LOS 计算已存在

3. **Phase 3**: 敌人舰队组成
   - 替换 `build_sortie_enemy_ships()` stub — 不再使用 ship ID 412
   - 每格敌人数据需要从外部数据源导入 (kcwiki / poi 数据)
   - 参考: `sortie.rs:395-427` 当前简化实现

4. **Phase 4**: 联合舰队完整实现
   - 存储 `combined_type` 状态
   - 验证 CTF/STF/TCF 组成限制
   - 修改 port 响应 `api_combined_flag`
   - 实现联合舰队专属战斗顺序 (护卫舰队先攻等)
   - 参考: `combined.rs:23` 的 `// TODO: implement this`

5. **Phase 5**: LBAS (航空基地支援)
   - 实现 `AirbaseOps` trait
   - 实现 `api_req_air_corps/set_action`
   - 实现 LBAS 战斗阶段 (Jet → LBAS Air → Carrier Air → Support)
   - 参考: `profile/airbase.rs` 数据结构已存在

6. **Phase 6**: 活动地图 HP / 难度系统
   - 解析 `api_max_maphp` / `api_now_maphp`
   - 实现难度选择对敌人属性的影响
   - 实现多 gauge 表 (多重 HP 表)

7. **Phase 7**: 战斗模拟改进
   - 目标优先级系统 (不是所有攻击者都打同一个)
   - 反潜作战 (ASW) 对潜艇伤害
   - 夜战 (Night Battle) 阶段

---

## 五、关键文件参考

| 文件 | Lines | 说明 | 状态 |
|------|-------|------|------|
| `crates/emukc_model/src/kc2/start2.rs` | — | API 类型定义 (MapArea, MapInfo, EventMap) | ✅ |
| `crates/emukc_model/src/kc2/api/mod.rs` | — | KcApiMapInfo, KcApiEventmap | ✅ |
| `crates/emukc_db/src/entity/profile/map_record.rs` | — | 数据库实体 | ✅ |
| `crates/emukc_gameplay/src/game/sortie.rs` | 648 | 出击逻辑 (hardcoded cell_id=2) | ⚠️ Stub |
| `crates/emukc_gameplay/src/game/battle/core.rs` | 834 | 战斗核心 — 日战完整模拟 | ✅ 可用 |
| `crates/emukc_gameplay/src/game/battle/sortie.rs` | 127 | SortieBattleSession 存储 | ✅ |
| `crates/emukc_gameplay/src/game/battle/practice.rs` | 367 | 练习战斗 — 完整 including rival system | ✅ |
| `crates/emukc_gameplay/src/game/battle/mod.rs` | 3 | 模块导出 | ✅ |
| `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/map.rs` | ~427 | 地图资源解析 (spots 未反序列化) | ⚠️ Partial |
| `crates/emukc_model/src/profile/airbase.rs` | — | Airbase 模型 (未使用) | ❌ Unused |
| `crates/emukc_model/src/codex/ship.rs` | — | LOS 计算, 装备加成 | ✅ |
| `crates/emukc_db/src/entity/profile/airbase/base.rs` | — | Airbase 数据库实体 | ❌ Unused |
| `crates/emukc_db/src/entity/profile/airbase/plane.rs` | — | Plane 数据库实体 | ❌ Unused |
| `crates/emukc_model/src/thirdparty/quest/composition.rs` | — | Quest 条件匹配 (speed stub) | ⚠️ Stub |
| `src/bin/net/router/kcsapi/api_req_map/mod.rs` | 7 | 地图路由 (仅 `/start`) | ⚠️ 缺少 `/next` |
| `src/bin/net/router/kcsapi/api_req_map/start.rs` | 39 | 出击开始 handler | ✅ |
| `src/bin/net/router/kcsapi/api_req_hensei/combined.rs` | 33 | 联合舰队 handler (stub) | ❌ Stub |
| `src/bin/net/router/kcsapi/api_req_sortie/mod.rs` | ~10 | Sortie 路由 (battle, battleresult) | ✅ |
| `src/bin/net/router/kcsapi/api_req_sortie/battle.rs` | 39 | 战斗 handler | ✅ |
| `src/bin/net/router/kcsapi/api_port/port.rs` | — | Port 响应 (combined_flag=0) | ⚠️ |
| `src/bin/state/mod.rs` | 80 | App State with `Extension<StateArc>` | ✅ |
| `docs/apilist.txt` | — | 日语 API 文档 (spots, air_base 字段) | 📖 Ref |
| `docs/battle/research.md` | — | 14-step 战斗流程 | 📖 Ref |
