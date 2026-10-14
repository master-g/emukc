---
title: "Expedition Requirements - Plan"
type: fix
date: 2026-10-14
status: implemented
execution: code
---

# Expedition Requirements - Plan

Issues #116、#117、#118。远征条件的内容读错了（`total-fp` 当火力、伪舰种 27 与 `flag-cond: "a/b"` 不认），
判定的时机也不对（不满足就拒绝出发，实机是出发后失败）。验收条件在三个 issue 的 Agent Brief 里。
#119（大发加成）是数值改动，按 Balance Defaults Policy 另写计划，不在这里。

## Evidence

- **条件模型** `Kc3rdExpeditionRequirements`（`emukc_model/src/thirdparty/expedition.rs:65`）：`ship_count`、
  `flagship_level`、`fleet_level`、`flagship_type: Option<i64>`、`composition`、`total_firepower`、`total_asw`、
  `total_los`、`drum_requirements`。没有对空。
- **解析** `convert_to_internal_model`（`emukc_bootstrap/src/parser/kcanotify/expedition.rs:76`）写
  `total_firepower: src.total_firepower.or(src.total_fp)`；`parse_flagship_type`（`:112`）是
  `s.parse().ok()`，`"7/27"` 得 `None`。`parse_composition` 已经把 27 原样产出成 `ship_types: [27]`，解析侧不用改。
  `parse_flagship_type` 的单元测试在约 314 行。
- **kcanotify 的语义**（antest1/kcanotify `874e414f`，`KcaExpeditionCheckViewService.checkFleetCondition`）：
  `total-fp` 按 `taiku` 求和；护卫空母同时计入 27 与本舰种；`flag-cond` 按 `/` 拆，任一匹配即过，旗舰是护卫空母时
  按 27 比。护卫空母名单在 `ships_filter_data.json` 的 `cve_ids`，
  `https://antest1.github.io/kcanotify-gamedata/files/ships_filter_data.json` 可直接取（17 个 id）。
- **下载与码典**：kcanotify 远征数据在 `emukc_bootstrap/src/res.rs:100` 下载为 `kcanotify_expedition.json`，
  `parser/mod.rs:93` 解析；码典 `expedition_condition.json` 由 `Codex`（`codex/mod.rs:107`、`:228`、`:350`）读写。
  `make drift-check` 的指纹只覆盖 `crates/emukc_bootstrap/assets/`，码典目录下新增文件不需要 `drift-accept`。
- **判定** `validate_expedition_start`（`game/expedition.rs:428`）返回 `Result<(), GameplayError>`，不满足即报错；
  `start_expedition` 先判定，再取快照、算补给、扣补给、写 fleet 行。`apply_expedition_supply_costs`（`:902`）
  在船的燃弹不够时返回 `Insufficient` 错误，不会扣成负数。
- **返回** `complete_expedition`：`failure_kind: Option<ExpeditionFailureKind>`，变体只有 `Recall`、`Fatigue`；
  `match failure_kind.unwrap()` 与 `ExpeditionCompletion.admiral_exp` 的 `match` 都按变体分支。成功分支才产出
  `GameplayOutcome::ExpeditionCompleted`。
- **fleet 行** 已有 `launch_*` 快照列；加列的写法见 `emukc_db/src/entity/profile/airbase/mod.rs:17`
  （`ALTER TABLE ... ADD COLUMN`，失败即已存在）。
- **测试**：`tests/gameplay_tests/` 下没有远征文件，入口是 `tests/gameplay_tests/mod.rs`。

## Decision

1. **一个判定函数，三个 issue 共用**：`validate_expedition_start` 改名为 `evaluate_expedition_requirements`，
   返回 `bool`（或一个列出未满足项的结构，方便日志），不再报错。#116、#117 改它判断的内容，#118 改它的调用方。
   「舰队为空」「已在远征」仍在 `start_expedition` 里报错：那是不能出发，不是条件不满足。
2. **#116**：`Kc3rdExpeditionRequirements.total_aa: Option<i64>`，`#[serde(default)]`；解析时 `total-fp` → `total_aa`，
   `total_firepower` 只取 `total-firepower`。判定按 `ship.aa_now` 求和，与火力等三项同一写法。装备改修加成与
   扣除舰载机对潜不做（brief 的 Out of scope）。
