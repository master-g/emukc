# Battle Data Fidelity Status

> 这份文档只关注“battle payload / response 自洽性与客户端兼容性”这一条线，不展开完整 battle roadmap。

## Current Status

第一轮高优先级 data-fidelity 修复已经完成，当前已落地的关键项包括：

- `api_si_list` 按攻击上下文返回展示装备，不再无脑回退前两个槽位
- manifest 中不存在的敌舰装备会在 runtime 被丢弃，并同步清零对应 `onslot`
- manifest-only fallback 敌舰在缺少装备数据时返回自洽的 `api_onslot = [0; 5]`
- torpedo payload direction 已修正为：
	- friendly dealt damage -> `api_fydam` / `api_fydam_list_items`
	- enemy dealt damage -> `api_eydam` / `api_eydam_list_items`
- battle writeback 与保护逻辑已经统一到 `BattleRuntimeShip::apply_damage()`，避免某些 phase 重新写出“看起来已沉”的错误状态

## What This Means

当前 battle payload 的问题已经从“明显会把客户端带崩的协议错误”转向更深层的 fidelity 问题：

- 数值能否更接近线上
- 敌舰属性来源是否足够完整
- 展示规则是否足够结构化、可审计

也就是说，下一阶段不是继续修补最基础的 response 自洽性，而是要把 battle 数据源与规则表做得更强。

## Remaining Gaps

### 1. Enemy data source fidelity

如果 enemy build 仍然拿不到 battle-ready 的完整属性/装备，payload 虽然可以自洽，但仍可能和线上不完全一致。

### 2. Structured display rules

`api_si_list` 已经修掉最危险的事故源，但更多 display / payload 规则仍散落在 battle core 分支里。

### 3. Incident corpus depth

当前已有回归样例，但还需要更多真实事故样本来约束：

- 昼战 / 夜战展示装备
- 特殊攻击 payload
- 客户端对 phase 字段存在性的敏感点

## Recommended Order

1. 先补 enemy master-data source
2. 再把 display / payload 规则继续抽成结构化规则表
3. 持续扩充 incident corpus，把 battle validate / analyze-incident 用起来
