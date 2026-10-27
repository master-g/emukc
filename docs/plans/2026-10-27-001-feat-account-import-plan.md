---
title: "Account Import from a Live Capture (slice 1) - Plan"
type: feat
date: 2026-10-27
status: implemented
execution: code
---

# Account Import from a Live Capture (slice 1) - Plan

把 `scripts/fetch_live_api.py` 抓下来的线上账号快照导入成一个本地账号。依据：
- 字段对照：`docs/research/2026-10-27-account-import-field-map.md`（下文简称"对照"）；
- 抓取规则：`docs/solutions/best-practices/live-api-investigation.md`；
- 图鉴形态差异：#302。

**基线**：`main`（PR #303 之后）。

## 三个默认（2026-10-27 用户已确认）

- **导入目标**：新建账号和档案，走 `sign_up` → `new_profile`，并跳过教程，做法同 `new-session`。用户名已存在就拒绝。原因是覆盖已有档案要先决定删什么、留什么，放到第二刀。
- **码典缺主数据时**：整单中止，列出缺的舰娘、装备 ID。半导入的舰队（舰娘在、装备没了）比不导更糟。10-27 快照的舰娘和装备 ID 在码典里都能找到。
- **入口**：新增顶层子命令，候选名 `import-account --snapshot <dir> -u <name> -p <pass>`，读取 `fetch_live_api.py` 写出的目录。Rust 这边不碰 token，也不发请求，所以导入可以离线跑、可以测试。

## 非目标

任务、海域记录、基地航空队、远征、预设（编成、装备、开发）、图鉴形态（#302）、戦果、覆盖已有档案。这些都放到第二刀。

因此第一刀导入后的档案会是这个样子：舰娘、装备、舰队、资源、道具、家具、设置都和线上一致；任务、海域开放、远征、基地航空队都是新号状态（`init_profile_game_data` 生成的那份）。图鉴只有导入时持有的舰娘和装备，因为会走 `add_*_to_picturebook_impl`。

## 事实

- `ship.id` 和 `slot_item.id` 都是全局自增主键，多个档案共用。现有的 `From<KcApiShip> for ship::ActiveModel` 会写入 `id: Set(api_id)`，导入不能直接用它，必须改成 `NotSet` 后再插入。
- 新建档案时，`init_profile_game_data` 会给每个领域生成初始行：舰队、船坞、资源、设置、家具等。所以第一刀涉及的领域要在同一个事务里替换这些行，而不是往空表里插。
- `add_ship_impl` 插入舰娘后会做两件事：给装备回填 `equip_on`（`update_slot_item_impl`），写图鉴（`add_ship_to_picturebook_impl`）。`add_slot_item_impl` 用 `exp_of_level(alv)` 算熟练度经验，并写装备图鉴（`add_slot_item_to_picturebook_impl`）。导入复用这几个辅助函数，不另写一套规则。
- `recalculate_ship_status_with_model` 会按本地公式重算舰娘属性（含装备）。
- 快照用到的 `KcApi*` 类型都已经能反序列化（`KcApiShip`、`KcApiSlotItem`、`KcApiDeckPort`、`KcApiUserBasic`、`KcApiMaterialElement`、`KcApiNDock`、`KcApiKDock`、`KcApiUserItem`、`KcApiFurniture` 等），但还没有拿线上 JSON 实际跑过。`port.rs` 里 `api_plane_info` 和 `api_furniture_affect_items` 的类型声明是错的，所以 port 响应要按字段取，不要整体解析。
- 夹具 `z/snapshot/2026-10-27/`：同一时刻抓的 48 个文件。舰娘 340 艘，装备 1855 件（362 件带熟练度），没有舰娘带 `api_sp_effect_items` 或 `api_sally_area`，没有中队在换装中。快照是个人数据，不能进仓库。

## Implementation Units

### U1 快照类型与读取

- 在 `emukc_model` 里定义 `LiveSnapshot`，由第一刀用到的端点的 `api_data` 组成：port 的 `api_basic`、`api_ship`、`api_deck_port`、`api_ndock`、`api_material`，require_info 的 `api_slot_item`、`api_kdock`、`api_useitem`、`api_furniture`、`api_oss_setting`、`api_skin_id`、`api_position_id`、`api_extra_supply`，以及 port 的 `api_p_bgm_id`。
- 读目录、去掉 `svdata=` 前缀、检查 `api_result == 1`，这些放在二进制 crate 里，因为涉及文件 IO。分层保持：类型在 `emukc_model`，导入在 `emukc_gameplay`，读文件在 `src/bin/cli`。
- 测试：手写一个合成夹具，2 艘舰娘、3 件装备（其中 1 件带 `api_alv`、1 件装在补强增设上）、1 个舰队，不含任何账号里的真实值。确认它能解析成 `LiveSnapshot`，缺端点或 `api_result != 1` 时报错。

