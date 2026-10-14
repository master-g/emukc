---
title: "Practice Rivals From HQ Level - Plan"
type: feat
date: 2026-10-14
status: implemented
execution: code
---

# Practice Rivals From HQ Level - Plan

Issue #102。现在每次生成五个固定模板对手：司令部等级 119～115，各带一艘 Lv180 的随机舰，玩家选的演习组别存了但不用。
改成按玩家司令部等级和演习组别生成，验收条件以 #102 的 Agent Brief 为准（下文 Decision 只补 Brief 没定的细节）。
术语用 `GLOSSARY.md` 的「Practice rival（演习对手）」「Practice group（演习组别）」。

**范围**：只做 #102。演习经验公式是 #109，Brief 明确划在外面；#109 要在本计划落地后，用新对手的真实等级去核公式，另写计划。
**基线**：`main` `13abd10d`，分支 `feat/practice-rivals`。

## Evidence

- **现状** `generate_practice_rivals`（`crates/emukc_gameplay/src/game/practice.rs`）：参数 `_select_type: RivalType` 未用；
  舰船池是 `api_mst_ship` 里 `api_aftershipid.is_some()` 的；`(1..6)` 生成五个，`level: 120 - i`，一艘 `level: 180, star: 10`；
  先删旧的 `rival_ship`/`detail`/`rival` 三张表再插入。唯一调用方是 `get_practice_rivals_impl`（无对手或跨过 3 点/15 点时）。
- **玩家等级**：`profile::Model::hq_level`。**组别**：`RivalType::{FirstGroup = 0, SecondaryGroup = 1, All = 2}`
  （`emukc_model/src/profile/practice.rs`），由 `change_practice_matching_kind` 写进 `practice::config.selected_type`。
- **舰船池**：码典 `ship_extra` 正好是 865 艘玩家舰（id 全部 ≤ 1500，与 `api_mst_ship` 里 ≤ 1500 的条目一一对应），不含深海舰。
  `codex.new_ship` 本来就要求舰在 `ship_extra` 里，战斗时 `build_practice_enemy_ships` 用它取初期装备，所以池子取 `ship_extra` 的键。
- **改造等级**：`ship_extra[id].remodel.level` 是「改造成这个形态需要的等级」，没有 `remodel` 的是初始形态（`new_ship` 同样把它当最低等级）。
  用 Dijkstra 沿 `api_aftershipid` 算「从初始形态走到这个形态途中的最高等级」，865 艘里与形态自身的 `remodel.level` **0 处不同**，
  所以直接用形态自身的值即可，不必沿链求最大。例：大和 1、大和改 60、大和改二 88、大和改二重 93。
- **改造链**：`Codex::ships_before_and_after(mst_id)` 返回整条链（处理了来回转换和宗谷 645/650/699 这种首尾相接的环），
  测试 `remodel_chains_end_even_when_remodels_convert_back_and_forth` 钉住。
- **補 形态**：743/744/745/748/749（長波改二補 等）在 `ship_extra` 里，`buildable: false`，但 `api_mst_ship` 里 543/578/1034/411/412
  的 `api_aftershipid` 指向它们（Lv92），客户端也就允许改造过去，按可拥有处理。
- **随机数**：`emukc_crypto::rng` 有线程局部的自由函数（`rng::seed` 可重设种子）和可注入的 `GameRng::seeded(u64)`。
- **测试**：`crates/emukc_gameplay/tests/practice_battle.rs` 的胜利等级测试先 `rng::seed(WIN_RANK_SEED)` 再列对手，
  注释写明「对手和战斗用同一条随机流；种子跑输了就换种子，不放宽断言」。`tests/gameplay_tests/level_cap_exp.rs` 不依赖对手形状。
  `calculate_base_exp`（`battle/practice/response.rs`）读 `rival.level`，`level × 9` 夹在 100～1200。

## Decision

1. **等级区间先夹再抽**：司令部等级 H 从 `[max(1, lo), min(120, hi)]` 均匀抽，第二群 `[P−10, P+10]`，第一群 `[P, P+20]`，
   全部则每个对手先等概率选一个区间。不先抽后夹：那样 P=1 时近一半第二群对手会堆在 H=1。
2. **先抽等级再抽舰**：每艘舰先从 `[max(1, H/2), min(H, 99)]` 均匀抽等级 L（整数除法），再从「池中 `remodel.level ≤ L`、
   且改造链与本舰队已有舰不重叠」的形态里均匀抽一个。初始形态最低等级都是 1，候选永远非空。不结婚由 L ≤ 99 保证。
