# 远征系统 (api_req_mission) 实现方案

## 背景与目标

远征系统是Kancolle核心游戏循环的重要组成部分，允许玩家派遣舰队执行远征任务以获取资源、经验和道具。当前EmuKC已实现远征列表查询 (`api_get_member/mission`)，但缺失三个关键API端点：

1. `api_req_mission/start` - 开始远征
2. `api_req_mission/result` - 获取远征结果
3. `api_req_mission/return_instruction` - 召回/中止远征

## 现有基础分析

### 数据模型（已存在）

**舰队远征状态** (`crates/emukc_db/src/entity/profile/fleet.rs`):
```rust
pub struct Model {
    pub mission_status: MissionStatus,  // Idle, InMission, Returning, ForceReturning
    pub mission_id: i64,                // 远征ID
    pub return_time: Option<DateTime<Utc>>, // 返回时间
}
```

**远征配置** (`crates/emukc_model/src/kc2/start2.rs` - `ApiMstMission`):
```rust
pub struct ApiMstMission {
    pub api_id: i64,                    // 远征ID
    pub api_time: i64,                  // 远征时间（分钟）
    pub api_use_fuel: f64,              // 燃料消耗比例
    pub api_use_bull: f64,              // 弹药消耗比例
    pub api_win_mat_level: [i64; 4],    // 资源奖励等级 [fuel, ammo, steel, bauxite]
    pub api_win_item1: [i64; 2],        // 奖励物品1 [id, count]
    pub api_win_item2: [i64; 2],        // 奖励物品2 [id, count]
    pub api_sample_fleet: [i64; 6],     // 示例舰队配置
}
```

**远征完成记录** (`crates/emukc_db/src/entity/profile/expedition.rs`):
```rust
pub struct Model {
    pub mission_id: i64,                // 远征ID
    pub state: Status,                  // NotStarted, Unfinished, Completed
    pub last_completed_at: Option<DateTime<Utc>>,
}
```

### 参考实现模式

从工厂建造 (`kdock`) 和入渠修复 (`ndock`) 系统中可以借鉴以下模式：

1. **时间计算**: `complete_time = Utc::now() + Duration::minutes(api_time)`
2. **完成检查**: 在查询时自动检查当前时间是否超过 `return_time`
3. **资源扣除**: 使用 `deduct_material_impl()` 统一扣除资源
4. **状态机**: `Idle -> InMission -> Returning -> Idle`

---

## 实现方案

### Phase 1: Gameplay 层实现

#### 1.1 扩展 `ExpeditionOps` trait

**文件**: `crates/emukc_gameplay/src/game/expedition.rs`

```rust
#[async_trait]
pub trait ExpeditionOps {
    // 现有方法
    async fn get_expeditions(&self, profile_id: i64) -> Result<(Vec<expedition::Model>, Option<i64>), GameplayError>;

    // 新增方法
    /// 开始远征
    async fn start_expedition(
        &self,
        profile_id: i64,
        fleet_id: i64,
        mission_id: i64,
    ) -> Result<Fleet, GameplayError>;

    /// 获取远征结果
    async fn get_expedition_result(
        &self,
        profile_id: i64,
        fleet_id: i64,
    ) -> Result<ExpeditionResult, GameplayError>;

    /// 召回远征（强制返回）
    async fn return_expedition(
        &self,
        profile_id: i64,
        fleet_id: i64,
    ) -> Result<Fleet, GameplayError>;
}
```

#### 1.2 远征结果数据结构

**文件**: `crates/emukc_model/src/kc2/api/mod.rs` (追加)

```rust
/// 远征结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMissionResult {
    /// 远征是否成功
    pub api_clear_result: i64,          // 0=失败, 1=成功, 2=大成功
    /// 提督经验值获取
    pub api_get_exp: i64,
    /// 舰队经验值获取（数组，对应每个舰船）
    pub api_get_ship_exp: Vec<i64>,
    /// 远征日志详情
    pub api_detail: String,
    /// 资源获取 [[fuel, count], [ammo, count], [steel, count], [bauxite, count]]
    pub api_get_material: Vec<Vec<i64>>,
    /// 物品获取
    pub api_useitem_flag: [i64; 2],     // [物品1是否获得, 物品2是否获得]
    /// 物品获取详情
    pub api_get_item1: Option<KcApiMissionResultItem>,
    pub api_get_item2: Option<KcApiMissionResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMissionResultItem {
    pub api_useitem_id: i64,
    pub api_useitem_count: i64,
}
```

