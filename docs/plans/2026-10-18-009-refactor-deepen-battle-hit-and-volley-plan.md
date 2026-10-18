---
title: "Deepen Battle Hit Landing and Volleys - Plan"
type: refactor
date: 2026-10-18
status: active
execution: code
product_contract_source: improve-codebase-architecture
---

# Deepen Battle Hit Landing and Volleys - Plan

`/improve-codebase-architecture` 对 `crates/emukc_battle/` 的审查。按近 60 个提交统计：改公式的修复只动一个文件（169ec40e、b3f6244b 只改 `damage.rs`）；改“谁打谁、打哪个 deck、谁算旗舰”的修复要动 3–12 个文件（d756a51c 改了 8 个阶段文件里的 13 处 `apply_damage` 调用，b7a9d94b、a9623045、7f22d4c4 各 3–4 个文件）。本计划把这两类知识各收进一个 module：命中落地收进 `Hit`，交战面收进 `Volley`。四个阶段之后，同一个 interface 背后是同样的行为，修复只改一处。

**范围**：U0–U4，一个单元一个 PR，按下文依赖顺序做。
**基线**：`main` `38e88b4e`。

## 不变量

- **纯重构，不改行为。** U1–U4 每个 PR 的现有 golden（`crates/emukc_battle/tests/golden/` 40 份、`tests/gameplay_tests/battle_golden.rs`）与 U0 新增的 golden 必须逐字节不变，不运行 `EMUKC_BLESS_GOLDEN`。需要重冻就说明这个单元改了行为：停下，把行为变化拆成单独的 fix PR（自己的计划、自己的说明），重构 PR 不夹带。
- **RNG 抽取顺序不变。** 命中、伤害、`apply_damage` 的保护抽取（只在致命伤时抽）、目标抽取的先后都保持原样；golden 就是这条的检查。
- 每个 PR 的质量门：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`，并说明 golden 没有变化。

## 非目标

- **昼战后夜战的 deck 选择留在 gameplay。** 审查里的候选 5 与 `2026-09-23-1116-refactor-deepen-map-sortie-battle-modules-plan.md` 的 KTD13 冲突：联合舰队布局由 `SortieBattleSession` 维护，分界从舰船标签现算。只有出现第三个需要同样切分的调用方（例如演习联合舰队夜战）时再重议。
- 公式类工单 #193、#196、#197、#200、#204：它们改 `accuracy.rs`、`damage.rs`，不属于本计划。和 U1 改同一批调用点时，先合先做，后者 rebase。
- `damage.rs:377` 的 `calculate_single_slot_airstrike_damage` 是 `#[expect(dead_code)]` 的重复，`kouku.rs` 用的是自己那份。本计划只记录，不删。
- 只按行数拆分 `night.rs` 不算 deepening，不做。
- `accuracy::roll_attack` 与 `Aim`、`combined_packet.rs`、`BattleRng`、`choice_with_protect` 已经 deep，不动。
- `simulation/night_attack_table.rs` 是生成文件，不手改。
- 开幕对潜“无熟练度”编码了两次（`asw.rs:39` 走 `Aim::in_the_opening()`，`:99` 传 `PlaneProficiency::NONE`）。记录在案，U1 不顺手合并。

## 依赖与顺序

```
U0 ──┬── U1 ──────────────┐
     ├── U2 ── U3          ├── U4c（决策门）
     └── U4a、U4b          ┘
```

建议顺序：U0 → U1 → U2 → U3 → U4a/U4b → U4c。U1 与 U2 互不依赖；U3 依赖 U2 的 `Volley`；U4c 要等 U1、U2 都完成后才看得清还剩多少差异。

## Implementation Units

### U0 扩大 golden 安全网（只加测试）

现有 golden 只覆盖 `BattleType::Normal` 单舰队的昼战与夜战起点。U2、U3 改的正是 deck、旗舰和索引逻辑，这张网看不到。

- `crates/emukc_battle/tests/golden_transcript.rs` 新增几组设置，每组种子 1..=20，只新增文件，现有 40 份不动：
  - 友方连合艦隊 × 三种（空母機動、水上打撃、輸送護衛）对敌方单舰队，昼战；
  - 友方单舰队对敌方连合艦隊（`ec_battle`），昼战；
  - 夜战：第2艦隊单独对敌方连合艦隊的一个 deck（`NightBattleInput` 直接传护卫 deck，与 `absorb_night` 之前 gameplay 交给本 crate 的输入一致）；
  - `execute_sp_midnight`；
  - 单舰队出击带两支基地航空队（只有单对单会出动基地航空队，所以放在这里）；
  - 演习昼战与夜战（`is_sortie = false`：`apply_damage` 截断伤害，显示值规则不同）。
