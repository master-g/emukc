---
title: "Opening Airstrike Follows the Source - Plan"
type: fix
date: 2026-10-17
status: implemented
execution: code
---

# Opening Airstrike Follows the Source - Plan

Issue #207。来源 `kancolle-replay@69097abc` `kcsim.js`：`AADefenceBombersAndAirstrike`（2543）、`getContact`（2505）、`airstrike`（1968）。每方：有攻击机才继续 → 触接（AS+/AS/劣势才掷）→ 每个攻击机槽经 `choiceWProtect(targets, null, true)` 抽目标，全部抽完 → 逐个结算：命中、鱼雷机 ×0.8/×1.5、`floor(cap(pow)) × 触接`。

**范围**：U1–U3，一个 PR。
**非目标**：出击顺序、击坠与抽目标交错、两方先后（issue 第二条评论撤掉了 RNG 顺序要求）；50/50 第一/第二舰队分流（来源在 `if (false)` 里）；気球（#198）、対空噴進弾幕、特殊加成与 dock/summer 倍率（#200）、喷气机 ×0.7+0.5、基地航空队（#208）、警戒陣重抽（#195）。

**基线**：`main` `0761eee3`。

## Implementation Units

### U1 触接与单槽伤害

- `roll_contact(codex, rng, carriers, air_state) -> Option<(f64, i64)>`（#208 复用）：第一段概率 `(1 + Σ floor(√机数 × 索敌)) / 25|40|55`，只计能触接、非鱼雷机的槽；第二段按舰、按槽，命中不高于当前选中者的跳过，`索敌 / 14|16|18`。倍率按命中 ≥3 / 2 / 其余：1.2 / 1.17 / 1.12。能触接的种类：8、9、10、41、49、94。第二段不看机数（来源如此）。
- 单槽伤害：未命中直接 0，不再掷防御；鱼雷机命中后掷 0.8/1.5；陆上目标的雷装记 0、基础 25 保留（用户 2026-10-17 选定照来源），删去「装备威力为 0 就不打」的提前返回；触接倍率在上限之后、暴击之前。

### U2 先抽完再结算

- 每方一次遍历：按舰、按槽（爆击、雷击混排），每个仍有飞机的攻击槽 `choice_with_protect(…, 0, true, 防守方阵形)`；候选是水面目标（不含潜艇，来源的 `targets` 是 `alive2`）。
- 出击舰取空袭开始时还浮着的：先被基地航空队击沉的不再出击。
- 抽完再逐个结算；被前一击击沉的目标仍被攻击，伤害作废。かばう 标记写成 `DamageCell::Shielded`。
- 一方没有攻击机就不掷触接，`api_touch_plane` 为 -1。
- `simulate_kouku` 多传两方阵形。

### U3 测试、golden 与文档

- 单测：鱼雷机伤害在多个种子下取到两种值；已知舰队与制空状态下的触接概率与倍率；均势不掷；`api_touch_plane` 是掷中的飞机或 -1；旗舰可被かばう；陆上目标不计雷装；同一目标两击、第二击打在已沉的船上。
- 两份 golden 按各自流程重冻并在 PR 说明；定种子测试落空就换种子。
- `docs/solutions` 里写着“没做”的条目随之更新。