#### 1.3 开始远征实现细节

**函数**: `start_expedition_impl()`

1. **前置条件检查**:
   - 舰队必须存在且状态为 `Idle`
   - 第二舰队及以上才能远征 (fleet_id >= 2)
   - 远征ID必须有效（从Codex验证）
   - 舰队不能为单舰

2. **资源消耗计算与扣除**:
   ```rust
   let mission = codex.manifest.find_mission(mission_id)?;
   let fuel_cost = (total_fuel_capacity * mission.api_use_fuel) as i64;
   let ammo_cost = (total_ammo_capacity * mission.api_use_bull) as i64;

   deduct_material_impl(&tx, profile_id, &[
       (MaterialCategory::Fuel, fuel_cost),
       (MaterialCategory::Ammo, ammo_cost),
   ]).await?;
   ```

3. **更新舰队状态**:
   ```rust
   fleet.mission_status = MissionStatus::InMission;
   fleet.mission_id = mission_id;
   fleet.return_time = Utc::now() + Duration::minutes(mission.api_time);
   ```

4. **触发任务更新**:
   - 调用 `update_quest_progress_for_action()` 更新远征相关任务进度

#### 1.4 远征结果实现细节

**函数**: `get_expedition_result_impl()`

1. **完成检查**:
   ```rust
   if fleet.mission_status != MissionStatus::InMission ||
      fleet.return_time > Some(Utc::now()) {
       return Err(GameplayError::InvalidState("expedition not complete".to_string()));
   }
   ```

2. **成功判定**:
   - 基础成功率：满足编成条件时约 50%
   - 旗舰等级加成：每级约 +0.1%，上限约 6%
   - 大成功率计算：≈ 16.67% × 闪舰数量（morale >= 50）
     - 4闪 ≈ 66.67%，5闪 ≈ 83.33%，6闪 ≈ 100%
   - 远征 21, 24, 37, 38, 40 为运输桶远征，大成功率计算不同

3. **奖励计算**:
   - **普通成功**: 100% 基础资源 (`api_win_mat_level`)
   - **大成功**: 150% 基础资源 + 额外道具
   - 大发动艇加成: 每艘 +5%，最多 4 艘 → +20%
   - 最终倍率 = 大成功倍率(1.5) × 大发动艇加成(1.0~1.2)
   - 物品奖励: `api_win_item1/2` 在大成功时概率获得
   - 经验值: 提督经验 (`api_get_exp`) + 舰船经验 (`api_get_ship_exp`)

4. **发放奖励**:
   - 使用 `add_material_impl()` 增加资源
   - 使用 `add_use_item_impl()` 增加物品
   - 更新远征完成记录 `expedition::Entity`

5. **重置舰队状态**:
   ```rust
   fleet.mission_status = MissionStatus::Returning;  // 先设为返回中
   ```

6. **任务进度更新**:
   - 更新远征任务完成计数

#### 1.5 召回远征实现细节

**函数**: `return_expedition_impl()`

1. **验证状态**:
   ```rust
   if fleet.mission_status != MissionStatus::InMission {
       return Err(GameplayError::InvalidState("fleet not in mission".to_string()));
   }
   ```

2. **立即返回**:
   ```rust
   fleet.mission_status = MissionStatus::ForceReturning;
   fleet.return_time = Some(Utc::now());  // 立即返回
   ```

3. **无奖励**: 强制召回不发放任何奖励

### Phase 2: API Handler 实现

#### 2.1 模块结构

创建目录: `src/bin/net/router/kcsapi/api_req_mission/`

```
api_req_mission/
├── mod.rs       # 路由注册
├── start.rs     # 开始远征
├── result.rs    # 远征结果
└── return_instruction.rs  # 召回远征
```

#### 2.2 路由注册

**文件**: `src/bin/net/router/kcsapi/api_req_mission/mod.rs`

