# 线上账号导入：字段对照

日期：2026-10-27。这份调研给"把线上账号导入 EmuKC"的计划提供输入。对照对象是 `emukc_db::entity::profile` 下的全部实体。数据来源是两份快照，都在 gitignored 目录下，由 `scripts/fetch_live_api.py` 抓取：

- `z/snapshot/2026-10-16/`：对照工作就是在这份上做的。
- `z/snapshot/2026-10-27/`：同一账号，同一时刻抓的 48 个文件，比 10-16 多了图鉴、装备预设、开发预设和付费道具。48 个文件都能解析为 JSON，`api_result` 全是 1。**导入的测试夹具以这份为准**，因为所有端点是在同一时刻抓的；用本地类型反序列化能不能通过还没试，见下文"解析用的类型"。两份之间舰娘数（340）、`api_start2` 规模，以及任务状态（全部未接）都一样。

文档里只写结构和计数，不写昵称、提督 ID、签名这类个人信息。

## 快照概况

- 10-16 的端点包括 `api_port/port`、`require_info`、`slot_item`、`deck`、`material`、`ndock`、`kdock`、`mission`、`useitem`、`preset_deck`、`practice`、`mapinfo`、`questlist`（tab 0/1/2/3/4/5/9）、`remodel_slotlist` 和 `api_start2/getData`。
- 规模：舰娘 340 艘（300 种），装备 1855 件（304 种，其中 362 件带 `api_alv`），家具 409 件，使用道具 23 种，已开放海域 33 张（已通关 26 张），远征 50 个，任务 105 个，预设编成 9 个，基地航空队 3 队（12 个中队位）。
- 这份快照覆盖不到的情况：抓取时没有接任何任务（105 个任务全是 `api_state: 1`、`api_progress_flag: 0`），不在活动期（没有 `api_eventmap`，舰娘没有 `api_sally_area`），没有舰娘带 `api_sp_effect_items`，`api_plane_info` 是 `null`（没有中队在换装中）。导入代码遇到这些键时必须能处理，但这份夹具测不到它们。
- 和码典的对照：舰娘、装备、使用道具、家具、海域、远征的 ID 全部能在 `.data/codex/start2.json` 里找到。任务有 3 个不在 `quest.json` 里：915、984、1152，见下文"推断规则"。线上 `api_start2` 有 741 件装备，码典有 743 件，码典更全。

## ID 重映射与插入顺序（关键）

`ship.id` 和 `slot_item.id` 都是全局自增主键，多个 profile 共用这个 ID 空间。线上的实例 ID 不能直接写进去，即使 `From<KcApiShip> for ship::ActiveModel` 现在就是 `id: Set(api_id)`。导入时要建一张"线上 ID → 本地 ID"的映射表，按下面的顺序写入：

1. **装备**：按线上 `api_id` 升序插入，`equip_on` 先不设。
2. **舰娘**：按线上 `api_id` 升序插入。`slot_1..slot_5` 和 `slot_ex` 用装备映射换成本地 ID（`-1` 和 `0` 照原样保留）。
3. **回填装备的 `equip_on`**：根据舰娘的装备栏写入本地舰娘 ID。
4. **引用舰娘的表**：`fleet.ship_1..6`、`preset_deck.ship_1..6`、`ndock.ship_id`、`ship_sp_effect_item.ship_id`，都换成本地舰娘 ID。
5. **引用装备的表**：`plane_info.slot_id` 换成本地装备 ID。

按升序插入是为了保住"入手順"：客户端按 ID 排这个顺序，本地 ID 单调递增，相对顺序就不会变。

各列存的是实例 ID 还是主数据 ID：

| 实例 ID（要重映射） | 主数据 ID（照搬，导入前校验码典里有没有） |
| --- | --- |
| `ship.id`、`ship.slot_*`、`slot_item.id`、`slot_item.equip_on`、`fleet.ship_*`、`preset_deck.ship_*`、`ndock.ship_id`、`plane_info.slot_id`、`ship_sp_effect_item.ship_id` | `ship.mst_id`、`slot_item.mst_id`、`kdock.ship_id`（名字像实例 ID，存的其实是建造中舰娘的主数据 ID）、`use_item.mst_id`、`pay_item.mst_id`、`furniture_record.furniture_id`、`map_record.map_id`、`expedition.mission_id`、`quest_*.quest_id`、`preset_slot.mst_id_*` |

## 逐实体对照