3. **#117**：
   - `flagship_type: Option<i64>` → `flagship_types: Vec<i64>`（空 = 不限），`#[serde(default)]`；解析按 `/` 拆。
   - 码典加 `escort_carriers.json`（`Vec<i64>`），`Codex` 加字段 `escort_carriers: BTreeSet<i64>`（或 `HashSet`）。
     下载 `ships_filter_data.json` 到 `.data/temp/`，解析出 `cve_ids` 写进码典。
   - 判定时一艘船的「舰种集合」= `{api_stype}`，是护卫空母时再加 27；编成的 `ship_types` 与旗舰条件都对这个集合
     判断。27 只在本项目的远征判定里出现，不进 `api_mst_stype`。
4. **#118**：
   - `start_expedition` 照常算判定，把结论存进 fleet 行新列 `launch_requirements_met BOOLEAN NOT NULL DEFAULT TRUE`。
     旧存档里在途的远征按「已满足」处理：它们当时已通过旧的出发检查。远征期间舰队不能改编成和装备，
     所以出发时的结论就是返回时的结论，不必快照全部属性。
   - 补给不满的船也能出发：`apply_expedition_supply_costs` 改为扣到 0 为止，不报错；补给是否满额计入判定。
   - `ExpeditionFailureKind::RequirementsNotMet`，返回时 `launch_requirements_met == false` 即走它；效果同
     `Fatigue`（无资源、无道具、失败的提督经验与舰船经验、不产出 `ExpeditionCompleted`）。两个 `match` 补分支。
     判定顺序：召回 > 条件不满足 > 疲劳 > 成功/大成功。

## Implementation Units

- **U1 判定函数改形**：`validate_expedition_start` → `evaluate_expedition_requirements`，暂时在
  `start_expedition` 里对 `false` 仍报错，行为不变。现有单元测试改用新返回值。这一步只是为后面三个 U 留接口。
- **U2 #116**：模型字段、解析、判定。重建码典，抽查 B1（无火力、对空 200）与 A2（火力 50、对空 70）。解析单元测试
  覆盖两字段都有与只有 `total-fp` 两种。
- **U3 #117**：`res.rs` 加下载、解析 `cve_ids`、码典读写 `escort_carriers.json`、`flagship_types`、判定里的舰种集合。
  改 `parse_flagship_type` 的测试并补 `"7/27"`；重建码典确认 43、45 的 `flagship_types` 为 `[7, 27]`。
- **U4 #118**：fleet 新列与 `ALTER TABLE`、`start_expedition` 不再因条件报错、补给扣到 0、
  `RequirementsNotMet` 分支。
- **U5 集成测试** `tests/gameplay_tests/expedition.rs`，在 `mod.rs` 注册。走 `Ctx` 的出发与返回，返回时间用现有
  测试里推进时间的做法（读 `complete_expedition` 的时间判断，必要时直接改 fleet 行的 `return_time`）：
  A2 对潜不足 → 出发成功、返回失败、资源不变、402 无进度；补给不满 → 同上；条件满足 → 成功；B1 无火力门槛；
  护卫空母满足 45。
- **U6 文档**：新写 `docs/solutions/architecture-patterns/expedition.md`（远征的判定时机、kcanotify 字段语义、
  护卫空母），`PROJECT_MEMORY.md` 里对应的事实行改成指向它。

U1 → U2、U3（可并行）→ U4 → U5。U5 的用例可以随 U2–U4 逐步加。

## Risks

- **码典重建是硬依赖**：U2 改了 `expedition_condition.json` 的形状，旧码典里没有 `total_aa`、`flagship_types`，
  靠 serde 默认读成「没有对空门槛、不限旗舰」，不报错但测试结果会错。U3 新增的 `escort_carriers.json` 在旧码典
  目录里不存在，`Codex::load` 对每个文件都是 `read_to_string(&path)?`，会直接失败，`gameplay_tests` 整批报错；
  不做缺失容忍，错误信息要指向缺的文件名。实施记录写明重建过码典。
- **`flagship_type` 改名**会碰到 `expedition.rs` 测试里的构造（约 1395 行）。
- **护卫空母名单会过期**：新实装的护卫空母要等 kcanotify 更新、再跑 bootstrap。
- **客户端对出发报错的处理不再相关**，但 `api_req_mission/start` 的响应字段不变，客户端无感。
- **补给扣到 0**：补给不满的船出发后燃弹可能归零，返回后仍要补给，与实机一致。

## Verification

- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test --workspace --no-fail-fast`，报告 ignored 数。
- 重建码典 `cargo run -- bootstrap --codex-only`（U3 需要先完整 `bootstrap` 一次以下载新文件，或手动取到
  `.data/temp/`）。
- 可选：起服务器，用客户端派一支不满足条件的舰队去 A2，确认能出发、返回显示失败。
