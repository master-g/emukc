---
title: "Searchlight Accuracy, Carriers on Land Targets and Shielding - Plan"
type: feat
date: 2026-10-16
status: implemented
execution: code
---

# Searchlight Accuracy, Carriers on Land Targets and Shielding - Plan

计划 2026-10-15-004 核对时看到、没有做的三条（见该计划「实施偏差」最后一项），用户定为要做。三条都按钉住的 `kancolle-replay` 提交 `69097abc` 来做。

**范围**：下文 U1–U5。每个单元单独提交，可以合在一个 PR。
**非目标**：
- 空母昼战炮击对陆上设施的攻击力：来源只计能对地的艦爆的爆装（`CV.prototype.shellPower` 的 `installOnly`），但我们的空母炮击本来就是简化公式，这一项已列在 `anti-installation.md` 的 Known gaps 里。
- 夜战时空母对陆上设施的目标选择。
- 客户端演出核对。

**基线**：`main` `0d2fc03d`。

## Evidence

来源文件从 `raw.githubusercontent.com/KC3Kai/kancolle-replay/69097abc/js/` 取（`kcsim.js`、`kcships.js`、`kcEQDATA.js`）。仓库里不存副本。

### 探照灯命中 +7（`kcsim.js` `NBattack`）

- `:823` `if (searchlights[0]) acc += .07;`。
  - 位置在 `hitRate(...)` 之后，也就是已经乘过各项系数；同时在适合砲加成（`ACCfitN`）之后、PT 小鬼修正之前。
  - `searchlights[0]` 是攻击方自己这一方的 `light`。夜战开局时，这一方只要有一艘未沉的舰带探照灯就为真（`:1672-1680`）。
  - 双方对称：敌方攻击时，传入的是 `[light2, light1]`（`:1730`）。
- 特殊攻击走同一个 `NBattack`（`:1704`），同样加。
- 我们这边：`night.rs` 的 `NightLights.searchlight` 已经是这一方的重抽次数（大于 0 即点亮），目前只用于发动率（`:761`、`:1305`）和选目标，不进命中。

### 空母不炮击陆上设施（`kcsim.js` `shellPhaseTarget`、`kcships.js` `CV.canShellInstallation`）

- `kcsim.js:1105`：只在 `ship.hasDivebomber && !ship.canShellInstallation()` 时，从昼战炮击的目标里去掉陆上设施。
  - 去掉后没有目标就不攻击：`targets.length` 为 0 时 `result.target` 为空，`shellPhaseAttack` 直接返回。
  - 这个过滤只在昼战炮击里有。
- `hasDivebomber`（`kcships.js:461`）：装备里有 `DIVEBOMBER` 或 `JETBOMBER`，且这艘舰按空母方式炮击（`CVshelltype`）。只看有没有装，不看机数。
- `canShellInstallation`（`kcships.js:1982`）满足下面任一条件即为真：
  - 有一件 `canShellInstall` 的装备且机数大于 0；
  - 所有机数大于 0 的槽里都没有艦爆或喷气战斗轰炸机。
- `kcEQDATA.js` 里 `canShellInstall` 的有 14 件：64, 148, 233, 277, 305, 306, 319, 320, 391, 392, 420, 421, 474, 578。
  - 码典的 `slotitem_extra_info.json`（kcwiki `_can_attack_installations`）只标了前 7 件。
  - 缺的是 320 彗星一二型(三一号光電管爆弾搭載機)、391 九九式艦爆二二型、392 同(熟練)、420 SB2C-3、421 SB2C-5、474 F4U-4、578 SB2U-2。
  - 所以不能用 kcwiki 的标记，名单照来源。

### かばう 不涉及陆上设施（`kcsim.js` `choiceWProtect`）

- `:2309`：目标是陆上设施（`target.isInstall`）时直接返回，不掷护卫概率，也就不消耗随机数。
- `:2317`：陆上设施不做护卫者（`!targets[i].isInstall`）。
- 我们这边：`targeting.rs` 的 `select_escort_shield` 把陆上设施归在海面一类。旗舰是陆上设施时也会掷概率，陆上设施也能护卫。它有五个调用点：昼战炮击、雷击（双方）、夜战两处、特殊攻击，修函数本身就全部覆盖了。

### golden 的前提

`tests/gameplay_tests/battle_golden.rs` 是 1-1 的第一战：不进夜战，敌方也没有陆上设施。U1–U3 预计都不会改动它。

可能变化的是 `crates/emukc_battle/tests/golden/` 里固定种子的昼战和夜战 golden，以及固定种子的测试：
- U1 改命中，进而改变后续的随机序列；
- U3 在旗舰是陆上设施时少掷一次随机数。

处理方法沿用计划 004：种子测试结果变了就换种子，不放宽断言；golden 真变了，就有意重新 bless，并在提交里写明是哪一条规则、多少个种子变了。

