---
title: "The Carrier Night Air Attack and Its Proficiency - Plan"
type: feat
date: 2026-10-11
status: implemented
execution: code
---

# The Carrier Night Air Attack and Its Proficiency - Plan

## Problem

计划 `2026-10-10-004` 让舰载机熟练度进了开幕航空战与空母昼战炮击，留下两处：夜间航空攻击、飞机做的对潜攻击
（`battle-hit-and-critical-rolls.md`「Not modelled」第一句）。

查夜战时发现熟练度之下还缺一层：我方的夜战伤害对所有舰都是「火力 + 雷装 + 5」。来源里空母的夜间航空攻击有自己的
威力公式，按夜间机的装备值与机数算。在错的基数上乘熟练度没有意义，所以两件事一起做。

## Evidence

**我方现状：**

- `targeting.rs` `can_attack_night_ship`：空母夜战出手的条件是「要员（258、259）或 `EXEMPT_NIGHT_CV_IDS` 四艘」加
  「带任何航空战机种」。不看是不是夜间机，不看血量。其它空母夜战不出手。
- `simulation/night.rs`：空母只有切入一种专门处理（`is_cv_night_ci_eligible` → `detect_carrier_night_ci_sub_type`）。
  夜间机的识别已有：图标 45 / 46 / 58，以及按 ID 的光電管彗星 320、岩井爆戦 154、Swordfish 242–244。
  没组成切入的空母走 `NightAttackType::Normal`。
- `damage.rs` `calculate_night_damage`：`火力 + 雷装 + 5 + 改修 + 夜偵`，不分舰种。
- `night.rs` 943：`api_n_mother_list` 恒为 0。客户端读它（`main.decoded.js` 124720，`0x1 == …` 作为攻击记录的第 8 个
  构造参数）；它在演出里的作用还没有读。
- `accuracy.rs` `roll_attack`：熟练度只在 `AttackKind::Shelling` 下取，`Night` 与 `Asw` 传 `PlaneProficiency::NONE`。
- 夜战打潜艇：必中、擦伤（`night.rs` 里的 `ponytail:` 注释），没有对潜公式。

**参考（`KC3Kai/kancolle-replay` 提交 `69097abc`）：**

- 资格，`kcships.js`：
  - `hasNBAirGear`（2060）：（夜間要員 或 舰自带）且（夜間戦闘機 或 夜間攻撃機 或 夜間爆戦）。
    光電管彗星、岩井爆戦、Swordfish 属于另一类（`B_NIGHTBOMBER2`），单有它们不算。
  - `canNBAirAttack`（2057）：（`hasNBAirGear` 或 Ark Royal 级带 Swordfish）且血量过半；装甲空母是超过四分之一（2122）。
  - `CV.canNB`（1952）：（舰有 `nightattack` 且血量超过四分之一）或 `canNBAirAttack`。
  - 自带要员的我方舰（`kcSHIPDATA.js`）：545、599、610、883（我方已有的四艘），另有 1008 しまね丸改、
    1036 Independence Flight II。しまね丸改不是空母，靠 `canAirAttackNB` 取得同一套函数（2165）。
  - `nightattack` 的我方舰：Graf Zeppelin 432 / 353、Saratoga 433、大鷹改二 529、神鷹改二 536、加賀改二護 646、
    雲鷹改二 889、Lexington 966 / 735。它们夜战用普通公式出手，不是航空攻击。
- 威力，`CV.NBPower`（2024）：舰的素火力 + 素雷装；每个夜间机槽（四类）加
  `火力 + sqrt(改修)`，艦攻再加雷装（对陆上不加），非艦攻加爆装；除 `B_NIGHTBOMBER2` 一类外再加 `机数 × 3`；
  最后加 `系数 × sqrt(机数)`，系数是 `0.3 × (火力 + 雷装 + 対潜 + 爆装)`，非 `B_NIGHTBOMBER2` 再乘 1.5。
  Ark Royal 级没有夜间机而带 Swordfish 时：每架 Swordfish 加 `火力 + 雷装 + sqrt(改修)`。
  之后与普通夜战一样：加夜偵、乘切入倍率、上限、弹药（`kcsim.js` 876）。