分五类：**照搬**；**重映射**；**本地重算**（导入原值后交给本地公式重算，不直接信线上值）；**近似**（API 不给或只给一部分，取一个合理值并在导入报告里注明）；**取默认**（不导入）。

| 本地实体 | 线上来源 | 照搬 / 重映射 | 本地重算 | 近似 / 取默认 |
| --- | --- | --- | --- | --- |
| `profile` | port `api_basic`、require_info | `name←api_nickname`、`hq_level←api_level`、`hq_rank←api_rank`、`experience`、`comment`、`max_ship_capacity←api_max_chara`、`max_equipment_capacity←api_max_slotitem`、`sortie_wins/loses←api_st_win/lose`、`expeditions(_success)←api_ms_count/success`、`practice_*←api_pt_*`、`tutorial_progress`、`medals`、`large_dock_unlocked←api_large_dock`、`max_quests←api_parallel_quest_count`、`intro_completed←api_firstflag`、`extra_supply_expedition/sortie←api_extra_supply[0]/[1]` | — | `combined_type` 在 port 没有 `api_combined_flag` 时取 0；`war_result*` 取 0（戦果在排行榜接口里，抓不抓见下文）；`last_played` 取导入时间。`api_fcoin` 是使用道具 44，`api_count_deck/kdock/ndock` 由表的行数推出，都不另存 |
| `material` | `api_material` | ID 1–8 依次对应 `fuel/ammo/steel/bauxite/torch/bucket/devmat/screw` | — | `last_update_*` 取导入时间 |
| `ship` | port `api_ship`（`KcApiShip`） | 重映射 `id`、`slot_*`；照搬 `mst_id`、`sort_num`、`level`、`exp_*`、`locked`、`hp_now`、`fuel/ammo`、`condition`、`backs`、`slot_num`、`onslot_*`、`mod_*←api_kyouka[0..6]`、`sally_area` | `*_now/*_max`（火力到运）、`hp_max`、`speed`、`range`、`ndock_time/fuel/steel`、`srate`、`has_locked_euqip`，导入后交给 `recalculate_ship_status_with_model` 重算 | `married` 取 `level ≥ 100`；`onslot_plus_*` 取 `None`（线上不发） |
| `slot_item` | `slot_item` / require_info | 重映射 `id`；照搬 `mst_id`、`locked`、`level`、`aircraft_lv←api_alv`（没有该键时取 0） | `type3` 由码典 `api_type[2]` 推出，`equip_on` 回填 | `aircraft_exp` 线上不给，取该熟练度等级的经验下限 |
| `ship_sp_effect_item` | `api_ship[].api_sp_effect_items` | 重映射 `ship_id` | — | 这份快照里没有，要等有数据的样本 |
| `fleet` | `deck` / port `api_deck_port`（`KcApiDeckPort`） | `index←api_id`、`name`；`mission_status←api_mission[0]`、`mission_id←[1]`、`return_time←[2]`（毫秒）；重映射 `ship_*` | — | `launch_*`（出发时的舰数、闪光数、旗舰等级、桶）线上不给。远征中的舰队按导入时的编成计算，`launch_fully_supplied` 取 true（条件在归来时判，见 `expedition.md`） |
| `kdock` | `kdock`（`KcApiKDock`） | `index←api_id`、`status←api_state`（经 `ConstructionDockStatus`，-1 表示未开放）、`ship_id←api_created_ship_id`、`complete_time`、`fuel..devmat←api_item1..5` | `is_large` 由投入量推出（大型建造） | — |
| `ndock` | `ndock`（`KcApiNDock`） | `index`、`status`（经 `RepairDockStatus`）、重映射 `ship_id`、`complete_time`、`fuel←api_item1`、`steel←api_item3` | — | — |
| `use_item` | `useitem`（`KcApiUserItem`） | `mst_id←api_id`、`count` | — | — |
| `furniture_record` | require_info `api_furniture`（`KcApiFurniture`） | `furniture_id←api_id` | — | — |
| `furniture_config` | port `api_basic.api_furniture[6]` | 依次对应 `floor/wallpaper/window/wall_hanging/shelf/desk` | — | `season` 取默认 |
| `map_record` | `mapinfo.api_map_info`（`KcApiMapInfo`） | `map_id←api_id`、`cleared`、`defeat_count`、`gauge_index←api_gauge_num`（本地也从 1 开始数）；列表里出现即 `unlocked` | `api_required_defeat_count`、`api_gauge_type`、`api_air_base_decks` 来自码典，不导入 | `last_cleared_at` / `last_reset_at` 取导入时间（这会影响每月重置的 EO 海域）；`current_hp`、`stage_id`、`selected_rank`、`event_state` 只在活动期的 `api_eventmap` 里有 |
| `airbase` | `mapinfo.api_air_base`（`KcApiAirBase`）+ `api_air_base_expanded_info` | `area_id`、`rid`、`name`、`action←api_action_kind`、`base_range/bonus_range←api_distance.api_base/api_bonus`、`maintenance_level`（按 `area_id` 取） | — | — |
| `plane_info` | `api_air_base[].api_plane_info`（`KcApiPlaneInfo`） | `squadron_id`、`state`、重映射 `slot_id`、`count`、`max_count`、`condition←api_cond` | — | `since` 取 `None`；中队在换装中（`state: 2`）时取导入时间 |
| `expedition` | `mission.api_list_items` | `mission_id`、`state←api_state`（0/1/2 和本地 `NotStarted/Unfinished/Completed` 一致） | — | `last_completed_at` 线上不给。`state: 2` 的取导入时间，这会让月度远征延后到下个月才重置；`api_limit_time` 是月度远征的重置时间（apilist） |
| `preset_deck`、`preset_caps` | `preset_deck`（`KcApiPresetDeck`） | `index←api_preset_no`、`name`、`locked←api_lock_flag`、重映射 `ship_1..6←api_ship[0..5]`；`preset_caps.deck_limit←api_max_num` | — | `api_ship[6]`（第 7 艘，遊撃部隊用）本地没有对应列，快照里全是 -1，丢弃 |
| `preset_slot`、`preset_caps` | 10-27 `preset_slot`（`KcApiPresetSlot`） | `index←api_preset_no`、`name`、`mode←api_selected_mode`、`locked←api_lock_flag`、`ex_flag←api_slot_ex_flag`；`mst_id_n/stars_n←api_slot_item[n].api_id/api_level`（列表长度不定，3 到 5 条），`mst_id_ex/stars_ex←api_slot_item_ex`（只在有补强增设时出现）；`preset_caps.slot_limit←api_max_num` | — | 这里存的是主数据 ID，不用重映射 |
| `preset_dev_item` | 10-27 `preset_dev_items` | `preset_caps.dev_item_limit←api_max_num` | — | 这个账号没存过开发预设，响应里只有 `api_max_num`，有数据时是什么结构还没见过 |
| `pay_item` | 10-27 `payitem` | — | — | 这个账号的 `api_data` 是 `null`（没有未使用的付费道具），有数据时是什么结构还没见过 |
| `ship_record`（舰娘图鉴） | 10-27 `picture_book` `api_type=1`（592 个条目） | `sort_num←api_index_no` | — | 一个条目包含同一舰娘的多个形态：`api_table_id` 列出各形态的舰娘 ID（1 到 19 个），`api_state` 每个形态一行 `[已入手, 中破, 结婚, ?, ?]`。本地每个 `sort_num` 只有一行，`damaged`、`married` 只能取各形态的"或"。有 53 个条目的各形态状态不同，这些细节会丢掉 |
| `slotitem_record`（装备图鉴） | 10-27 `picture_book` `api_type=2`（314 个条目） | `sort_num←api_index_no` | — | `api_state` 全是 `[1,0,0,0,0]`，`api_table_id` 只有 1 个 |
| `quest_progress`、`quest_record_*` | `questlist` | 见下文"推断规则" | — | 进度只能取近似值 |
| `oss_settings` | require_info `api_oss_setting`（已有 `From<KcApiOssSetting>`） | 整块照搬 | — | — |
| `option_settings` | require_info `api_skin_id` | `skin_id` | — | 音量和语音开关取默认 |
| `game_settings` | require_info `api_position_id`、port `api_p_bgm_id` | `position_id`、`port_bgm` | — | 友军舰队请求开关取默认 |
| `ship_morale_timer` | — | — | — | 取导入时间 |
| `practice_*`、`rival*`、`incentive` | — | — | — | 不导入。对手由本地生成，`incentive` 是临时的领奖队列 |

