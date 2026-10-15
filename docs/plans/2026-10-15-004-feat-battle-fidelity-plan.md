---
title: "Night Lights, Anti-Installation and Carrier Night Attacks - Plan"
type: feat
date: 2026-10-15
status: planned
execution: code
---

# Night Lights, Anti-Installation and Carrier Night Attacks - Plan

Issue #140（照明弹与探照灯）、#136（对陆上设施装备倍率）、#141（空母夜战与来源的差距）。三条都按钉住的 `kancolle-replay` 提交 `69097abc` 来做，合在一份计划里，以便统一安排顺序和测试的冻结时机。

**范围**：下文各 Implementation Unit。每个单元单独提交，可以分几个 PR 合并。
**非目标**：基地航空队攻击陆上设施；#143 的特殊攻击；客户端演出核对（#144）。
**基线**：`main` `b6f95ba6`。

## Evidence

来源文件按 `scripts/gen_night_attack_table.py` 的方式，从 `raw.githubusercontent.com/KC3Kai/kancolle-replay/69097abc/js/` 取。仓库里不存副本。

### golden 的前提

`tests/gameplay_tests/battle_golden.rs` 只冻结了一场战斗：`fresh_1_1` 场景、种子 1、1-1 的第一战，不进夜战（`midnight no`），敌方也没有陆上设施。下面的单元都只改夜战或对陆上设施的攻击，所以预计都不会改动这个 golden。
原先「三条合起来、只冻结一次 golden」的打算，前提因此不成立。真正可能受影响的，是 `emukc_battle` 里固定种子的夜战测试，以及 `SeededRng` 的 golden 向量。每个单元都跑这两类测试和 `battle_golden.rs`。种子测试结果变了就换种子，不放宽断言；golden 要是真变了，按它文件头的约定有意重新冻结，并在提交里写明原因。

### #140 照明弹与探照灯（`kcsim.js`）

- **照明弹**：`NBattack`（`:695`）里有 `accBase = accBase + starshells[0]*5`。己方照明弹打出时，己方夜战命中的基础值 69 加 5，再乘夜侦的系数。
  我们这边：`accuracy.rs` 的 `AttackKind::Night` 用 69.0，旁边有 `ponytail:` 注释说明没计入照明弹。照明弹的发动已经实现（`roll_star_shells`），只影响夜战特殊攻击的发动率。
- **探照灯的吸引火力**：
  - 夜战开始时，对每一方，在未撤退的存活舰里取最大的 `hasSearchlight` 作为 `slrerolls`：小型探照灯是 1，大型是 2（`kcsim.js:1672-1680`，`kcships.js:457-458`）。
  - 对这一方选目标时，`choiceWProtect`（`:2295`）在抽中的目标没有探照灯时重抽，最多 `slrerolls` 次，抽到带探照灯的就停。
  - 夜战选目标走 `nightPhaseTarget`（`:1739`）。
- **探照灯的回避代价**：`NBattack` 中目标带探照灯（`target.hasSearchlight`）时，`evMod *= .2; evFlat *= .2`。也就是阵形的夜战回避系数和固定回避（重巡、航巡 +5；驱逐带电探和見張員 +10）都乘以 0.2。看的是这艘船有没有装探照灯，不看它是不是点亮的那一艘。
- **我们这边**：`night.rs` 的 `NightLights` 只有一个 `searchlight: bool`，只影响发动率和 `abyssal_night_attack == 3` 的空母。目标选择走 `select_random_target_index`。回避的公式是 `accuracy.rs` 的 `evasion_term(evasion, luck, formation, fuel_shortfall)`。

### #136 对陆上设施（`kcships.js`）

- **设施种类**：`kcSHIPDATA.js` 给 180 艘敌舰标了 `installtype`：1 软目标 72 艘，2 炮台小鬼 16 艘，3 补给基地 62 艘，4 离岛 11 艘，5 北方 3 艘，6 港湾夏姬 16 艘。另有单独的标记：`isSupplyDepot` 8 艘，`isAnchorage` 6 艘，`isDock` 6 艘，`isSummerBB` 3 艘，`isSummerCA` 3 艘。
  有 `installtype` 就是陆上设施（`kcships.js:422`）。
  我们这边：`targeting.rs` 的 `target_class` 用 `INSTALLATION_TARGET_NAME_MARKERS` 按名字判断。