```rust
use axum::{Router, routing::post};

mod result;
mod return_instruction;
mod start;

pub(super) fn router() -> Router {
    Router::new()
        .route("/start", post(start::handler))
        .route("/result", post(result::handler))
        .route("/return_instruction", post(return_instruction::handler))
}
```

#### 2.3 开始远征 Handler

**文件**: `src/bin/net/router/kcsapi/api_req_mission/start.rs`

```rust
pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<RequestBody>,
) -> KcApiResult {
    let fleet_id = params.api_deck_id;
    let mission_id = params.api_mission_id;

    let fleet = state.start_expedition(session.profile.id, fleet_id, mission_id).await?;

    Ok(KcApiResponse::success(&fleet))
}
```

**请求体**:
```rust
struct RequestBody {
    api_deck_id: i64,      // 舰队ID
    api_mission_id: i64,   // 远征ID
}
```

#### 2.4 远征结果 Handler

**文件**: `src/bin/net/router/kcsapi/api_req_mission/result.rs`

```rust
pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<RequestBody>,
) -> KcApiResult {
    let fleet_id = params.api_deck_id;

    let result = state.get_expedition_result(session.profile.id, fleet_id).await?;

    Ok(KcApiResponse::success(&result))
}
```

**请求体**:
```rust
struct RequestBody {
    api_deck_id: i64,      // 舰队ID
}
```

#### 2.5 召回远征 Handler

**文件**: `src/bin/net/router/kcsapi/api_req_mission/return_instruction.rs`

```rust
pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<RequestBody>,
) -> KcApiResult {
    let fleet_id = params.api_deck_id;

    let fleet = state.return_expedition(session.profile.id, fleet_id).await?;

    Ok(KcApiResponse::success(&fleet))
}
```

### Phase 3: 路由集成

#### 3.1 注册到主路由

**文件**: `src/bin/net/router/kcsapi/mod.rs`

```rust
mod api_req_furniture;
mod api_req_hensei;
mod api_req_hokyu;
mod api_req_init;
mod api_req_kaisou;
mod api_req_kousyou;
mod api_req_member;
mod api_req_mission;  // 新增
mod api_req_nyukyo;
mod api_req_quest;
mod api_req_ranking;

pub(super) fn router() -> Router {
    Router::new()
        // ... 现有路由
        .merge(Router::new().nest("/api_req_mission", api_req_mission::router()))  // 新增
        // ... 现有路由
}
```

### Phase 4: 舰队远征完成检查

修改 `get_fleet_impl` 和 `get_fleets_impl` 在查询舰队时自动检查远征是否完成：

**文件**: `crates/emukc_gameplay/src/game/fleet.rs`

```rust
pub(crate) async fn get_fleet_impl<C>(
    c: &C,
    profile_id: i64,
    index: i64,
) -> Result<Fleet, GameplayError>
where
    C: ConnectionTrait,
{
    let fleet = find_fleet(c, profile_id, index).await?;

    // 检查远征是否完成
    if fleet.mission_status == MissionStatus::InMission {
        if let Some(return_time) = fleet.return_time {
            if return_time <= Utc::now() {
                // 远征已完成，更新状态为Returning
                let mut am: fleet::ActiveModel = fleet.clone().into();
                am.mission_status = ActiveValue::Set(MissionStatus::Returning);
                am.update(c).await?;
            }
        }
    }

    Ok(fleet.into())
}
```

---

## 关键文件清单

| 类型 | 文件路径 |
|------|----------|
| **Gameplay Trait** | `crates/emukc_gameplay/src/game/expedition.rs` |
| **API Model** | `crates/emukc_model/src/kc2/api/mod.rs` (新增远征结果类型) |
| **远征条件模型** | `crates/emukc_model/src/expedition/condition.rs` (新建) |
| **远征数据Codex** | `crates/emukc_model/src/codex/expedition.rs` (新建) |
| **API Handler 模块** | `src/bin/net/router/kcsapi/api_req_mission/mod.rs` (新建) |
| **开始远征 Handler** | `src/bin/net/router/kcsapi/api_req_mission/start.rs` (新建) |
| **远征结果 Handler** | `src/bin/net/router/kcsapi/api_req_mission/result.rs` (新建) |
| **召回远征 Handler** | `src/bin/net/router/kcsapi/api_req_mission/return_instruction.rs` (新建) |
| **路由注册** | `src/bin/net/router/kcsapi/mod.rs` |
| **舰队完成检查** | `crates/emukc_gameplay/src/game/fleet.rs` |
| **远征数据文件** | `.data/expedition.json` (下载的 KCanotify 数据) |