port 里的 `api_log`、`api_dest_ship_slot`、`api_c_flags`（apilist 说是小活动限定道具的使用次数）、`api_furniture_affect_items` 都不是要持久保存的状态，不导入。

## 推断规则（任务）

`questlist` 只返回当前能看到的任务：`api_state` 1 表示未接，2 表示执行中，3 表示已达成待领奖；`api_progress_flag` 0/1/2 分别表示不到 50%、50%、80%。领过奖的任务不会出现在列表里，所以：

- **单次任务是否已完成**：从 `quest.json` 的 `prerequisite` 图出发反复迭代，直到不再变化。一个单次任务如果所有前置都已满足、按理应该能看到，却不在列表里，就记为已完成（`quest_record_oneshot`）。
- **周期任务本期是否已领**：可用但不在列表里，就算本期已领（`quest_record_periodic`，周期取导入时所在的那一期）。
- **执行中的任务**：`status←Activated`，`progress` 按标志位取 `Empty/Half/Eighty`；`api_state: 3` 的取 `Completed`。具体计数器（`requirements`）只能按标志位取一个比例近似值，导入报告里要列出来。
- **码典里没有的任务**：915、984、1152 不在 `quest.json`，没法导入，只能告警。其中 1152 是 1036 和 1153 的前置：本地无法记录 1152 已完成，这两个任务在推断里就永远不满足前置，重建码典之前也接不了。
- 这份快照里所有任务都是未接状态，"执行中"和"已达成"两条路径测不到。

