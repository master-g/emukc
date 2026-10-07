---
title: "Map LoS Routing: 判定式(33) - Plan"
type: fix
date: 2026-10-06
status: implemented
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Map LoS Routing: 判定式(33) - Plan

## Goal Capsule

让路由里的索敌条件真的起作用：运行时按 判定式(33) 算索敌得分，用谓词自带的分岐点系数。
本计划只管「得分怎么算、怎么比」。带系数的谓词从哪来，是计划 `2026-10-06-003`（路由规则转换器）的事。

> 2026-10-06 重排：原先的系数资产、7-5 阈值订正、路由对拍与按差异订正四个单元，
> 在找到带系数且 MIT 许可的规则来源之后被计划 003 取代，已从本计划删除。

## Product Contract

### Problem Frame

1. **比较的量错了。** 资产里所有 LoS 谓词的 `formula` 都是 `null`，
   `FleetRouteContext::los_by_formula(None)` 返回 `los_total`——舰队 `api_sakuteki[0]` 的裸合计。
   阈值却是 33 式得分。2026-09-22 真实账号快照里第一舰队裸合计 390、4 艘的第二舰队 147；
   2-5 去 boss 的门槛是 49。所以「索敌 ≥ N」恒真、「索敌 ≤ N」恒假。
2. **现有的「式3」也不是 33 式。** 它对所有装备统一乘 0.6，没有分岐点系数，没有改修加成，
   也没有任何数据引用它。
3. **阈值是整数，得分是小数。** 「28 未満 / 28 以上」在资产里是 `Lte 27` / `Gte 28`，
   27.5 会落进缝里一条规则都不匹配。

### 已核实的前提（不要再查一遍）

- **公式**（Fandom 艦これ検証Wiki「マップ索敵」，2018-09 停更）：

  ```
  得分 = Σ√(各舰素索敌) + Cn × Σ( Cl × (装备索敌 + 改修加成) ) − ⌈0.4 × 司令部Lv⌉ + 2 × (6 − 舰数)
  ```

  | 装备类别 | Cl | 改修加成 |
  | --- | --- | --- |
  | 水上偵察機 | 1.2 | 1.2 × √★ |
  | 水上爆撃機 | 1.1 | 1.15 × √★ |
  | 艦上偵察機、艦上偵察機(II) | 1.0 | 1.2 × √★ |
  | 艦上攻撃機（含夜間攻撃機） | 0.8 | 无 |
  | 小型電探 | 0.6 | 1.25 × √★ |
  | 大型電探、大型電探(II) | 0.6 | 1.4 × √★ |
  | 其余全部 | 0.6 | 无 |

- **有一份在维护的参考实现**：羅針盤シミュ `compass_dev` 分支的
  `src/logic/seek/{equip,equipBonus,fleet}.ts`（MIT）。上表停更于 2018 年，系数与改修加成以这份实现为准
  逐项核对；它还处理了装备ボーナス。
- 本项目的 `los_now`（`api_sakuteki[0]`）= 按等级算出的素索敌 + 装备索敌之和
  （`codex/ship.rs:282`、`:317`），不含装备ボーナス。所以 `los_now − Σ装备索敌` 就是素索敌。
- `slot_item` 实体有 `level`（★），`type3` 已经在 `build_fleet_route_context` 里取到。
- `los_total`/`los_formula1`/`los_formula3` 只在 `map_route.rs` 与 `sortie/route.rs` 两个文件里出现。
- 分岐点系数每张图一个值：2-5 是 1；4-5、5-2、5-4、5-5 是 2；1-6、6-2、6-3、6-5 是 3；
  3-5、5-6、6-1、7-2、7-4、7-5 是 4。这张表只用来写测试；运行时的系数来自谓词。

### Key Decisions

- **KD1 系数跟着谓词走。** `RoutePredicate::LoS` 的 `coefficient: Option<i64>`（字段由计划 003 的 U3 加）。
  不建地图级的系数表：来源代码里系数就写在每个条件上。
- **KD2 只有一个索敌得分。** 删掉 `los_total`、`los_formula1`、`los_formula3` 与 `los_by_formula` 的字符串分派。
  谓词的 `formula` 字段不再有读者，随旧资产一起在计划 003 的 U6 消失；在那之前留在 schema 里不读。
