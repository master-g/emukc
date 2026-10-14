---
title: "Practice Exp From the Rival's Ships - Plan"
type: feat
date: 2026-10-14
status: draft
execution: code
---

# Practice Exp From the Rival's Ships - Plan

Issue #109。演习基础经验现在是 `calculate_base_exp(rival)` = 对手司令部等级 × 9，夹在 100～1200，是项目自编的公式。
官方读的是对手**旗舰与二号舰的等级**。#102（PR #131）已经让对手带上真实等级的舰船，可以换公式了。
这是 gameplay 数值改动，按 Balance Defaults Policy 处理（不动 `Default` impl，规则 1～5 照做，见 Verification）。

**基线**：`main` `c200a555`，分支 `feat/practice-exp`。

## Evidence

- **现状**：`calculate_base_exp`（`crates/emukc_gameplay/src/game/battle/practice/response.rs`）只读 `rival.level`。
  `run_day_battle`（`battle/practice/orchestrate.rs`）用它算出 `base_exp`，然后：
  - `api_get_base_exp` = `base_exp`；
  - 提督经验 `calculate_admiral_exp(base_exp, rank)`（`game/ship/exp.rs`，按 S/A/B/C/其他 取 12%/10%/8%/5%/3%）；
  - 舰船经验 `calculate_ship_exp`：MVP ×2、旗舰 ×1.5、其他 ×1，再乘练巡与 `practice_exp_boost` 配置，**不乘评价补正**。
  夜战结果另有一份同样的计算（同文件 `run_night_battle`），实现时两处一起改。
- **一手来源**：wikiwiki「演習」（2026-10-14 取，「演習で得られる経験値」「基本経験値表」「評価による補正倍率」三节）：
  - 补正前经验 = 旗舰 Lv 的累计所需经验 / 100 + 二号舰 Lv 的累计所需经验 / 300；
  - 补正前 ≤ 500 时基础经验 = 补正前；> 500 时 = 500 + √(补正前 − 500)；
  - **单艘时按二号舰 Lv1 计算**（累计经验 0）；
  - 评价补正：S 1.2，A/B 1.0，C 0.64，D 0.56，E 0.4；结果画面显示的基础经验（`api_get_base_exp`）是「基础经验 × 评价补正」，
    舰船经验在此之上再乘 MVP、旗舰、练巡补正；
  - 「实际经验会多出 0～3」，原因未明。
- **对拍**：用本项目的 `level::ship_level_required_exp`（与 wiki「経験値」页同一张累计表）按「两项各自整除后相加，
  超过 500 的部分取平方根，结果向下取整」计算，wiki 基本経験値表 362 格中 **361 格一致**。唯一不一致的是旗舰 150 × 二号舰 140
  （wiki 727，算得 728，补正前 52800），它四周的格子都一致，判为 wiki 笔误。脚本与结果在会话 scratchpad，不进仓库。
- **对手舰船顺序**：`get_practice_rivals_impl` 与 `get_practice_rival_details_impl` 读 `rival_ship` 时没有 `ORDER BY`，
  旗舰是谁靠 SQLite 碰巧按主键返回。#102 生成时舰船 id 按位置递增，所以按 `id` 升序就是舰队顺序。
- **数值变化**（无评价补正前）：

  | 情形 | 旧 | 新 |
  |---|---|---|
  | 任意对手（#102 前的模板，或现在的 H×9） | 100～1200，模板时 1035～1071 | — |
  | 对手旗舰、二号舰都 Lv99 | 891（H=99） | 613 |
  | P=60 第一群（H 60～80，舰 Lv30～80） | 540～720 | 435～567 |
  | P=60 第二群（H 50～70，舰 Lv25～70） | 450～630 | 300～555 |
  | P=10（H 1～30） | 100～270 | 0～508 |
  | 旗舰 Lv1 单艘 | 100 | 0 |

## Decision

1. **按 wiki 公式实现基础经验**，签名改为读对手舰船：`calculate_base_exp(rival)` 取 `details.ships` 的第 1、第 2 艘的等级，
   没有第 2 艘时按 Lv1。整数运算：补正前 = `exp(flag) / 100 + exp(second) / 300`；超过 500 时 `500 + isqrt(补正前 − 500)`。
   用整数平方根，避免 `f64` 落在整数下方。去掉 100～1200 的夹取（官方没有下限，旗舰 Lv1 单艘就是 0）。
   「多出 0～3」不复现：原因未知，也不影响上限。