- **倍率与加成**：在 `Ship.prototype.loadEquips`（`kcships.js:429`）里，装备统计（`installeqs`、`numWG`、`numMortar`、`numRocket4`、`hasLC`、`installbonus1/3` 等）之后，算出这些量：
  - `installModAll`（约 `:582-686`，含 `abSynergyMult`）；
  - 六组乘区，各带昼战专用的一部分：`softSkinMult`/`Day`（`:718-741`）、`pillboxMult`、`isoMult`、`northernMult`、`harbourSummerMult`；
  - `installFlat`；
  - 补给基地的上限后倍率 `supplyPostMult`（`:852` 起）。

  另有泊地、船坞、夏季舰的倍率，其中船坞和夏季舰受 `SIMCONSTS` 开关控制，这三个开关默认都开（`kcsim.js:243-248`）。
- **应用**：
  - 昼战：`shellPower`（`:1529`）。目标是陆上设施时，按 `installtype` 用 `fp*Mult*(MultDay||1) + installFlat`，潜艇的 `fp` 多 30。
  - 夜战：`NBPower`（`:1557`）。同样按种类用 `fp*Mult + installFlat`，北方（5）再加雷装；没有对应倍率时只算火力。
  - 空母：`CV.prototype.shellPower`/`NBPower`（`:1986`、`:2024`）另有一套。
  - 补给基地的上限后倍率：`kcsim.js:566`、`:841`、`:2060`、`:3348`，条件是 `installtype == 3 || isSupplyDepot`。
- **我们这边**：
  - `damage.rs` 的 `night_gun_power` 对陆上设施只算火力，潜艇 +30；那里的 `ponytail:` 注释写明这些倍率还没做。
  - 昼战炮击的攻击力在 `calculate_shelling_damage`。
  - 夜战对陆上设施不计雷装，已在 `6b00bf99` 实现。
  - 裸装时的雷装：wikiwiki「戦闘について」的夜战一节写「対陸上でも、素手(装備を一切装備していない)の場合に限り雷装値が有効になる(つまり基本攻撃力 = 素火力 + 素雷装 + 夜間触接定数になる)」。来源里没有这条：昼战 `shellPower` 对陆上设施只算火力，夜战只有北方加雷装。

### #141 空母夜战（`kcships.js`、`kcsim.js`）

`docs/solutions/architecture-patterns/carrier-night-air-attack.md` 的 Known gaps 列了 7 项。来源里对应的位置：

- `CV.prototype.canNBAirAttack`/`hasNBAirGear`/`NBtypes`（`:2057-2084`）：能否夜间出击，以及特殊攻击按什么顺序判定。
- `getEquipBonusCVTorp`（`:1299`）：空母雷装的装备加成。
- `AO`（`:2148`）：补給艦，しまね丸改由此获得夜间出击的能力。
- `nightZuiunCIRate`（`kcsim.js:238`）及其分支：夜间瑞雲的特殊攻击。
- `NBattack` 的 `NBonly`、`isescort` 分支：联合舰队的护卫舰队在昼战后的夜战中对潜艇造成实际伤害。

## Decision

1. **顺序按依赖和规模排**：先做 #140（小、独立），再做 #136 的设施种类表（后面的倍率依赖它），最后做 #141（各项之间互不依赖）。
2. **来源里的数据表用脚本生成**：设施种类（`installtype`）和单独的标记，用 `scripts/gen_installation_types.py` 从钉住的 `kcSHIPDATA.js` 生成一张 Rust 表，形式照 `abyssal_night.rs`。表头注明「不手改，用脚本再生」，再写一个测试，钉住脚本打印的各类条数。这张表要同时登记进 `CLAUDE.md` 的禁止手改文件清单。
3. **来源里的计算逻辑手工移植**：倍率的计算是代码，不是数据。按来源的分支逐条移植，每条注释来源行号，每组倍率配一个已知输入、已知输出的测试。
4. **受开关控制的部分照默认值做**：`SIMCONSTS.enableModDock`、`enableModSummerBB`、`enableModSummerCA` 在来源里默认都开，所以船坞、夏季战舰、夏季重巡的倍率都做。
5. **裸装时的雷装照 wikiwiki 做**（用户 2026-10-15 定）：夜战中攻击陆上设施的舰一件装备都没有时，基本攻撃力 = 素火力 + 素雷装 + 夜間触接定数，不套任何设施倍率。
   这条只管夜战，和来源不一致，代码注释和文档里写明出处，并注明来源没有这条。它和北方（5）的「夜战加雷装」互不影响：裸装时两边都是火力加雷装。
