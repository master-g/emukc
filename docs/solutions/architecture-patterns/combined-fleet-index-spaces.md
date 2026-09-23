---
module: emukc_battle, emukc_gameplay
tags: [combined-fleet, battle, packet, indexing]
problem_type: architecture
---

# 連合艦隊 的两套 friendly 索引空间

联合舰队战斗里，"友方第 n 艘舰" 有两个互不相同的含义，而且两者都出现在同一次战斗
的数据流里。分不清它们，命中就会记到错误的舰上——不会崩，不会报错，只会渲染错。

## 两套空间

**模拟空间（连续）。** `BattleState::friendly` 把第2艦隊 直接接在 第1艦隊 后面，
边界 `escort_start` 就是 第1艦隊 的**实际舰数**。这样选出来的切片天然对应相位语义：
敌方打整支友军时是整条 slice，某支 deck 单独行动时是一段子 slice。

**客户端空间（固定偏移）。** 解码客户端的 `_getNum` 按 `index >= 6` 分派，然后读
`combined[index - 6]`。也就是说 第2艦隊 **永远**从 6 开始，哪怕 第1艦隊 只有两艘。
`docs/apilist.txt` 里 `api_raigeki.api_frai` 写成 `[12]` 也是这个意思。

第1艦隊 不足六艘时，两套空间之间就出现一个空洞，包必须把它带上。

## 翻译在哪里做

只在一处：`BattleState::finalize_day` / `finalize_night` 的末尾，调用
`crates/emukc_battle/src/combined_packet.rs`。所有相位函数、所有调用方都只在模拟
空间里工作，不需要知道有第二套空间。

翻译做四件事：

1. 砲撃轮（`api_hougeki1/2/3`、`api_opening_taisen`、夜战 `api_hougeki`）里的
   friendly 索引平移。`api_at_eflag` 决定哪一端是友方：`0` 是友方攻击（平移
   `api_at_list`，`api_df_list` 里是敌方索引不动），`1` 是敌方攻击（反过来）。
2. `api_raigeki` / `api_opening_atack` 的 friendly 数组重排到固定 12 槽，敌方数组
   裁回敌舰数（模拟按 `max(friendly, enemy)` 超额分配）。`-1` 是"无目标"哨兵，
   不能当索引翻译。
3. `api_kouku.api_stage3` 按 deck 切成 `api_stage3`（第1艦隊 + 整个敌方）与
   `api_stage3_combined`（第2艦隊）。这一项是**切分**不是重排——客户端按各自 deck
   的舰数读，不需要空洞。
4. `BattlePacket::friendly_nowhps` **故意不翻译**。它不上 wire（昼战响应报的是
   入场 HP，取自输入），而 sortie session 按舰位索引它。

## 切片编号：第三个坑

`simulate_shelling_side` 按**交给它的切片**起点从 0 编号 attacker。联合舰队按 deck
切片调用它，所以 第2艦隊 的攻击回来时是 0,1,2 —— 既不是模拟空间也不是客户端空间。
`execute_combined_shelling` 必须先用 `shift_friendly_attackers` 抬回模拟空间，上面
那次 remap 才能把它转成客户端空间。

两层平移缺任何一层，结果都是"第2艦隊 的炮击全部记在 第1艦隊 头上"。
`escort_deck_attacks_reach_the_packet_in_client_index_space` 钉住这条。

## deck 边界不必另存

`BattleRuntimeShip` 自带 `is_main_deck()` / `is_escort_deck()`。sortie session 把两支
deck 存成一条连续 vec，边界从舰船标签现算——没有第二个字段可以和它不一致。夜战因此
不需要给 `NightBattleInput` 加"这是联合舰队"的标志：传进去的就是 第2艦隊，
`finalize_night` 从舰船标签上认出来。

这条 vec 的布局只由 `SortieBattleSession`（`crates/emukc_gameplay/src/game/battle/sortie/mod.rs`）
维护：`main_deck()` / `night_fleet()` 取两段，`escort_start()` 给出边界（单舰队为 `None`，
没有 0 哨兵），`absorb_night()` 把夜战结果接在 第1艦隊 之后，`friendly` 与
`friendly_nowhps` 同步。sp_midnight 用同一个 `absorb_night()` 建 session。编排代码不直接
截断或拼接这两条 vec。

## 端点与编成是双向契约

gameplay 侧不看 URL，只看 profile 的 `combined_type`。所以放开联合舰队之后，
`api_req_sortie/battle` 会照样跑联合战斗，返回单舰队客户端读不懂的包；反过来
`battle_water` 对 空母機動部隊 会返回镜像错位的砲撃顺序。
`SortieBattleEndpoint` + `SortieBattleSetup::validate_endpoint`
（`crates/emukc_gameplay/src/game/sortie/setup.rs`）把这条恢复成显式契约。

顺带一条字段归属：`api_combined_flag` **不是**战斗响应字段，它属于
`api_port/port`（`docs/apilist.txt:777`）；`api_combined_type` 属于
`api_req_hensei/combined`（:3924）。`docs/battle/combined-fleet-reference.md`
的 §Protocol fields 是"客户端会读到的字段"总表，不等于战斗包的字段表。

## 测试联合舰队夜战

1-1 首格打不出夜战：1..=40 号种子里没有一个留下双方存活。需要夜战链路的测试必须
像 `weaken_for_midnight` 那样直接注入弱化舰队——`update_ship` 不写
`api_karyoku`/`api_soukou`（派生字段），从 DB 侧改不动。
昼战 → battleresult 这一半可以走集成测试。