### U2 档案与简单领域

- 写 `Ctx::import_account(...)`，配套 `import_account_impl`，整次导入在一个事务里完成：要么全部写入，要么什么都不写。
- `profile` 按对照的映射写入，`intro_completed`、`tutorial_progress` 也来自快照。
- 替换 `material`、`use_item`、`furniture_record`、`furniture_config`、`oss_settings`、`option_settings.skin_id`、`game_settings.position_id/port_bgm` 的初始行。
- 测试：合成夹具导入后，读回来的值与夹具一致。

### U3 装备与舰娘（ID 重映射）

插入顺序和 `id: NotSet` 关系到正确性，不是风格问题，审查时要确认没人改动：

1. 先校验码典：所有 `api_slotitem_id`、`api_ship_id` 都要能在码典里找到，否则中止并列出缺的 ID。
2. 装备按线上 `api_id` 升序插入，`id: NotSet`，`equip_on` 先不设。熟练度经验用 `exp_of_level`，`type3` 取自码典。每件装备写装备图鉴。
3. 舰娘按线上 `api_id` 升序插入，`id: NotSet`。`slot_1..5` 和 `slot_ex` 通过装备映射换成本地 ID，`-1` 和 `0` 照原样保留。每艘舰娘写图鉴。
4. 按舰娘的装备栏回填装备的 `equip_on`。
5. 每艘舰娘调用 `recalculate_ship_status_with_model`。
6. `api_sp_effect_items` 有就写入，`ship_id` 换成本地 ID。合成夹具里放 1 艘带它的舰娘，因为 10-27 快照里一艘都没有。

按升序插入，"入手順"的相对顺序就能保住。测试要覆盖三点：
- 本地 ID 和线上 ID 不同（数据库里先放一个别的档案的舰娘）；
- 装备的持有者正确；
- 排序与线上一致。

### U4 舰队与船坞

- 舰队：`fleet.ship_1..6` 用舰娘映射换成本地 ID；`name` 直接写；远征中的舰队保留 `mission_status`、`mission_id`、`return_time`。`launch_*` 按导入时的编成计算，`launch_fully_supplied` 取 true（理由见对照）。
- 修理坞：`ship_id` 换成本地 ID，保留完成时间和投入资源。建造坞：照原样写入，`ship_id` 是主数据 ID，不用换。
- 测试：合成夹具里有 1 个远征中的舰队和 1 个修理中的舰娘，导入后计时器和引用都对得上。

### U5 导入报告

导入返回 `ImportReport`，CLI 把它打印出来：

- 逐艘对比本地重算的结果和线上原值：`*_now/*_max`（火力到运）、`hp_max`、`speed`、`range`、`exp_next`、`ndock_time`、`ndock_item`、`srate`、`has_locked_euqip`。只列出不一致的舰娘和字段。
- 近似了哪些（`launch_*`、`aircraft_exp`、`married`），丢弃了哪些（`onslot_plus`、port 里不导入的键）。

这一单元是这个方向的价值所在：每一处不一致，要么是本地公式的 bug，要么是已知的近似。测试：合成夹具里故意改坏 1 艘舰娘的线上火力，报告里要列出它。

### U6 文档

- 新增 `docs/solutions/`：导入的流程、ID 重映射的顺序、报告怎么看。
- `CLAUDE.md` 的命令一节加一行；`BOOTSTRAP.md` 如果列了命令，也补上。
- 把对照文档里第一刀已经覆盖的行标为已实现。

## 验证

- U1–U5 的合成夹具测试通过。质量门三项都要过：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- 手动跑一次：把 `z/snapshot/2026-10-27` 导入一个临时数据库（不碰正在用的 `.data`），把 U5 报告的摘要贴进 PR，只贴计数和字段名，不贴账号的值。
- 不加读取真实快照的 `#[ignore]` 测试：被忽略的测试必须在 PR 里报出，而这份快照是个人数据。
- 起服务器用导入的账号登录，母港、编成、改装画面能正常打开。用 `make headless-check` 那套无头客户端，或者人工看一眼。