## 可当对拍用的线上值

导入后让本地重算，再和线上原值比较，每一处差异要么是公式 bug，要么是已知的近似。这正是"导入能把各模块的问题逼出来"的具体做法：

- 舰娘的 `api_karyoku..api_lucky[0]`（含装备加成）和 `[1]`、`api_maxhp`、`api_soku`、`api_leng`：本地重算后对比。**建议放进第一刀。**
- `api_exp[1]`（升到下一级还要的经验）、`api_ndock_time`、`api_ndock_item`、`api_srate`、`api_locked_equip`。
- require_info 的 `api_unsetslot`：本地按 `type3` 推出的未装备列表，应该和线上一致。
- `remodel_slotlist` 给出的最低消耗，已经知道是码典的 `dev_mat_min` / `screw_min`（见 `live-api-investigation.md`）。

## 解析用的类型（未实测）

`emukc_model::kc2` 里已经有这些类型：`KcApiShip`、`KcApiSlotItem`、`KcApiDeckPort`、`KcApiKDock`、`KcApiNDock`、`KcApiUserItem`、`KcApiFurniture`、`KcApiMapInfo`、`KcApiAirBase`、`KcApiPlaneInfo`、`KcApiAirBaseExpandedInfo`、`KcApiMission`、`KcApiPresetDeck`、`KcApiPresetSlot`、`KcApiPresetDevItem`、`KcApiPictureBookShip`、`KcApiPictureBookSlotItem`、`KcApiUserBasic`、`KcApiMaterialElement`，以及三个设置类型。它们能不能原样反序列化线上 JSON 还没试过，第一步要拿快照逐个跑一遍。已知的问题：`port.rs` 里 `api_plane_info` 声明成了 `Vec`，实际是对象；`api_furniture_affect_items` 也声明错了（见 `live-api-investigation.md`）。导入时 port 应该按字段取，不要整体解析。

## 10-27 补抓

`scripts/fetch_live_api.py` 加了 `preset_slot`、`preset_dev_items`、`payitem` 和 `picture_book`。图鉴的 `api_no` 是页码，从 1 开始，每页 70 条（见客户端的 `AlbumAPI`）。脚本按本次抓到的 `api_start2` 里的 `api_sortno` 算出有内容的页再去请求：舰娘 15 页，装备 9 页。`api_port` 的签名里含当前秒数，所以挪到了第一个请求。48 个请求全部返回 `api_result: 1`，全是读接口，没有写操作。

图鉴响应和本地实现有出入，这是导入以外的发现：线上一个条目会把同一舰娘的多个形态合在一起（`api_table_id` 和 `api_state` 都是数组）；本地 `api_get_member/picture_book` 每个条目只发一个形态，`api_table_id` 是 `[mst.api_id]`，`api_state` 只有一行。已开 #302。

戦果（`war_result`）在排行榜接口里，那个接口名是混淆过的，还要另外签名。它对导入价值不大，先取 0。

## 第一刀建议

舰娘、装备、舰队、资源、使用道具、家具、设置。这一刀的难点是 ID 重映射，验证用的就是上面"对拍"那一节：重算的舰娘属性和线上值对比。任务、海域进度、远征、图鉴、预设放到第二刀。
