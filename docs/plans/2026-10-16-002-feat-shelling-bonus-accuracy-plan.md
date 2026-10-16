---
title: "Equipment Bonus Accuracy in Day Shelling - Plan"
type: feat
date: 2026-10-16
status: implemented
execution: code
---

# Equipment Bonus Accuracy in Day Shelling - Plan

Issue #170。装備ボーナス表算出的命中（`GearBonusStats.houm`）自 #169 起已与客户端 `SlotItemEffectUtil` 对齐，但没有读者。按钉住的 `kancolle-replay` 提交 `69097abc`，把它加进昼战炮击的命中。

**范围**：U1–U2，一个 PR。
**非目标**：
- 夜战、雷击、对潜、航空攻击的命中（来源不加）。
- 舰船面板显示命中；`cal_ship_status` 不动。
- 改动装備ボーナス表或其 corrections。

**基线**：`main` `2000feba`。

## Evidence

来源文件从 `raw.githubusercontent.com/KC3Kai/kancolle-replay/69097abc/js/` 取，仓库里不存副本。

- `kcships.js:1011-1028`：逐件装上装备算 `getBonusStats`，总和存进 `statsEqBonus`；加到舰船属性时跳过 `ACC`（和 `DIVEBOMB`）。
- `kcsim.js:524-529` `shell()`：`accflat` = 装备命中 + 改修命中 + `statsEqBonus.ACC`，再交给 `hitRate(ship, base, accflat, accMod)`。`hitRate`（`:2139`）是 `(base + 2√Lv + 1.5√運 + accflat) × accMod`，所以阵型、士气、特殊攻击系数都乘在它上面。
- `shell()` 也处理特殊攻击（`attackSpecial`）和空母炮击，所以昼战炮击的所有路径都加。
- 全文件 `statsEqBonus.ACC` 只出现在 `:526`。夜战 `NBattack`（`:695`）、雷击（`:1856-1878`）、对潜 `ASW`（`:975`）都不读它。
- 我们这边：命中在 `accuracy.rs` `roll_attack`，平加项是 `equipment_accuracy` 与 `aim.flat`。昼战炮击（`shelling.rs:290`）和特殊攻击（`special_attack.rs:632`）都以 `AttackKind::Shelling` 进来。
- 夜战空母的雷装加成（`night.rs` `carrier_torpedo_bonus`）已经从运行时舰船拼 `GearBonusGear` 列表，两处共用这一段。

## Implementation Units

### U1 炮击命中加装備ボーナス命中

- 把构造 `GearBonusGear` 列表的代码从 `night.rs` 提到 `targeting.rs`（与 `ship_mst` 同处），两处共用。
- 把 `roll_attack` 里命中值（截断前）的算式提成函数，便于直接测。
- `AttackKind::Shelling` 时，在 `equipment_accuracy` 旁边加 `codex.gear_bonus(...).houm`；拿不到舰船或表出错按 0。
- 测试：鈴谷航改二 带 2 件 373，炮击命中值比不算加成时高出 `加成 × 士气 × 阵型 × 系数`；夜战、雷击、对潜的命中值不含加成。

### U2 文档

- `equipment-stat-bonus.md` 的 Known gap（表里的命中没用上）改成现在在哪里用。
- golden 或固定种子测试若变化，有意重冻并在提交里说明规则和变化的种子数。
