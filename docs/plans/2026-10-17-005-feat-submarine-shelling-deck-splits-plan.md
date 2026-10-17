---
title: "Submarine Shelling and Combined-Fleet Deck Splits - Plan"
type: feat
date: 2026-10-17
status: implemented
execution: code
---

# Submarine Shelling and Combined-Fleet Deck Splits - Plan

Issue #203。来源 `kancolle-replay@69097abc`：`SS.prototype.canShell`（`kcships.js:2132`），`hasDH3`/`numWG`（`kcships.js:489`、`:511-518`），`orderByRange` 与两段炮击的 `hasInstall` 门槛（`kcsim.js:3366`、`:3812-3838`；`kcsimcombined.js:968-985`），`shellPhaseTarget` 潜艇只打陆上（`kcsim.js:1104`），`doShellC` 的 `.39`（`kcsim.js:1578`），`torpedoPhase` 的 `.35`（`kcsim.js:1795-1838`）。

**范围**：U1–U3，一个 PR。
**非目标**：12 对 12（本仓库没有）；`doShellC` 的 `isAntiPT`（#197）；`isAntiInstall`（钉住的源码从不赋值）；特殊攻击各击的舰队抽选；雷击对陆上设施的排除；联合舰队对单舰队时敌方只打当轮那支舰队（现为打整个己方）；单舰队对敌联合的第 3 轮只有敌方主力还击（`kcsimcombined.js` 的 `order2` 取自 `ships2`，现为两支都开火）。

**基线**：`main` `42495364`。

## Implementation Units

### U1 潜艇炮击

- `can_shell_day_ship`：SS/SSV 活着且带 WG42（126）或 特型内火艇（`api_type[2]` 46：167、525、526）才算能炮击。源码 `hasDH3` 来自 `B_LC3`（即类型 46 的默认 `btype`）及 525/526 的单独判定，合起来正是类型 46。
- 源码的建序门槛（对方有陆上设施才入序，`hasInstall`）不单独实现：潜艇的候选只有陆上设施，对方没有就打不出，差别只在随机数消耗（排序，以及对敌联合时她那一掷；代码里 `ponytail:` 注明）。
- `target_candidates`：昼战炮击里潜艇的候选只有陆上设施。

### U2 舰队抽选

- 昼战炮击：目标切片里有敌方联合舰队的两支舰队时（仅本仓库单舰队对敌联合的第 3 轮），每击先掷 `< 0.39`：先护卫，否则先主力；先选的舰队没有候选就换另一支。候选在各自舰队内求（对潜偏好按舰队算）。
- 闭幕雷击：友方打敌方联合舰队时，一支舰队无候选就打另一支，否则 `< 0.35` 打主力、其余打护卫；开幕雷击不分。
- 敌方打己方不分（本仓库的流程里己方联合舰队对面总是单舰队）。

### U3 测试与文档

- 单测（炮击轮）：WG42 潜艇对有设施的舰队开炮且只打设施；特型内火艇同样；无装备不开；对无设施的舰队不开。
- 单测（抽选函数）：0.39 先护卫与回落；0.35 主力/护卫与单舰队回落。
- `day-shelling-order.md`「Not modelled」删去三项并写明来源；golden 若动，按流程重冻并说明。