- 熟练度，`kcsim.js` 869–874：`canNBAirAttack` 时用昼战同一组三个值（命中、暴击率、暴击伤害），否则不用。
- 报文，`kcsim.js` 903：`api_n_mother_list` 在不是连击且 `canNBAirAttack` 时为 1。
- 切入，`kcships.js` 2064：空母夜战切入要求 `canNBAirAttack` 且 `hasNBAirGear`。
- 对潜，`kcsim.js` 990–1000：`usePlaneProf = planeasw 且 非先制 且 不是正规空母、不是補給艦`。
  `planeasw` 的舰种：航巡、航战、水母（1897、1929、2144）与轻空母（2091）。同一组三个值。

## Decision

1. **夜间航空攻击的资格**照来源：要员或自带（现有四艘加 1036；しまね丸改见 Scope），带夜間戦闘機、夜間攻撃機或夜間爆戦
   之一且该槽有飞机；或 Ark Royal 级带有飞机的 Swordfish。血量过半，装甲空母超过四分之一。
2. **空母夜战出手**改为：能做夜间航空攻击，或者是 `nightattack` 的九艘且血量超过四分之一。
   后者用现有的普通夜战公式（用户已定）。这让 Graf Zeppelin 等九艘开始在夜战出手，也让「带要员和普通艦攻、
   没有夜间机」的空母不再出手。
3. **夜间航空攻击的威力**用 `CV.NBPower` 的公式取代「火力 + 雷装 + 5 + 改修」那一段，其后的夜偵、倍率、上限、弹药不变。
   素火力、素雷装取舰的数值减装备的数值。
4. **熟练度**：夜间航空攻击的命中抽签与暴击伤害带上该舰的 `plane_proficiency`。仍是一次攻击一次抽签。
5. **空母夜战切入**的资格加上 Decision 1（原先不看血量）。
6. **`api_n_mother_list`**：夜间航空攻击且不是连击时为 1。实施时先读客户端对这个标记的用法，再定是否需要补资源或场景。
7. **昼战对潜的熟练度**：航巡、航战、水母、轻空母在非先制的对潜攻击里带上 `plane_proficiency`，命中与暴击伤害都算。
8. **没有夜间机、没有熟练度的舰，抽签与结果逐位不变。**

## Scope

**不做：**

- 夜战对潜的公式（现在是必中擦伤）；因此 Decision 7 只管昼战。
- しまね丸改（補給艦）的夜间航空攻击：我方的空母判断都按舰种，要为一艘舰另开一条路。记入已知差异。
- 装备加成里空母雷装的那两项（`eqBonusCV`）、对陆上目标不加雷装的分支、夜间航空攻击的特殊机加成（`bonusSpecialP`）。
- 敌方空母的夜战：敌舰由 manifest 构造，没有装备槽，资格判断对它们恒为假；`nightattack` 的深海舰不在这次范围。
- 带艦攻/艦爆的補給艦昼战对潜（计划 004 已注明我方只认 CV / CVL / CVB）。

## Implementation Units

- **U1** `night.rs` / `targeting.rs`：`can_night_air_attack`；`can_attack_night_ship` 按 Decision 2；
  切入资格按 Decision 5。测试：要员加夜戦可以、要员加普通艦攻不行、只有光電管彗星不行、Ark Royal 加 Swordfish 可以、
  中破不行而装甲空母中破可以、Graf Zeppelin 出手但不是航空攻击、自带要员的四艘加 1036。
- **U2** `damage.rs`：夜间航空攻击的威力。测试：对一艘装备固定的空母手算一个值（含 `B_NIGHTBOMBER2` 不加机数项、
  艦攻加雷装、改修的开方），以及 Swordfish 那一条；非空母的夜战威力不变。