---

## 数据源导入步骤

### 1. 下载远征数据

```bash
# 下载 KCanotify 远征数据
curl -L -o .data/expedition.json https://antest1.github.io/kcanotify-gamedata/files/expedition.json

# 验证数据完整性
head -100 .data/expedition.json
```

### 2. 创建远征条件模型

**文件**: `crates/emukc_model/src/expedition/condition.rs`

```rust
/// 远征编成条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpeditionCondition {
    pub api_id: i64,
    pub name: ExpeditionName,
    pub time_minutes: i64,
    pub resource_reward: [i64; 4],  // [fuel, ammo, steel, bauxite]
    pub flagship_lv: i64,
    pub fleet_lv: Option<i64>,
    pub ship_count: i64,
    pub flagship_type: Option<i64>,  // 旗舰舰种要求
    pub composition: Vec<ShipTypeRequirement>,
    pub total_firepower: Option<i64>,
    pub total_asw: Option<i64>,
    pub total_los: Option<i64>,
    pub drum_ship: Option<i64>,      // 携带桶的舰船数量
    pub drum_num: Option<i64>,       // 桶的总数量
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipTypeRequirement {
    pub ship_types: Vec<i64>,  // 允许的舰种ID列表
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpeditionName {
    pub jp: String,
    pub scn: String,
    pub en: String,
}
```

### 3. 解析 KCanotify 数据格式

```rust
// 解析 total-cond 字段的示例
fn parse_composition_condition(cond: &str) -> Vec<ShipTypeRequirement> {
    // "3-1|1,2-2" -> 
    //   [ShipTypeRequirement { ship_types: [3], count: 1 }]
    //   OR
    //   [ShipTypeRequirement { ship_types: [1, 2], count: 2 }]
    
    let parts: Vec<&str> = cond.split('|').collect();
    // 解析逻辑...
}
```

### 4. 集成到 Codex

在 `Codex` 结构中添加远征条件映射：

```rust
pub struct Codex {
    // 现有字段...
    pub expedition_conditions: HashMap<i64, ExpeditionCondition>,
}
```

---

## 验证步骤

1. **编译检查**:
   ```bash
   cargo check
   cargo clippy --workspace
   ```

2. **运行测试**:
   ```bash
   cargo test --test gameplay_tests
   ```

3. **手动验证**（需要bootstrap数据）:
   ```bash
   cargo run -- serve
   ```

   然后使用API客户端测试：
   - 调用 `api_get_member/deck` 获取舰队信息
   - 调用 `api_req_mission/start` 开始远征
   - 等待（或使用测试模式缩短时间）
   - 调用 `api_req_mission/result` 获取结果
   - 验证资源和物品是否正确增加

4. **任务系统验证**:
   - 完成远征后检查任务进度是否正确更新
   - 远征任务是否被标记为完成

---

## 远征条件验证说明

### 现状分析

官方API (`start2.json` - `ApiMstMission`) **不包含**详细的远征编成条件数据：
- `api_sample_fleet`: 仅用于显示编成图标（舰种ID数组）
- `api_deck_num`: 所需舰队舰船数量
- `api_difficulty`: 远征难度等级

远征编成条件通常包括：
- 旗舰类型要求（如：轻巡旗舰）
- 特定舰种数量（如：驱逐舰2艘以上）
- 舰队总等级
- 索敌值要求
- 特定舰船要求

### 数据源发现

经过全面搜索和时效性验证，发现以下可用的远征数据源：

#### 1. KCanotify 远征数据 ⭐⭐⭐⭐⭐ **推荐使用**
- **数据URL**: https://antest1.github.io/kcanotify-gamedata/files/expedition.json
- **GitHub仓库**: https://github.com/antest1/kcanotify-gamedata
- **时效性**:
  - 仓库活跃度: ✅ **非常活跃** - 最后提交 2026年3月16日
  - 数据更新时间: ⚠️ **2025年6月7日** (9个月前)
  - 数据稳定性: 远征机制极少变更，数据仍然有效
