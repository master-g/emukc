---
title: "Expedition Landing Craft Bonus - Plan"
type: feat
date: 2026-10-14
status: implemented
execution: code
---

# Expedition Landing Craft Bonus - Plan

Issue #119。远征的资源报酬只有基本量（大成功 ×1.5），舰队带的大発系装备、鬼怒改二不加成。验收条件在 issue 的 Brief 里。
这是 gameplay 数值改动，按 Balance Defaults Policy 处理（不动 `Default` impl，但规则 1–5 照做，见 Verification）。

**基线**：PR #126（`8af3b9de`）已合并，分支 `feat/expedition-landing-craft` 从 `main` 出发。下面的行号取自 `a29ad9e3`，之后 `5cc74319` 让它们偏移了几行，以函数名为准。

## Evidence

- **现状** `calculate_expedition_resource_reward`（`game/expedition.rs:879`）：成功给 `resource_reward`，大成功逐项
  `((amount as f64) * 1.5).floor()`，失败 `[0; 4]`。唯一调用方是 `complete_expedition` 成功分支（`:284`），结果交给
  `grant_expedition_rewards` 发放，并写进 `ExpeditionCompletion.resource_reward`（即客户端的 `api_get_material`）。
- **读装备的现成写法** `count_drum_canisters`（`:567`）：取六个槽（含 `slot_ex`）的 id，一次 `slot_item::Entity::find()`
  查出，按船归属计数。`slot_item::Model` 有 `mst_id`、`level`（★）。
- **测试入口**：`crates/emukc_gameplay/tests/expedition.rs`（`expedition_type1_great_success_applies_reward_multipliers`
  断言 `[75, 150, 75, 75]`，不带大発，改后仍成立）；`Ctx::add_slot_item(profile_id, mst_id, stars, alv)`、
  `Ctx::set_slot_item`（`game/ship/mod.rs:213`）可直接给船装上带★的装备。`expedition.rs` 底部已有 `#[cfg(test)]` 单元测试。
- **码典里的装备**：下表 17 个 id 与 487 鬼怒改二在 `.data/codex/start2.json` 都存在。0% 的 230、355、482、494、495、514
  与 5% 的 68、193 同属 `api_type[2] == 24`，所以**不能按装备类型推导**，只能按 id 列表。

### 公式来源与交叉核对

采用 **KC3Kai**（`KC3Kai/KC3Kai@6b0534d2`，`src/library/objects/Fleet.js` 的 `calcLandingCraftInfo`）的系数，
**fandom** `Expedition/Introduction`（「Extra bonuses to expedition incomes」）的取整顺序：

```
报酬 = ⌊R·S⌋ + ⌊R·S·(B1 + B★)⌋ + ⌊R·S·B2⌋
```

- `R` 基本量，`S` 成功 1、大成功 1.5（`R·S` 不先取整）。
- `B1 = min(20%, Σ 各装备素补正 + 鬼怒改二 5%)`。
- `B★ = B1 × ★平均 / 100`。★平均只算素补正 > 0 的装备（鬼怒改二不进分母）。
- `B2 = min(cap, 2% × 特大発(193)个数)`。cap 按特大発个数与**大発動艇(68)个数**查表：

  | 特大発 \ 大発(68) | 0 | 1 | 2 | 3 | ≥4 |
  |---|---|---|---|---|---|
  | ≤3 | 5.0% | 5.0% | 5.2% | 5.4% | 5.4% |
  | ≥4 | 5.4% | 5.6% | 5.8% | 5.9% | 6.0% |

  特大発 1、2 个时 cap 不起作用（2%、4%）。

素补正（与 wikiwiki「大発動艇」页的装备表逐 id 一致）：

| 素补正 | mst_id |
|---|---|
| 5% | 68 大発動艇、193 特大発動艇、526 特四式内火艇改 |
| 4% | 525 特四式内火艇 |
| 3% | 409 武装大発、576 大発動艇(R35&フランス兵) |
| 2% | 166 大発(八九式)、449 特大発+一式砲戦車、436 大発(II号戦車)、408 装甲艇(AB艇) |
| 1% | 167 特二式内火艇 |
| 0%（不计入★平均） | 230、355、482、494、495、514 |

交叉核对：

- **wikiwiki**（「特大発動艇」页 資源獲得ボーナス）：cap 表、`B2` 单独取整一致；系数表一致；「最大改修」列（大発 0.5%、
  八九式 0.2%、特二式 0.1%）只有 `B★ = B1 × ★/100` 能解释，确认 KC3Kai 的★项。它写的 `0.2% × ★平均` 是 `B1 = 20%`
  时的特例。取整它写成两段 `⌊R·S·(1+B1+B★)⌋ + ⌊R·S·B2⌋`，与 fandom 的三段只在 `R·S` 带 .5 时差 1（见 Risks）。
- **kcanotify**（`antest1/kcanotify@874e414f`，`KcaExpeditionCheckViewService.calculateBonusValue`）两处不同，不采用：
  ★项固定 `0.002 × ★平均`，不随 `B1` 缩放；cap 的列按 `min(4, 68+166+167 个数)` 取，而不是只数 68。它的装备表也只到 193。
  下面测试里各有一个用例把这两处钉住。
- **KC3Kai 代码自身**先算 `actualBase = ⌊R·S⌋` 再乘加成，与它注释里写的公式（即 fandom 的）不符；以注释与 fandom 为准。

## Decision

