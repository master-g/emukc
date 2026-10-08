---
title: "Transport Gauge for 5-6 - Plan"
type: feat
date: 2026-10-08
status: implemented
execution: code
---

# Transport Gauge for 5-6 - Plan

## Problem

5-6 的第一段是输送血条：在 G 点取得 A 胜以上，按舰队运载的输送量（TP）扣减，共 280。
计划 005 里它是占位（G 点 A 胜以上 3 次），用户当时定了另开计划，2026-10-08 说开始。

## Decision

- **输送量怎么算**：每艘舰按舰种给点（駆逐 5、軽巡 2、航巡 4、航戦 7、水母 9、揚陸艦 12、補給艦 15、
  練巡 6、潜水母艦 7、潜水空母 1，其余 0），装备另加（上陸用舟艇 8、ドラム缶 5、特型内火艇 2、戦闘糧食 1），
  鬼怒改二自带一艘大発。S 胜全额，A 胜乘 0.7 向下取整，更低不计。来源：zekamashi.net `yusou-tp`，2026-10-08 读取。
- **什么时候算**：抵达揚陸点 E 时按当时的舰队算一次，记在这次出击上；大破的舰和她的装备不算，之后再受伤不影响。
  没经过揚陸点就打到 boss，不扣血条。
- **告诉客户端什么**：按客户端代码，非活动图的输送血条用 `api_gauge_type` 3 加
  `api_required_defeat_count`（总长）和 `api_defeat_count`（已输送）画出；boss 战结果带 `api_landing_hp`
  播放揚陸动画；揚陸点是 `event_id` 9 的格子。5-6 的 E 在地图数据里是空格子，由阶段规则把它标成揚陸点。
- **规则文件**：`map_gauge_rules.json` 的 `wins` 换成 `tp` 加 `landing`；变体上的 `gauge_counts_wins` 换成
  `transport_gauge`。占位用的"A 胜计次"没有别的使用者，一并删除。
- 不动：出击中途退避的舰不再运载（常规图没有退避）；活动图里某些装备的特殊点数。

## Verification

- 单元测试：`a_ship_brings_her_type_and_her_equipment`、`an_a_rank_lands_seven_tenths_rounded_down`、
  `build_map_info_draws_a_transport_gauge_only_at_its_stage`。
- 走真实 5-6 数据：`map_5_6_empties_its_transport_gauge_by_what_the_fleet_lands`、
  `map_5_6_counts_the_cargo_on_arrival_at_the_landing_cell`、`a_badly_damaged_ship_carries_nothing`。
- `make route-oracle` 差异 0；三道质量门。
- 再生资产：`map_gauge_phases.json`（由规则文件生成）。