## Implementation Units

每个单元都先写测试，做完跑 `cargo test -p emukc_battle`、`cargo test --test gameplay_tests battle_golden`，以及完整的三项质量门。

- **U1 探照灯命中（`night.rs`、`accuracy.rs`）**：攻击方自己这一方点亮探照灯（`own.searchlight > 0`）时，夜战命中在乘完系数、加完适合砲之后再加 7（百分点），位置照来源。普通夜战攻击和夜战特殊攻击都要加。
  测试：同一攻击者、同一目标，自己这一方有探照灯时比没有时正好高 7。敌方的探照灯不影响这个值。
- **U2 能对地的艦爆名单（生成脚本）**：让 `scripts/gen_installation_types.py` 同时读 `kcEQDATA.js`，在 `installation_types.rs` 里生成 `INSTALLATION_BOMBERS`（上面那 14 件）。条数写进 `the_installation_tables_follow_the_source`。
- **U3 空母不炮击陆上设施（`shelling.rs`、`targeting.rs`）**：昼战炮击选目标时，满足以下两条的舰从候选里去掉陆上设施：
  - 按空母方式炮击，并且装着艦爆或喷气战斗轰炸机（不看机数）；
  - 没有一件机数大于 0 的 `INSTALLATION_BOMBERS`，同时至少有一件机数大于 0 的艦爆或喷气战斗轰炸机。

  去掉后没有目标就不攻击，也不消耗随机数。

  测试：
  - 带 99 艦爆的空母面对陆上设施加驱逐时只打驱逐；
  - 只剩陆上设施时不攻击；
  - 换成彗星一二型甲(64) 或 SB2C-3(420) 后会打陆上设施；
  - 99 艦爆机数为 0、另带艦攻时也会打陆上设施。
- **U4 かばう（`targeting.rs` `select_escort_shield`）**：
  - 被选中的旗舰是陆上设施时，直接返回 `None`，在掷概率之前返回；
  - 护卫者候选里去掉陆上设施。
  - 「海面一类护卫海面一类」那条注释里关于 installation 的部分随之改写。

  测试：
  - 陆上设施旗舰从不被护卫，并且不消耗随机数（用桩随机数确认掷骰次数为 0）；
  - 满血的陆上设施僚舰从不当护卫者；
  - 普通旗舰仍照旧被护卫。
- **U5 文档**：删掉 `night-attack-types.md` 和 `anti-installation.md` 里 Known gaps 中对应的三条。把规则写进这两份文档的正文：
  - 探照灯写进夜战命中一节；
  - 艦爆和かばう 写进对陆上设施一节，并写明能对地的名单照来源，不用 kcwiki 的标记。

  然后在计划里补「实施偏差」，把 `status` 改为 `implemented`。

U1 与 U2–U3、U4 互不依赖。U3 依赖 U2。U5 最后做。

## Risks

- **种子漂移**：U1 改变命中判定的结果，U4 在旗舰是陆上设施时少掷一次随机数，这两处都会让后续的随机序列移位。U3 也会改变带艦爆的空母对陆上设施时的出手与随机数消耗。
- **名单与 kcwiki 不一致**：码典 `can_attack_installations` 仍只有 7 件，别处要是读它，会和战斗里的判定不一致。目前除解析器外没有读它的地方。本计划不改它，只在 `anti-installation.md` 里写明。

## 实施偏差

- 实际基线是 `main` `194d11b3`（#169 合并之后），不是 `0d2fc03d`。
- U2 与 U3 合在一个提交：`INSTALLATION_BOMBERS` 单独提交时没有使用者，`clippy -D warnings` 报 dead_code。
- 计划写的「彗星一二型甲(64)」名字错了：`kcEQDATA.js` 与码典里 64 都是 Ju87C改。id 照用，测试按 Ju87C改 命名。
- U1 的测试在 `roll_attack` 这一层，用固定骰子找出命中边界，比较有灯和无灯。`Aim` 只带攻击方自己这一方的探照灯，`night_turn` 传的是 `own`，所以敌方探照灯不进这个值，测试不再单独断言。特殊攻击的行在旗舰那一回合取己方探照灯，这时夜战刚开始，和开局时的取值相同。
- U3「按空母方式炮击」用现有的 `is_cv_type`（CV/CVL/CVB）。来源还把带艦攻或艦爆的补给舰（速吸改）算在内，这里没有算，和昼战炮击公式的口径一致。
- golden：U1 让夜战 20 个种子里的 seed 12 变了，已重新 bless。golden 的敌方山城改二带着探照灯，第二行的敌方攻击由未命中变为命中，后面的随机序列随之移位。U3、U4 没有改动任何 golden，`battle_golden.rs` 也没变。