- **数据完整性**:
  - 记录数量: **65条完整远征数据**
  - 覆盖范围: Area 1-5, 7 全部常规远征 (1-46) + A/B/D/E/S系列
  - 文件大小: ~1,287行JSON
- **包含字段**:
  - 基础信息: `no`, `code`, `area`, `name` (多语言: 日/韩/英/简中/繁中)
  - 时间: `time` (分钟)
  - 资源奖励: `resource` [燃料, 弹药, 钢材, 铝土]
  - 物品奖励: `reward` [[物品ID, 数量], ...]
  - 经验值: `exp` [提督经验, 舰队经验]
  - **编成条件**:
    - `total-num`: 舰队舰船数量要求
    - `flag-lv`: 旗舰等级要求
    - `total-lv`: 舰队总等级要求
    - `total-cond`: 编成条件表达式（如 `"3-1|1,2-2"` 表示轻巡1艘或驱逐2艘+驱逐/轻巡1艘）
    - `flag-cond`: 旗舰类型要求（如 `"3"` 表示轻巡旗舰）
    - `total-firepower`/`total-fp`: 舰队火力要求
    - `total-asw`: 反潜值要求
    - `total-los`: 索敌值要求
    - `drum-ship`: 携带桶的舰船数量
    - `drum-num`: 桶的总数量

#### 2. POI Plugin Expedition ⭐⭐⭐⭐ ⚠️ **数据过时**
- **URL**: https://github.com/poooi/plugin-expedition
- **数据文件**: `assets/expedition.json`
- **时效性**:
  - 最后更新: ❌ **2018年12月10日** (6+年前)
  - 维护状态: ❌ **停止维护** - 5个未处理issues
- **数据覆盖**: 仅46条基础远征，缺少A/B/D/E系列
- **特点**: `required_shiptypes` 字段直接定义舰种要求
- **结论**: 数据严重过时，**不建议使用**

#### 3. ElectronicObserver (74式电子观测仪) ⭐⭐⭐⭐⭐ **验证逻辑参考**
- **URL**: https://github.com/andanteyk/ElectronicObserver
- **验证文件**: `ElectronicObserver/Data/MissionClearCondition.cs`
- **时效性**:
  - 仓库状态: ⚠️ **2023年10月后未更新** (2.5年前)
  - 远征代码最后更新: **2021年4月1日** (远征115, 133支持)
- **验证覆盖**: 42条远征 (1-40, 100-115, 131-133, 141-142)
- **核心价值**: 提供完整的编成条件验证算法实现，可作为Rust实现的参考

### 数据源时效性对比

| 数据源 | 时效性 | 完整性 | 推荐用途 |
|--------|--------|--------|----------|
| **KCanotify** | ⚠️ 9个月 | ✅ 65条完整 | 🥇 **主数据源** |
| **POI Plugin** | ❌ 6年 | ⚠️ 46条过时 | 🚫 不使用 |
| **ElectronicObserver** | ⚠️ 2.5年 | ⚠️ 42条 | 🥈 **验证逻辑参考** |

### 推荐方案

**数据层**: 使用 KCanotify `expedition.json`
```bash
# 下载远征数据
curl -L -o .data/expedition.json \
  https://antest1.github.io/kcanotify-gamedata/files/expedition.json
```

**验证层**: 参考 ElectronicObserver 实现 `MissionClearCondition`

### 编成条件格式解析

KCanotify 的 `total-cond` 字段使用以下格式：
```
格式: "舰种ID-数量|舰种ID,舰种ID-数量/舰种ID-数量|..."

示例: "3-1|1,2-2" 表示:
  - 1艘轻巡(3) 或
  - 2艘驱逐(2) + 1艘任意(1)

舰种ID映射:
1 = 任意舰
2 = 驱逐
3 = 轻巡
5 = 重巡
7,11,16,18 = 航母系
13,14 = 潜水艇
16 = 水母
20 = 潜水母舰
21 = 练习巡洋舰
27 = 轻空母
```