- **KD3 上下文存两个与地图无关的量：** 舰船项（`Σ√素索敌 − 司令部补正 + 艦数补正`）与装备项
  （`Σ Cl × (索敌 + 改修)`）。得分 = 舰船项 + 系数 × 装备项，在求值时算。
- **KD4 比较前对得分向下取整**，使 `Lte N-1` 等价于「N 未満」、`Gte N` 等价于「N 以上」。
- **KD5 `coefficient` 为 `None` 时求值为 `SourceUnknown`**，沿用现有的降级路径，不猜系数。
- **KD6 装备ボーナス先不做。** 本项目的舰船数值里没有装备ボーナス这一层，单为索敌加一个不成体系。
  在代码里用 `// ponytail:` 写明：将来舰船数值有了ボーナス，它应进素索敌（根号内）。

### Requirements

- R1 得分按 33 式计算，Cl 与改修加成与参考实现逐项一致。
- R2 系数取自谓词；缺失时按 KD5 降级，不 panic、不取默认值。
- R3 小数得分不会落进整数阈值之间。
- R4 用一支手算过的舰队固定得分，防止系数被无意改动。

### Acceptance Examples

- AE1 得分 40（系数 1）的舰队不满足「49 以上」。修复前它恒被判为通过。
- AE2 得分 27.5 的舰队匹配 `Lte 27`（即「28 未満」）。
- AE3 同一支舰队，系数 4 下的得分比系数 1 下高出 3 × 装备项。

### Scope Boundaries

- 不产出、不订正任何规则或阈值（计划 003）。
- 不做随机带的概率形状；不做遊撃部隊（7 艘）的艦数补正，现有的 `max(0)` 截断保持原样并留注释。
- 不做戦闘時索敵（触接、弾着）——那是另一套公式。

#### Deferred to Follow-Up Work

- 司令部补正的系数：Fandom 记录了 3-5 G 与 6-3 H 的反例（6-3 H 实测 0.33～0.35），标为要検証。先按 0.4。
- 装备ボーナス（KD6）。

## Implementation Units

### U1. 33 式得分与求值（行为修正）

- **Dependencies:** `RoutePredicate::LoS.coefficient` 字段已存在（计划 003 的 U3）。
- **Files:** `crates/emukc_gameplay/src/game/sortie/route.rs`、`crates/emukc_gameplay/src/game/map_route.rs`
- **Approach:**
  1. `build_fleet_route_context` 里把每件装备的贡献算成 `Cl × (api_saku + 改修加成)`。类别判定用已取到的
     `type3`，一个 `match` 即可，不建配置。前提表按日文类别名列出，实现时逐行对到
     `api_mst_slotitem_equiptype` 的 id 并写进分支注释——艦上偵察機(II)、大型電探(II) 有各自独立的 id，
     夜間攻撃機 要确认是独立 id 还是归在艦攻下。以参考实现的 `equip.ts` 为准，不要猜。
     取装备时把 `level` 一并带出来。
  2. `FleetRouteContext` 用 `los_ship_term` 与 `los_equip_term` 替掉原来三个字段。
  3. `route_predicate_matches` 的 LoS 分支：`floor(ship_term + coefficient × equip_term)` 与阈值比较；
     `coefficient` 为 `None` 返回 `SourceUnknown`。
  4. `route_predicate_key` 把 `coefficient` 也序列化进去，否则同阈值不同系数的两条规则会被并成一组。
- **Execution note:** 先写 R4 的手算测试并确认它在现有代码上失败，再改实现。
- **Test scenarios:**
  - R4：固定一支舰队（各舰素索敌、装备、★、司令部 Lv），手算得分写在测试注释里，断言到小数点后两位。
  - AE1、AE2、AE3 各一条。
  - 改修加成：同一台小型電探 ★0 与 ★9 的得分差等于 `系数 × 0.6 × 1.25 × 3`。
  - 艦攻走 0.8、水偵走 1.2、水爆走 1.1、主砲走 0.6。
  - 少于 6 艘的艦数补正；空舰队不 panic；`coefficient: None` 返回 `SourceUnknown`。
  - 现有的 `los_formula_*` 单测随 KD2 重写或删除，不保留断言旧行为的测试。