- 建舰沿用 `simulation/mod.rs` `combined_tests` 的辅助；它们是 `#[cfg(test)]`，集成测试拿不到时，就在 `golden_transcript.rs` 里按同样写法建，不新增公共测试 API。
- 已知漏洞：`tests/gameplay_tests/battle_golden.rs` 是单舰队、不进夜战，昼战后夜战的 gameplay 拼接不在任何 golden 里。本计划不改 gameplay 的切分，所以只记录。
- **验收**：生成新 golden 要跑一次 `EMUKC_BLESS_GOLDEN=1`，它也会重写现有 40 份。跑完 `git status` 在 `tests/golden/` 下只能出现新文件、没有修改过的文件；之后不带 bless 重跑通过。

### U1 命中落地收进 `Hit`

**问题。** 11 处水上攻击（`shelling.rs` ×3、`special_attack.rs`、`night.rs` ×2、`torpedo.rs` ×4、`asw.rs`）和 2 处航空攻击（`kouku.rs`、`air_base.rs`）各自手写同一段：`apply_damage` → 记 `damage_dealt` → `display_damage` → 包成 `DamageCell`。这些副本已经不一致：
- `shelling.rs:270`、`special_attack.rs:647` 只在攻击方是友方时记伤害，鱼雷、对潜、夜战总记；
- `Shielded`/`Plain` 的 match 在 `shelling::damage_cell` 之外又内联了 5 次；
- `kouku.rs:623` 要靠注释提醒“别累加 raw”。

**决策。**
- **seam 放在“落地”这一步，不放在整次攻击。** 命中判定和伤害公式在各阶段确实不同：6 个伤害函数、夜战与特殊攻击夹着 `cut_in_hit`、鱼雷火力同时进 `Aim` 和伤害、航空用 `roll_strike`。把它们收进一个 `strike` 需要公式选择器，interface 会和 implementation 一样宽。落地这一步在 13 处完全相同，删掉它会让复杂度重现在 13 个调用者里，符合 deletion test。
- interface 大致是 `BattleRuntimeShip::take_hit(&mut self, rng, raw, shield) -> Hit`，`Hit` 带 `cell: DamageCell` 与 `dealt: i64`。名字实现时可以再定，形状不变。
- **伤害记账统一为无条件 `attacker.damage_dealt += hit.dealt`，留在调用方。** 唯一的读者 `outcome::calculate_mvp` 只读友方，所以对输出没有影响，还能删掉两个分支。记账不放进 `Hit`，因为调用方有三种形状：每次命中记一次、夜战按次累加后记一次、航空攻击没有攻击舰。
- `display_damage` 与 `shelling::damage_cell` 并入 `take_hit` 后删除，`apply_damage` 收到 `types` 模块内（`pub(super)`），`types/mod.rs` 的保护测试照旧直接测它。返回值类型实现时定名 `Landing`，避开 `HitOutcome::Hit`。
- 航空两处去掉 `if damage > 0` 守卫：`apply_damage(0)` 不抽 RNG，返回 `(0, 0)`（沉没舰提前返回，`effective = 0` 时达不到保护条件），显示值为 0，输出相同。golden 会核对这一点；若有差异，航空两处退回原写法，单独记录。
- 轰沉保护的判定（`is_flagship || !was_taiha_at_entry`）在 `runtime.rs:211` 与 `outcome.rs:68` 的校验里各写一遍，收成 `BattleRuntimeShip` 的一个私有方法，两处共用。
- 鱼雷火力每次命中算两次（`torpedo.rs` 算一次给 `Aim`，`calculate_torpedo_damage` 内部又算一次）。改为把算好的火力传进伤害函数。`torpedo_attack_power` 不抽 RNG，输出不变。

**测试。**
- `types/mod.rs` 的保护测试和 `targeting.rs` 的三个 `display_damage_*` 测试改为通过 `take_hit` 断言；interface 就是测试面。
- 阶段级的“保护下显示一致”测试（`simulation/mod.rs:694`、`night.rs:3421`）保留，它们检查的是阶段有没有接上 `take_hit`。