3. **纯函数 + 薄的写库层**：抽签做成不碰数据库的纯函数，形如
   `draw_practice_rivals(codex, hq_level, group, rng: &GameRng) -> Vec<Rival>`；`generate_practice_rivals` 读 `hq_level`，
   用 `GameRng::seeded(rng::u64(..))` 调它，然后照旧删表、插表。这样单元测试可以固定种子大量抽样，生产仍走线程局部随机流，
   `practice_battle.rs` 里 `rng::seed` 的做法继续有效。对手和舰船 id 的分配方式不变。
4. **改造链去重**：抽中候选后用 `ships_before_and_after` 取链，与本舰队已用的链求交，相交就从候选里剔除重抽。
   不预先给全池建索引：一次生成最多 30 艘，按需算足够。
5. **展示**：名字、签名各一份十来条的常量表放在 `practice.rs`，每个对手各抽一条（同一批对手可以重名，不额外去重）；
   `deck_name` 为旗舰的 `api_name` + 「艦隊」。`rank`、`flag`、`medals` 维持随机。
   `RivalDetail` 里的 `exp_now`/`exp_next`、舰船与装备数量、`furniture` 和 `RivalShip.star` 维持现值：Brief 没要求，
   客户端代码里也搜不到 `api_star` 的读取。

## Implementation Units

- **U1 纯函数**：常量表、`draw_practice_rivals`、区间与改造等级规则。先写单元测试（`practice.rs` 的 `#[cfg(test)]`，加载 `.data/codex`），
  再实现。对 P ∈ {1, 60, 120} × 三种组别 × 若干种子逐条断言：
  - H 落在所选组别的区间内（全部则落在并集内），且在 1～120；
  - 每个对手 1～6 艘，每艘等级在 `[max(1, H/2), min(H, 99)]`；
  - 每艘的 mst_id 在 `ship_extra` 里，且 `remodel.level ≤ 舰等级`；
  - 同一舰队里任意两艘的改造链不相交；
  - `deck_name == 旗舰 api_name + "艦隊"`，名字和签名都来自常量表；
  - 组别确实生效：第一群在 P=60 时没有 H < 60 的对手，第二群没有 H > 70 的。
  - **经验回归**（Balance Defaults Policy 规则 5）：用 `calculate_base_exp` 读生成的对手，P=10 时每个对手的基础经验在 100～270；
    P=60 时第一群在 540～720、第二群在 450～630。旧值是固定 H 115～119 的 1035～1071，若有人改回模板或改动区间，这组断言会失败。
- **U2 接线**：`generate_practice_rivals` 读 `profile.hq_level`、用上 `select_type`，调 U1；删除过时的 TODO 注释。
- **U3 集成测试**（`crates/emukc_gameplay/tests/practice_battle.rs`）：把档案的 `hq_level` 设成中间值、组别切到第二群，
  `rng::seed` 后 `get_practice_rivals`，断言五个对手的 H 都在区间内；再挑一个多艘舰的对手打完 `practice_battle` →
  `practice_battle_result`，断言有结果。现有胜利等级测试若因随机流改变而跑输，按该文件注释换 `WIN_RANK_SEED`，不放宽断言。
- **U4 文档**：新建 `docs/solutions/architecture-patterns/practice-rivals.md`：生成规则、改造等级取自 `ship_extra`、
  補 形态按可拥有处理、经验公式仍读 H（#109 待定）、名单在代码里。

U1 → U2 → U3 → U4。

## Risks

- **经验会大幅变化**：经验仍是 `H × 9`（100～1200）。新档案 HQ 1 时对手 H 在 1～21，基础经验从约 1050 掉到 100～189；
  HQ 10 时 100～270；第一群比第二群多给。用户选择接受这个落差（方案 A），按 balance 流程记录，#109 再改公式。
- **胜利等级测试的种子**：对手抽签消耗的随机数变了，`WIN_RANK_SEED = 2` 可能跑输，需要换种子（见 U3）。
- **補 形态**是否真能入手没有一手来源；若确认只是友军形态，在池子上排除这 5 个 id 即可。
- **名单是手写的**：只影响显示，不影响数值。

## Verification

- `cargo test -p emukc_gameplay practice` 随开发跑；`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`；
  最后 `cargo test --workspace --no-fail-fast`，报告 ignored 数。
- **按 Balance Defaults Policy 处理**（用户 2026-10-14 选定）：不改 `Default` impl，但演习经验随对手等级大幅变化，规则 1～5 照做。
  计划本身单独 `docs(plans):` 提交；实现（含 U1～U3 的代码与测试）单独一个
  `feat(balance): draw practice rivals from the player's HQ level` 提交，正文写明旧值「五个对手 H 固定 115～119，各一艘 Lv180；
  基础经验 `H × 9` 约 1035～1071，与玩家等级无关」并引用本计划；回归测试见 U1 的经验回归；PR 单列经验变化。U4 文档另起 `docs:` 提交。
- 不动码典与资产。
- 可选：`make headless-check` 没有演习场景，不跑；起服务器打开演习画面看五个对手的等级、舰数与舰队名。