1. **常量表放在 `expedition.rs`**，与 `DRUM_CANISTER_MST_ID`、`ESCORT_CARRIER_STYPE` 同一做法：
   `LANDING_CRAFT_RATES: [(i64, i64); 11]`（mst_id, 素补正基点），`KINU_KAI_NI_MST_ID = 487`，`TOKU_DAIHATSU_MST_ID = 193`，
   `DAIHATSU_MST_ID = 68`，`TOKU_CAPS: [[i64; 5]; 2]`。不进码典：没有现成的第三方数据文件可下载，表也只有十几项。
2. **返回时判定，不加 fleet 列**：在 `complete_expedition` 成功分支用 `mission_ships` 当下的六个槽（含 `slot_ex`）计算。
   实机远征中可以换补強増設的装备（wikiwiki「遠征」页），KC3Kai 也数补强槽。不要把它改成出发快照。
3. **整数运算**：基点表示比率，`S` 用分数 `(3, 2)` / `(1, 1)`。三项各一次整数除法：
   - `⌊R·s_num / s_den⌋`
   - `⌊R·s_num·B1bp·(100n + Σ★) / (s_den·10_000·100n)⌋`（`n = 0` 时 `B★ = 0`，分子分母去掉 `100n` 因子）
   - `⌊R·s_num·B2bp / (s_den·10_000)⌋`

   不用 `f64`：`0.29 * 100` 之类会落在整数下方，取整差 1。无加成时第一项等于现有的 `⌊1.5R⌋`。
4. **形状**：纯函数 `landing_craft_bonus(slots: impl Iterator<(mst_id, level)>, has_kinu_kai_ni: bool) -> LandingCraftBonus`
   （`b1_bp`、`star_sum`、`star_count`、`b2_bp`）；`calculate_expedition_resource_reward` 多收一个 `&LandingCraftBonus`。
   另一个 async 函数照 `count_drum_canisters` 的写法从 DB 取槽，喂给纯函数。只作用于四种资源，道具报酬不变。

## Implementation Units

- **U1 纯函数与常量**：`LandingCraftBonus`、常量表、`landing_craft_bonus`、改造后的 `calculate_expedition_resource_reward`。
  先写单元测试（下表），再实现。
- **U2 接线**：`complete_expedition` 成功分支取槽、算加成、传入。失败分支不动。
- **U3 回归测试**：
  - 单元测试（`expedition.rs` 的 `tests` 模块），每例成功 / 大成功各一个断言：

    | 用例 | R | 成功 | 大成功 |
    |---|---|---|---|
    | 无大発 | 35 | 35 | 52 |
    | 大発★0 ×4（`R·S` 带 .5，钉住三段取整） | 35 | 42 | 62 |
    | 特大発★4 ×2 + 大発★10 ×2 | 100 | 125 | 188 |
    | 特大発★10 ×4 + 大発★10 ×2 + 鬼怒改二（B1 封顶、B2 cap 5.8%） | 100 | 127 | 191 |
    | 特二式★10 ×1（★项随 B1 缩放；kcanotify 会得 1030） | 1000 | 1011 | 1516 |
    | 特大発★0 ×3 + 八九式★0 ×2（cap 只数 68；kcanotify 会得 1242） | 1000 | 1240 | 1860 |
    | 大発★0 + 230★10（0% 装备不进★平均） | 100 | 105 | 157 |

    数值由 `Fraction` 精确算出，实现后若有出入，先查实现，不改期望值。
  - 集成测试（`crates/emukc_gameplay/tests/expedition.rs`）：给一支能成功的舰队装 4 个大発（`add_slot_item` + `set_slot_item`），
    走 `start_expedition` → 推进返回时间 → `complete_expedition`，断言 `resource_reward` 等于基本量 ×1.2 的取整结果
    （按所选远征的基本量写死数字），证明返回时读了装备。
- **U4 文档**：在 `docs/solutions/architecture-patterns/expedition.md` 加一节「Rewards: landing craft bonus」：公式、来源与
  kcanotify 的差异、返回时判定的理由、常量表过期风险。

U1 → U2 → U3（单元测试随 U1 先写）→ U4。

## Risks

- **取整口径未有定论**：fandom（三段）与 wikiwiki（两段）在 `R·S` 带 .5 且加成项小数部分 ≥ .5 时差 1，例如 R=35 大成功
  带 4 大発：62 对 63。选 fandom 是因为它与 KC3Kai 注释一致、且显式写了「不能合并取整」。有实测反例再改。
- **特大発 3 个以上的 cap** fandom 标注为未完全验证（有反例），wikiwiki 与 KC3Kai 的表一致，先按表。
- **常量表会过期**：新的大発系装备实装后要手动补表（对照 wikiwiki「大発動艇」页的装備表与 KC3Kai `calcLandingCraftInfo`）。
- **鬼怒改二联合舰队特例**（两队各一艘只算一个）不处理：远征没有联合舰队。

## Verification

- Balance Defaults Policy：实现单独一个提交，`feat(balance): add the landing craft bonus to expedition rewards`；正文写明旧值
  「无大発加成：报酬 = 基本量，大成功 ⌊×1.5⌋」，引用本计划。计划本身单独 `docs(plans):` 提交。
- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test -p emukc_gameplay expedition` 随开发跑；最后 `cargo test --workspace --no-fail-fast`，报告 ignored 数。
- 可选：起服务器，带 4 大発派一次短远征，确认结果画面的资源数与公式一致。