**文档**：`battle-damage-foundation.md`、`night-battle-sinking-protection.md` 改成指向 `take_hit`。

### U2 交战面收进 `Volley`

**问题。** 要知道“谁能被打”，得经过这些地方：
- 编排器的 range（`simulation/mod.rs:154-158, 268-272`）；
- `ShellingRound` 的三个 range；
- 阶段内再切片（`torpedo.rs:98`、`asw.rs:34`）；
- 四种候选函数和 `choice_with_protect`；
- 每个阶段各自把切片内索引抬回整支舰队：`append_turn`、`defender_at`、`fired_at + target_idx`。

deck 边界在本 crate 内有四种求法：`escort_deck_start`、`deck_candidates` 自己的 `position`、`CombinedLayout.escort_start`、`state.enemy_escort_start`。`combined-fleet-index-spaces.md` 的“切片编号：第三个坑”就是这种结构造成的。

**决策。**
- 每个阶段、每一方一个 `Volley`：开火方的 range 与被打方的 range；拆借用与只读切片（`split`、`attackers_in`、`targets_in`）、索引抬升（`attacker_at`、`target_at`）都由它负责。阶段函数不再自己写 range、不再自己抬索引。阵形和 `attacker_is_enemy` 仍由阶段自己传：它们不随 deck 变化，放进 `Volley` 只会加宽 interface。
- “谁打谁”的规则全部写在 `simulation/volley.rs`：开幕三阶段走 `Volleys::opening`，砲撃轮按形态有 `Volleys::whole`、`friendly_deck_round`、`enemy_combined_round`。b7a9d94b、a9623045、7f22d4c4 改的都是其中某一行；哪一轮何时打、落进哪个包槽仍由编排器决定。
- 开幕雷撃的“第1艦隊不开火”这类按舰跳过的检查，改为在 `Volley` 的开火 range 里体现。遍历顺序与 RNG 顺序不变：原来是整支遍历、遇到主力 deck 跳过，现在是只遍历护卫 deck，二者抽取序列相同。
- **旗舰：写成 `Volley` 的不变量，不加字段谓词。** 实现时逐处核对了“旗舰”的用法。`is_flagship` 字段由 `mark_flagship` 设给每个 deck 的首舰，轰沉保护读它。其余都按位置判断：昼夜 cut-in、特殊攻击、瑞雲的 `idx == 0` 指“这支 deck 的旗舰”；かばう 的 `target_idx != 0 || defenders[0].is_escort_deck()` 指“舰队旗舰”，即非护卫 deck 的首舰。两者都依赖“每段切片都从某支 deck 的首舰开始”，这条写进 `volley.rs` 的模块注释，`Volley::split` 在 debug 构建里断言它。改成字段谓词，需要在 `from_context` 和 `simulate_night` 里也给敌方标旗舰，还要改所有直接调用相位函数、自己建舰的测试，而输出不变，所以不做。
- 相位用的 deck 边界只留 `volley::escort_start` 一个函数（删掉 `targeting::escort_deck_start`）。`deck_candidates` 的边界保持只认 `enemy_deck`：它写的是来源对敌方 deck 的掷骰（0.39 / 0.35），换成认双方标签只是因为现在没有调用方会把友方两支 deck 一起传进去才等价。`CombinedLayout.escort_start` 与 `state.enemy_escort_start` 在建舰时从同一个向量算出一次，供编排器和包翻译使用，也不动。gameplay 的 `SortieBattleSession` 不动（见非目标）。
- `ShellingRound` 的三个 range 换成 `Volleys`。测试专用入口 `simulate_shelling_side` 保留：它只让一方开火，换成真实回合会让双方交替开火、改变这些测试的 RNG 序列；它是只供本模块测试的内部 seam。
- 实现时 `Volley` 由相位函数开头调用表函数取得，而不是由编排器传入：表只有一处，调用位置不影响 locality，还省得改 13 个直接调用相位函数的测试。

**测试。**
- `combined_tests`（`simulation/mod.rs:1083` 起）原样保留，作为验收。
- 新增表格测试，逐行断言“阶段 × 舰队形态 → 两个 `Volley`”，开幕三阶段和三种砲撃轮都覆盖。