- **U3** `accuracy.rs` 与 `night.rs`：`Aim` 带上「夜间航空攻击」，`roll_attack` 与夜战伤害取熟练度；
  `api_n_mother_list`。测试：同一抽签下满熟练度把普通命中变成暴击且伤害更重；无熟练度时抽签不变；
  标记在航空攻击时为 1、连击与非空母时为 0。
- **U4** `accuracy.rs` 与 `damage.rs`：昼战对潜的熟练度（Decision 7）。测试：带满熟练度水爆的航巡对潜暴击更重，
  先制对潜与驱逐舰不变。
- **U5** 客户端：读 `api_n_mother_list` 的用法；场景里加一个夜间空母的预设进 sim→validate 门，能跑无头检查就跑。
- **U6** 文档：`battle-hit-and-critical-rolls.md`（「Not modelled」、熟练度一节）、夜战相关的沉淀文档、
  `PROJECT_MEMORY.md`；基线若有变化则重新冻结并说明。

## 实施记录（2026-10-11）

- U1–U6 做完。与计划不同的几处：
  - 夜战按炮击出手的空母是 11 艘：来源的九艘加 Wasp、Wasp改。后两艘的出处是 wikiwiki 的 Wasp 页
    （「Intrepidと異なり火力初期値が1以上のため夜間砲撃を行う。改も同様」，2026-10-11 取），客户端自己的名单与之一致
    （`_getNormalAttackType` 夜战版里按炮击演出的空母），来源的提交比它们早。
  - 素火力、素雷装没有用「舰的数值减装备的数值」，改为主数据加近代化改修：`api_karyoku` 里还并着装备加成，
    减装备值减不干净。
  - 对潜的舰种多了揚陸艦（来源 2207 行同样是 `planeasw`）。
  - Decision 6 读完了客户端：标记为 1 时按舰载机演出（`PhaseAttackKansaiki`），不需要补资源。
- 现有 340 条战斗测试里只有一条断言旧规则（带要员和普通艦爆的轻空母能在夜战出手），按新规则改写。
  没有带种子的测试移位，没有换种子；两类基线都没有变化。
- 新测试：`night.rs` 5 条、`accuracy.rs` 1 条、`damage.rs` 1 条；预设 `night_air_attack` 进了 sim→validate 门，
  夜战那道门断言「这个预设有航空攻击、其它预设都没有」。
- 质量门：fmt 与严格 Clippy 通过；`cargo test --workspace --no-fail-fast` 1328 过、0 败、0 ignored。
- 无头检查 `night_air_attack` 通过：2-1 打进夜战，我方三次出手的 `api_n_mother_list` 都是 1、`api_sp_list` 都是 6
  （三架夜戦组成戦他他的切入），无页面错误、无缺失资源。那一场里旗舰没有轮到出手，所以带夜攻的那一艘没有演到；
  没有组成切入的普通夜间航空攻击也没有在客户端里演到。

## Risks

- **现有夜战测试会动。** `night.rs` 里有十几条空母夜战切入的测试，资格加了血量与夜间机的条件、威力换了公式，
  带种子的断言可能落空。落空时换种子，不放宽断言；断言的若是旧规则本身（例如「带要员就能出手」），按新规则改写并在计划里记下。
- **基线。** `crates/emukc_battle/tests/golden/` 的夜战转储若含空母出手会变；`battle_golden.rs` 只冻结 `fresh_1_1` 第一战，预期不变。
- **Decision 2 改变了谁能出手**，是这份计划里对玩家可见面最大的一处，所以单列出来等确认。
- **素火力的取法。** 我方舰的 `api_karyoku[0]` 含装备值，减装备值得到的「素火力」含近代化改修，与来源的 `statsBase` 一致；
  装备加成（`gear_bonus`）若已并进 `api_karyoku`，要确认减的时候没有少减。实施时核对。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、
  `cargo test --workspace --no-fail-fast`。
- sim→validate 门里的夜间空母预设；无头检查（若该预设能稳定进夜战）。
