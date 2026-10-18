---
title: "Escort Flagship Protection in Every Phase - Plan"
type: fix
date: 2026-10-18
status: implemented
execution: code
---

# Escort Flagship Protection in Every Phase - Plan

Issue #226。来源 `kancolle-replay@69097abc`：`kcships.js:45` 每支 `Fleet` 都把第一艘标为 `isflagship`（护卫舰队 `F1C` 也是），`:1825` 以此给保护，与阶段无关；`kcsim.js:2121-2126` `takeDamage`。

**范围**：U1–U2，一个 PR。
**非目标**：12 对 12；`select_escort_shield` 与特殊攻击里把第 0 艘当旗舰的判断（攻击方或单支舰队内，已正确）。

**基线**：`main` `9fec9456`。改前唯一漏保护的是敌方空袭（`kouku.rs` 把整个联合舰队传给 `execute_airstrike_phase`，护卫旗舰在 `escort_start`）。

## Implementation Units

### U1 旗舰是舰船的属性

- `BattleRuntimeShip` 加 `is_flagship`；`BattleState::from_context`（生产代码唯一建船处）给我方舰队与护卫舰队的第一艘打标记，敌方不打（保护只给我方）。
- `apply_damage` 读 `is_flagship`，去掉 `ship_index` 参数；调用点只删实参。

### U2 测试与文档

- 单测：40 个种子，CTF 四艘 1/30 HP 的驱逐对四艘正规空母，两支舰队的旗舰都不沉，护卫第二艘至少沉一次（证明命中致命）。改前第 3 个种子护卫旗舰被击沉。
- `types/mod.rs` 的保护单测改为显式标旗舰。
- `day-shelling-order.md` 改写保护的说明。