6. **#141 第 1 项（敌方空母的夜战特殊攻击、装备加成和熟练度）放在最后**：它依赖敌方装备的熟练度数据，我们这边有没有还不确定。缺数据的话，这一项留在 #141 里，并写明缺什么。

## Implementation Units

每个单元都先写测试，做完跑 `cargo test -p emukc_battle`、`cargo test --test gameplay_tests battle_golden`，以及完整的三项质量门。

- **U1 照明弹命中（#140）**：照明弹打出的一方，夜战命中基础值加 5，再乘夜侦系数。去掉 `accuracy.rs` 里的 `ponytail:` 注释。
  测试：同一攻击者、同一目标，有照明弹时的命中值比没有时高，差值符合「先加 5 再乘系数」。
- **U2 探照灯的吸引火力与回避代价（#140）**：
  - `NightLights` 改记每一方的重抽次数（0、1、2）。夜战选目标时，抽中的目标不带探照灯就重抽，最多这么多次。
  - 带探照灯的目标，阵形夜战回避系数和固定回避都乘 0.2。
  - 测试：用桩随机数确认重抽次数和停下的条件；回避值比不带时低，且符合乘 0.2 的结果。
- **U3 设施种类表（#136）**：写生成脚本和生成的表，`target_class` 改为查表；查表没有结果、而名字匹配的，暂时保留名字匹配作为兜底，并写日志。
  测试：钉住各类条数；对照现在的名字匹配，列出分类有变化的敌舰，写进提交说明。
- **U4 软目标与通用倍率（#136）**：移植 `installModAll`、`softSkinMult`/`Day` 和 `installFlat`，接进昼战炮击和夜战的攻击力。
  测试：至少覆盖三式弾、WG42、大発加戦車、迫撃砲，各一个已知值。
- **U5 其余各组与上限后倍率（#136）**：炮台、离岛、北方（夜战加雷装）、港湾夏姬；补给基地、泊地、船坞、夏季战舰、夏季重巡的上限后倍率。
  测试：每组一个已知值；上限后倍率要乘在上限之后。
- **U6 裸装时的雷装（#136）**：夜战中，攻击方一件装备都没有、目标是陆上设施时，攻击力取素火力加素雷装，再加夜间触接定数，不用 U4、U5 的倍率，也不加潜艇对陆上设施的 +30（wikiwiki 的公式里没有）。
  测试：同一艘舰对同一设施，裸装时攻击力里有雷装，装一件任意装备后没有。
- **U7 しまね丸改夜间出击（#141 第 4 项）**：按来源 `AO` 的规则判断能否夜间出击，不再只看舰种。
- **U8 联合舰队护卫对潜（#141 第 7 项）**：昼战之后的夜战里，护卫舰队对潜艇造成实际伤害，条件照来源的 `!NBonly && !isescort`。
- **U9 夜战开局时敌方轻空母对潜（#141 第 6 项）**：用来源里使她能对潜的那个数值，代替码典里的对潜 0。
- **U10 空母雷装加成、陆上目标与特殊机体倍率（#141 第 2 项）**：移植 `getEquipBonusCVTorp`；对陆上目标不计雷装；特殊机体倍率照来源。
- **U11 多个特殊攻击依次判定（#141 第 3 项）**：按 `CV.prototype.NBtypes` 的顺序依次判定，而不是只判她的航空队组成的那一种。
- **U12 夜间瑞雲（#141 第 5 项）**：实现 `nightZuiunCIRate` 那条路径，以及对潜艇时烟幕的影响。
- **U13 敌方空母（#141 第 1 项）**：先确认我们有没有敌方装备的熟练度数据，有就做，没有就把缺的数据写进 #141，然后结束这个单元。
- **U14 文档**：每个单元更新它所属的 `docs/solutions/` 文档（`night-attack-types.md`、`carrier-night-air-attack.md`，以及对陆上设施的那一份），从 Known gaps 里移除已经完成的项。全部完成后关闭三条 issue。

U1 → U2 → U3 → U4 → U5 → U6 → U7 … U13。U7～U12 之间互不依赖，可以调整顺序。

## Risks

- **种子测试会漂移**：凡是改变随机数消耗的单元（U2 的重抽、U11 的依次判定），都会让后面的随机序列移位。处理方法是换种子，不放宽断言。
- **分类变化**：U3 把按名字判断换成查表，分类有变化的敌舰会改变它受到的攻击方式（包括潜艇能不能攻击、三式弾是否生效等）。提交说明里要列出这些舰。
- **工作量**：#136 一条大约等于计划 004 另外三部分的总和，U4、U5 是最大的两个单元。