**文档**：`combined-fleet-index-spaces.md` 的“切片编号”一节、`day-shelling-order.md`、`reconnaissance-and-escort-cells.md`（如涉及）改成按 `Volley` 写。

### U3 折叠敌我镜像（依赖 U2）

**问题。** `torpedo.rs` 有四块近乎相同的代码，每块约 55 行：友/敌 × 开幕/闭幕。差别只在能力判定、候选函数、阶段和主力 deck 跳过；阵形在每块里手动对调。`asw.rs:36-59` 也是这样。夜战用 `(friendly_form, enemy_form)` 元组和 `params.contact[0]` / `[1]` 区分两方（`night.rs:1514, 1561`）。

**决策。**
- 每个阶段写成“对每一方的 `Volley` 开火”，一个函数覆盖敌我两方；阵形、触接按开火方各建一次 `Aim` 传入，不再在每块里对调。
- **保留“开火方按阶段开始时的状态决定”。** 开幕雷撃里敌方按 `enemy_at_start` 快照开火，闭幕雷撃同理。折叠后先给两方各取快照，再依次开火：友方全部开完，再轮到敌方，这个顺序不变。
- 如果 U2 或 U3 改了 `NightBattleParams` 的字段，就在同一个 PR 里给测试加构造辅助，代替逐个修改 16 处手写字面量。字段不变就不加。

**实现时的范围。** 阵形按开火方各建一次 `Aim`，由 `Aim` 带进 `salvo`，和 `asw_turn` 的做法一样；不放进 `Volley`，因为它不随 deck 变化（见 U2）。夜战触接同理留在 `Aim`。只折叠雷撃：两个公开函数共用 `torpedo_phase`，敌我两方各走一次 `salvo`。两方在阶段开始时各取一份快照；友方原来读的是活舰，但友方开火期间它们只有 `damage_dealt` 会变，而计算不读它，所以结果相同。开幕对潜在 U2 之后已经是一个 `asw_turn` 服务两方。砲撃轮与夜战的两方回合本来就不对称：友方旗舰回合要先判断特殊攻击，夜战还有瑞雲；剩下的差别只是阵形和触接要按开火方交换，合并后 interface 不会变窄，所以不折叠。

**测试。** 现有鱼雷、对潜测试不改断言；它们已经分别覆盖敌我两方，不另做参数化。

### U4 昼夜共用的特殊攻击执行与战报记录

**问题。** 夜战重写了昼战已有的东西：
- 装备计数：`night.rs:157, 170` 是 `day_cutin.rs:105, 126` 的副本，昼战一侧也没有用 `targeting::is_main_gun_type`；
- 战报记录：夜战把 8 个 `api_*` 向量内联写了两次（`night.rs:1473-1480`、`1710-1732`），昼战已经有 `record_day_attack`（2026-09-23 计划的 KTD12）；
- 特殊攻击行：`special_attack_row`（`night.rs:1417-1481`）基本是昼战 `execute_special_attack`（`special_attack.rs:589-670`）换了 `Aim` 和伤害函数。

**单元。**
- **U4a**：`BattleNightHougeki::record_attack`，替换两处内联写入。可以在 U0 之后任何时候做。
- **U4b**：主炮、副炮计数昼夜共用一份，建在 `is_main_gun_type` / `is_secondary_gun_type` 上。可以在 U0 之后任何时候做。
- **U4c（决策门）**：U1、U2 完成后再比较 `special_attack_row` 与 `execute_special_attack`。剩下的差异如果只有 `Aim` 的构造和伤害函数，就合成一个执行函数，由调用方传入这两项。如果还有别的分叉（重抽次数、第二舰队能否かばう、记录器类型），合并后 interface 会和 implementation 一样宽，那就不合，把理由记进 `night-attack-types.md`。

**测试。** 夜战按攻击种类的测试保持不变；U4b 的计数函数直接沿用现有的昼战 cut-in 测试。

**文档**：`night-attack-types.md`、`carrier-night-air-attack.md` 只更新函数位置。

## 收尾

- U1、U2 落地时在 `GLOSSARY.md` 加「命中落地（Hit）」「交战面（Volley）」两条，只定义概念、不写实现；module 存在之前不加。
- 每个单元合并后，按 `/ce-compound` 把学到的东西写进对应的 `docs/solutions/` 文件，然后 `/code-review`。
- 全部完成后把本计划的 `status` 改为 `implemented`；U4c 不合并的话也算完成，理由写在文档里。