2. **舰船经验补上评价补正**：`api_get_base_exp` = ⌊基础经验 × 评价补正⌋，舰船经验在这个值上乘 MVP、旗舰、练巡与
   `practice_exp_boost`，与 wiki「最終取得経験値=基本経験値×評価補正×練巡補正」一致。补正按 S 1.2、A/B 1.0、C 0.64、D 0.56、E 0.4。
   这一项不在 #109 的原文里，但它和基础经验是同一条公式、同一个显示字段，不补的话 S 胜少给两成、败北多给，
   换完基础经验后差距只会更显眼。用户 2026-10-14 选定并入。
3. **提督经验不改公式**：仍是 `calculate_admiral_exp(基础经验, 评价)`，输入是评价补正**前**的基础经验，和现在的含义相同；
   wiki「演習」页没有给提督经验的公式，没有依据去改。它会随新的基础经验一起变小。
4. **读对手舰船时按 `id` 升序**：两处查询加 `order_by_asc(rival_ship::Column::Id)`，保证第一艘是旗舰。
   这也让 #102 的舰队名（旗舰名 +「艦隊」）在读回时稳定。
5. **#102 的经验回归测试按新公式重写**：`practice_base_exp_follows_the_player_level` 的区间换成新值
   （P=10 为 0～508；P=60 第一群 435～567，第二群 300～555），注释写明旧值与改动原因。这是有意重新钉住，不是放宽。

## Implementation Units

- **U1 基础经验公式**：先写单元测试（`battle/practice/response.rs` 的 `#[cfg(test)]`），期望值直接抄 wiki 表，不用公式推：
  - 单艘 / 二号舰 Lv1：旗舰 1 → 0，30 → 435，40 → 516；
  - 两艘：(1, 5) → 3，(15, 50) → 503，(60, 60) → 543，(99, 99) → 613，(120, 120) → 627，(150, 150) → 740，(180, 180) → 915；
  - 跨过 500 的边界：(30, 20) → 498，(30, 25) → 505。
  再改 `calculate_base_exp`。
- **U2 评价补正**：先写单元测试：基础经验 510 时 S → 612、A/B → 510、C → 326、D → 285、E → 204（⌊510 × 补正⌋）。
  再在 `run_day_battle`、`run_night_battle` 里算出补正后的值，写进 `api_get_base_exp` 并传给 `calculate_ship_exp`。
- **U3 舰船顺序**：两处 `rival_ship` 查询加按 `id` 升序。
- **U4 回归测试**：重写 `practice_base_exp_follows_the_player_level`（Decision 5）；在 `crates/emukc_gameplay/tests/practice_battle.rs`
  加一个集成测试：固定种子打一场，断言 `api_get_base_exp` 等于「按对手前两艘等级查 wiki 表得到的值 × 该场评价补正」，
  期望值由测试里写死的对手等级推出（必要时直接把对手舰船改成已知等级，如旗舰 Lv60 + 二号舰 Lv60 → 543）。
  现有测试里断言 `api_get_exp`、舰船经验的，若因数值变化失败，逐个确认是预期的变化再改期望值。
- **U5 文档**：更新 `docs/solutions/architecture-patterns/practice-rivals.md` 的「Why This Matters」，补公式、来源、对拍结果与
  评价补正；去掉「#109 待定」。

U1 → U2 → U3 → U4 → U5。

## Risks

- **经验整体下降（用户 2026-10-14 接受）**：官方公式在双方都 Lv99 时只有 613（S 胜 735），现在是 891～1071，高等级账号也会明显少拿。
  `practice_exp_boost` 配置可以整体放大，默认值不动（改它属于另一个 balance 改动）。
- **低等级账号几乎拿不到经验（用户 2026-10-14 接受）**：#102 让对手舰船等级不超过对手司令部等级，官方公式又没有保底，
  HQ 10 左右大多只有几十到两百点（旗舰 Lv10 单艘 45，双方 Lv1 为 0）。实机里低等级玩家常遇到放高等级舰的对手，
  这里不模拟；不放宽 #102 的等级区间，也不加保底。
- **wiki 表的笔误**只有一格，测试不取那一格。
- **「多出 0～3」**没有复现，实测会比本实现多 0～3。

## Verification

- **按 Balance Defaults Policy 处理**：计划单独 `docs(plans):` 提交；实现单独一个
  `feat(balance): read practice exp from the rival's flagship and second ship` 提交，正文写明旧值
  「基础经验 = 对手司令部等级 × 9，夹在 100～1200；舰船经验不乘评价补正」并引用本计划；回归测试见 U1、U2、U4。
  文档另起 `docs:` 提交。
- `cargo test -p emukc_gameplay practice` 随开发跑；`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`；
  最后 `cargo test --workspace --no-fail-fast`，报告 ignored 数。
- 不动码典、资产与 frozen golden。