- **Verification:** `cargo test -p emukc_gameplay`、`cargo test --test gameplay_tests`。
  集成测试里若有依赖「索敌恒通过」才能到 boss 的，逐条判断是舰队本来就该被拦还是新实现算错了——
  不要靠给测试舰队塞水偵让它变绿而不说明。

### U2. 与参考实现对数

- **Dependencies:** U1
- **Approach:** 一次性的核对，不留工具：取 `z/snapshot/2026-09-22/port_live.json` 的第一舰队，
  用参考实现（bun 直接 import `src/logic/seek/fleet.ts`）与本实现分别算系数 1～4 下的得分。
  两边应只差装备ボーナス那一项；把差值与归因写进 U3 的文档。若差值不能用ボーナス解释，回到 U1。
- **Verification:** 文档里有四个系数下两边的得分与差值。

### U3. 收口

- `docs/solutions/logic-errors/` 记一条：症状、三层原因、为什么 183 条规则静默失效这么久
  （`formula: None` 回退到裸合计，被单测固定成了「向后兼容」）。
- `docs/map/data-dependencies.md` 补一句索敌得分的算法出处。
- `PROJECT_MEMORY.md` 回写，删掉「索敌分歧形同虚设」那条。

## 实施记录（2026-10-06）

U1–U3 已实施。U1 与计划 003 的 U4 同一次提交。U2 的对数结果（两支真实舰队、四个系数下与参考实现差值均为 0.00）
写在 `docs/solutions/logic-errors/los-routing-compared-raw-sum-2026-10-06.md`；对数用的是按本公式写的一次性脚本，
Rust 实现由 `fleet_los_terms_follow_formula_33` 固定同一张系数表。两支舰队都没有带装备ボーナス的装备，KD6 的差异项未被触发。

KD6 的影响量（2026-10-06 按钉住的来源 `src/data/equipBonus.ts` 估算）：来源收了 28 种装备、60 条索敌ボーナス规则，
单条 +1～+6；一件装备的全部规则相加最高 +18（Fairey Seafox改，其中 4 条是一次性且限定舰），常见的是 SG レーダー对美国舰
+4、紫雲(熟練)/Walrus +5、熟練見張員 +3。ボーナス在根号内，单舰得分增量是 `√(s+b) − √s`：`s=40, b=4` 为 0.31，
`s=50, b=10` 为 0.67，`s=20, b=10` 为 1.01，`s=50, b=18` 为 1.18。一支舰队通常只有一两艘带这类装备，偏差在 0.3～1.5 分；
六艘全带 +10 的极端情况约 4 分。方向固定：本项目的得分偏低，贴着阈值的舰队会被判成不过关。数据源是现成的
（同一份来源，条件用到的国籍在它的 `src/data/ship.ts` 的 `na` 字段），落点应是 `codex/ship.rs` 的属性组装，
让 `api_sakuteki` 带上ボーナス，路由得分随之自动正确。

## Sequencing

三份计划合起来的顺序（每一步都能独立通过质量门）：

1. 计划 003 的 U1–U3：取源、解析、normalize，加上 `LoS.coefficient` 字段。
2. 本计划 U1 与计划 003 的 U4 **同一次合入**：公式修好的同时换上带系数的规则。
   分开合的话，中间状态是所有索敌条件因 `coefficient: None` 退化成随机。
3. 本计划 U2、U3；计划 003 的 U5（对拍）。
4. 计划 001 的 U1–U4（KCNav：掉落、敌方编成与等级）。
5. 计划 003 的 U6–U7（wikiwiki 数据链退役），计划 001 的收口。

## Verification Contract

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -W warnings`、`cargo test --workspace`，以退出码为准。
- `battle_golden.rs` 走 1-1，没有索敌条件，不应因本计划变化。

## Definition of Done

- 索敌条件按 33 式得分判定，AE1–AE3 有测试。
- 代码里不再有 `los_total` 参与路由判定。
- 与参考实现的得分差只剩装备ボーナス，且已记录。