### 远征条件验证逻辑参考 (ElectronicObserver)

参考 [MissionClearCondition.cs](https://github.com/andanteyk/ElectronicObserver/blob/469015b46e061978f857abb57140b4b23459feda/ElectronicObserver/Data/MissionClearCondition.cs) 的实现：

**验证器API设计（C#）**:
```csharp
public static MissionClearConditionResult Check(int missionID, FleetData fleet)
{
    return new MissionClearConditionResult(fleet)
        .CheckFlagshipLevel(50)           // 旗舰等级 ≥ 50
        .CheckLevelSum(200)               // 舰队总等级 ≥ 200
        .CheckShipCount(5)                // 5艘舰船
        .CheckSmallShipCount(4)           // 驱逐+海防 ≥ 4艘
        .CheckFirepower(360)              // 总火力 ≥ 360
        .CheckASW(180)                    // 总对潜 ≥ 180
        .CheckLOS(140);                   // 总索敌 ≥ 140
}
```

**可用验证方法清单**:

| 方法 | 用途 |
|------|------|
| `CheckFlagshipLevel(int)` | 最低旗舰等级 |
| `CheckLevelSum(int)` | 舰队总等级和 |
| `CheckShipCount(int)` | 最低舰船数量 |
| `CheckShipCountByType(ShipTypes, int)` | 特定舰种数量 |
| `CheckSmallShipCount(int)` | 驱逐+海防数量 |
| `CheckAircraftCarrierCount(int, bool)` | 航母数量（含/不含水母） |
| `CheckFlagshipType(ShipTypes)` | 旗舰类型要求 |
| `CheckEscortFleet()` | 護衛隊编成验证 |
| `CheckEscortFleetDD3/4()` | 護衛隊+3/4驱逐要求 |
| `CheckFirepower/AA/LOS/ASW(int)` | 属性总和验证 |
| `CheckEquipmentCount(EquipmentTypes, int)` | 装备总数 |
| `CheckEquippedShipCount(EquipmentTypes, int)` | 携带装备的舰船数 |
| `OrCondition(params Action[])` | 条件分支（或逻辑） |

**護衛隊编成验证逻辑**:
```csharp
// 以下4种编成满足護衛隊要求:
// 1. 轻巡1艘 + (驱逐或海防)2艘
// 2. 護衛空母1艘 + (驱逐或海防)2艘  
// 3. 驱逐1艘 + 海防3艘
// 4. 练习巡洋舰1艘 + 海防2艘
```

**参考实现建议（Rust）**:
```rust
pub struct ExpeditionValidator<'a> {
    fleet: &'a Fleet,
    errors: Vec<String>,
}

impl<'a> ExpeditionValidator<'a> {
    pub fn new(fleet: &'a Fleet) -> Self { ... }
    
    pub fn check_flagship_level(mut self, min: i64) -> Self {
        if self.fleet.flagship.level < min {
            self.errors.push(format!("旗舰等级不足: {} < {}", ...));
        }
        self
    }
    
    pub fn check_composition(mut self, cond: &str) -> Self {
        // 解析 KCanotify total-cond 格式
        // "3-1|1,2-2" -> 轻巡1艘 或 (驱逐2艘 + 任意1艘)
        self
    }
    
    pub fn validate(self) -> Result<(), Vec<String>> {
        if self.errors.is_empty() { Ok(()) } else { Err(self.errors) }
    }
}
```

### 成功率/大成功计算

根据 KCWiki 和社区研究:

**基础成功率**: 约 50% (满足编成条件时)
**旗舰等级加成**: 每级约 +0.1%, 上限约 6%

**大成功率**: ≈ 16.67% × 闪舰数量 (morale >= 50)
- 4闪 ≈ 66.67%
- 5闪 ≈ 83.33%
- 6闪 ≈ 100% (理论上)

**运输桶远征特殊计算** (远征 21, 24, 37, 38, 40):
- 需要携带运输桶 (ドラム缶)
- 大成功率计算方式不同，通常与运输桶数量和闪舰都有关

**大成功奖励**:
- 普通成功: 100% 基础资源
- 大成功: 150% 基础资源 + 额外道具
- 大发动艇加成: 每个 +5%，最多 4 个 → +20%
- 最终倍率 = 大成功倍率(1.5) × 大发动艇加成(1.0~1.2)

### 建议实现

**推荐方案**: KCanotify 数据源 + ElectronicObserver 验证逻辑参考

#### 1. 数据导入

```bash
# 下载 KCanotify 远征数据
curl -L -o .data/expedition.json \
  https://antest1.github.io/kcanotify-gamedata/files/expedition.json

# 验证数据完整性 (应包含65条远征)
jq '. | length' .data/expedition.json
```

#### 2. 解析 KCanotify 数据

实现 `expedition.json` 解析器，将JSON数据转换为Codex内部结构：

```rust
// 解析示例: expedition.json -> ExpeditionCondition
// 远征4 (対潜警戒任務): "total-cond": "3-1|1,2-2"
// 表示: 轻巡1艘 或 (驱逐2艘 + 任意1艘)

let condition = parse_total_cond("3-1|1,2-2");
// 解析为: OR条件 [
//   AND条件 [舰种3 × 1],
//   AND条件 [舰种1或2 × 2, 舰种1或2 × 1]
// ]
```

#### 3. 条件验证实现

参考 ElectronicObserver 的 `MissionClearCondition.cs` 设计验证器：

```rust
// 验证顺序
pub async fn validate_expedition_conditions(
    codex: &Codex,
    fleet: &Fleet,
    mission_id: i64,
) -> Result<(), GameplayError> {
    // 1. 基础状态检查
    if fleet.mission_status != MissionStatus::Idle {
        return Err(GameplayError::InvalidState("fleet not idle".into()));
    }
    if fleet.ships.len() < 2 {
        return Err(GameplayError::InvalidInput("fleet must have at least 2 ships".into()));
    }
    
    // 2. 获取远征条件
    let condition = codex.expedition_conditions
        .get(&mission_id)
        .ok_or_else(|| GameplayError::NotFound(format!("mission {} not found", mission_id)))?;
    
    // 3. 编成条件验证
    ExpeditionValidator::new(fleet)
        .check_flagship_level(condition.flag_lv)?
        .check_fleet_level_sum(condition.total_lv)?
        .check_ship_count(condition.total_num)?
        .check_composition(&condition.total_cond)?  // 解析 KCanotify 格式
        .check_flagship_type(condition.flag_cond)?
        .check_total_firepower(condition.total_firepower)?
        .check_total_asw(condition.total_asw)?
        .check_total_los(condition.total_los)?
        .check_drum_requirements(condition.drum_ship, condition.drum_num)?
        .validate()?;
    
    // 4. 资源检查
    validate_resource_availability(codex, profile, mission_id).await?;
    
    Ok(())
}
```

#### 4. 成功率计算

```rust
// 基础成功率: 50% (满足编成条件时)
// 旗舰等级加成: 每级 +0.1%, 上限约 6%
// 大成功率: 16.67% × 闪舰数量 (morale >= 50)

fn calculate_success_rate(
    fleet: &Fleet,
    is_tokyo_express: bool,  // 远征 21, 24, 37, 38, 40
) -> (f64, f64) {  // (成功率, 大成功率)
    let base_rate = 0.50;
    let flagship_bonus = (fleet.flagship.level as f64 * 0.001).min(0.06);
    let success_rate = base_rate + flagship_bonus;
    
    let sparkled_ships = fleet.ships.iter()
        .filter(|s| s.morale >= 50)
        .count();
    let great_success_rate = if is_tokyo_express {
        // 运输桶远征特殊计算
        calculate_tokyo_great_success_rate(fleet)
    } else {
        (sparkled_ships as f64 * 0.1667).min(1.0)
    };
    
    (success_rate, great_success_rate)
}
```

#### 5. 风险缓解措施

| 风险 | 缓解措施 |
|------|----------|
| KCanotify 数据9个月未更新 | 实现数据版本检查，定期对比游戏API数据 |
| 数据与游戏实际不符 | 抽样验证 `api_start2` 中的 `ApiMstMission` |
| 数据源失效 | 本地缓存并备份 `.data/expedition.json` |
| 编成条件解析错误 | 为每个远征编写单元测试，验证条件解析正确性 |
