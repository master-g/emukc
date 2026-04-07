# EmuKC Battle System Plan

> 这份文档描述 battle 子系统的**当前实际基线**与下一阶段工作，不再复述已经被实现掉的旧计划。

## Current Baseline

### Core engine

- practice 与 sortie 现在共享同一套 battle core
- battle context 已区分 `BattleMode::Practice` / `BattleMode::Sortie`
- 当前 day battle type 已覆盖：
	- `Normal`
	- `AirBattle`
	- `LdAirBattle`
	- `LdShooting`

### Implemented phase coverage

- 航空战 / 开幕航空战
- 开幕对潜（OASW）
- 开幕雷击
- 昼战炮击
- 闭幕雷击
- 夜战连击 / CI 的基础展示与结算
- sortie `sp_midnight`（夜战点直接开夜战）

### Safety and settlement invariants already fixed

- 出击战中，**非大破入场**的己方舰不会在本战被击沉
- 旗舰不会沉
- 该保护只对 sortie friendly side 生效；演习和敌方都不会触发
- 友军沉舰会影响胜利评级，不再把“己方沉船”算成 `S`
- 已沉舰不会继续拿 ship EXP，也不会像正常存活舰那样参与返港结算语义

### Data-fidelity fixes already landed

- `api_si_list` 已按 attack context 选择展示装备，而不是盲目回退前两个槽位
- 敌舰 bootstrap 装备若不存在于当前 manifest，会在 runtime 丢弃并同步清零对应 `onslot`
- manifest-only fallback 敌舰在缺装备数据时返回自洽的 `api_onslot = [0; 5]`
- torpedo payload direction 已修正：
	- friendly dealt damage -> `api_fydam` / `api_fydam_list_items`
	- enemy dealt damage -> `api_eydam` / `api_eydam_list_items`

## What Is No Longer The Main Problem

下列问题已经不是当前 battle 子系统的主缺口：

- 沉船保护缺失导致昼战直接击沉
- 被击沉后仍吃经验并像正常舰一样返港
- torpedo damage payload 左右方向写反
- `cell_0` 起点错误导致单舰队常规图一开场就飞离航道

这些都已经有代码与测试落地。

## Current Real Gaps

### 1. Enemy master-data source is still too weak

当前 sortie battle 能从 map catalog 拿到 enemy ship IDs，而且**repo-tracked normal map 里出现的敌舰 ID 当前都能命中 `enemy_ship_extra`**。

但 battle-ready 敌舰数据源仍没有完全收敛，问题主要变成了：

- 这条覆盖是否能稳定扩展到后续新增 normal maps / event maps
- 一旦落回 `manifest-only fallback`，退化路径是否还足够可控

运行时仍然存在：

- `codex.new_enemy_ship(ship_id)` 成功时走 enemy bootstrap data
- `codex.new_ship(ship_id)` 成功时走完整构建
- 失败时退回 manifest-only fallback

所以真正限制 fidelity 的第一问题已经变成：

- 如何把当前 enemy bootstrap coverage 稳定守住，并继续缩小 manifest-only fallback 的适用面

### 2. Advanced battle topologies are still missing

当前 battle baseline 仍以**单舰队常规战斗**为主。下面这些还没有完整落地：

- combined fleet battle
- base air sortie / LBAS
- support expedition
- 更完整的 event / special topology handling

### 3. Target taxonomy is still incomplete

虽然对潜 / 非对潜 / 雷击目标约束已经有第一批规则，但更完整的目标分类仍未建完，例如：

- `Installation`
- `PT`
- 更细粒度的对陆 / 对海合法目标约束

battle core 现在已经有第一阶段显式分类：`SurfaceShip` / `Installation` / `PT` / `Submarine`。
当前真正未完成的是 attacker-side legality 仍主要只按 “submarine vs surface-like” 分桶，还没有把对陆 / 对海差异正式接进去。

### 4. Display / response rules are still partly hardcoded

`api_si_list` 已经从最危险的错误里走出来，但 battle core 里仍有不少“规则已知、实现仍偏硬编码”的分支。

这会限制：

- 新 battle type 扩展
- 事故复盘
- 和 decoded client rules 的逐项对表

## Next Work Tracks

### Track 1. Introduce a real enemy battle-data source

目标：

- 让常规 abyssal / event enemy 的 battle-ready stats、装备、onslot、特殊属性有稳定来源
- 减少 manifest-only fallback 的覆盖面

产出预期：

- 当前 normal-map coverage 被回归测试守住，后续新增 map / enemy 数据不会悄悄退回 manifest-only fallback
- enemy build path 不再以“尽量自洽”为目标，而能在更大范围内接近线上真实值

### Track 2. Complete target legality / taxonomy

目标：

- 在现有 `Installation` / `PT` taxonomy 之上，把对陆 / 对海目标合法性纳入统一规则层
- 为后续 combined / support / event battle 打基础

### Track 3. Move more response logic into structured rules

目标：

- 减少 `api_si_list`、attack display、phase-specific payload 的硬编码
- 建立更明确的规则表和 incident corpus

### Track 4. Expand topology coverage

目标：

- combined fleet
- support expedition
- LBAS
- 更多 night / air / special battle path

## Relationship To Map Work

map 子系统本轮已经把“起点不忠实”这个大问题压下去了。battle 下一阶段最值得做的，不是再修 start routing，而是：

1. 利用更稳定的 map enemy encounter 数据
2. 补齐 battle-ready 敌舰属性来源
3. 在此基础上继续扩展 advanced battle coverage

## Recommended Validation

```bash
cargo test -p emukc_gameplay
cargo test -p emukc_gameplay --test sortie_battle
cargo run -- battle validate --input <battle.json>
```
