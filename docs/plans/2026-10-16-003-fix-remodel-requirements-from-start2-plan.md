---
title: "Remodel Requirements from start2 - Plan"
type: fix
date: 2026-10-16
status: implemented
execution: code
---

# Remodel Requirements from start2 - Plan

Issue #180。服务器的改装需求取自 kcwiki 舰船数据（`Kc3rdShipRemodelRequirement`，经 `Codex::find_ship_after`），与客户端显示、校验的不一致：81 条路线少扣、6 条反向改装多扣、狭霧改→改二 报 `NotFound`。按用户决定（2026-10-16）改为和客户端一样读 start2，start2 没有的数字取客户端 `main.js` 里的表，由 `main-decoder` 抽取，不手抄。

**范围**：U1–U4，一个 PR，叠在 #178（PR #181，补 `api_tech_count`）之上。
**非目标**：
- 服务器检查改装等级（客户端已校验；今天服务器也不查）。
- 改 `Kc3rdShip.remodel`：`codex/ship.rs` 的初始等级与演习对手等级仍读它的 `level`。
- kcwiki 缺的舰船（902）的数据：#182。

**基线**：`fix/start2-tech-count-bgm-id` `62138b1d`。

## Evidence

客户端规则（6.3.6.0，模块号随版本变，按导出名找）：

- `ShipUpgradeModelHolderEdit.setData`：`api_mst_shipupgrade` 按 `api_current_ship_id` 建表，跳过 ≤0，同一 before 取第一行。
- `getNextID(before)`：有行取行的 `api_id`，否则 `api_mst_ship[before].api_aftershipid`。
- `getRequires(before)`：`api_aftershipid` ≤0 时全为 0；否则 ammo=`api_afterbull`、steel=`api_afterfuel`；有行时用品取行，没有行时为 0；devkit=`_getRequiredDevkitNum(before, blueprint, steel)`，buildkit=`_getRequiredBuildKitNum(before)`。等级 `getRequiredLevel` = `api_afterlv`。
- `_getRequiredDevkitNum`：按 before 查表；表外时 `blueprint≠0 且 before∉_USE_DEVKIT_GROUP_([503,504])` 或 steel<4500 → 0，<5500 → 10，<6500 → 15，否则 20。`_getRequiredBuildKitNum`：按 before 查表，表外 0。
- `ShipUpgradeModel`（行）：blueprint/catapult/report/77/94/boiler/tech 读 `api_*_count`；另有三处按 after 写死的表，start2 没有对应字段：75 `newhokohesosizai`（911 只在 before=136 时为 3）、4 `revkit`（975→5）、104 `arsenalmaterial`。issue 第 2 步只点了两个函数，这三张表也须抽，否则对拍做不到无例外。
- `RemodelingAPI._completedEnd` 扣：32 弾薬、33 鋼材、3 開発資材、2 高速建造材、4 改修資材、58、65、78、75、77、94、100、104；ボイラー在别处按装备扣。

服务器今天：`remodel_impl` 扣 ammo、steel、devmat、torch 与 58/65/78/77/75/94/100、ボイラー；不扣 4 改修資材 与 104。

## Implementation Units

### U1 decoder 抽取 `remodel_tables`

- `main-decoder/src/remodel-tables.ts`：按导出名 `ShipUpgradeModelHolder`、`ShipUpgradeModel` 找模块，用 babel 解析 switch，得 devkit/buildkit（按 before）、devkit 默认分支（组、钢材档位）、75/4/104（按 after，可带 before 条件）。形状不认识就报错。
- `pipeline.ts` 写到 `out/resources/remodel_tables.json`，`--sync-assets` 同步到 `crates/emukc_bootstrap/assets/`（`make update`/`decode-main` 已带此参数）。不写 `generatedAt`，重复 decode 无漂移。
- 测试：bun 单测喂合成模块源码；`remodel-tables-oracle` 脚本加载真实客户端，对 1..1500 的 id 跑客户端函数与资产比对，挂在 `make update` 后（不阻断）。

### U2 codex 读资产，按客户端规则求需求

- `assets.rs` 登记 `remodel_tables`（embedded），bootstrap 写进 codex 目录，`Codex` 加载。
- `Codex::remodel_requirement(before) -> (after, RemodelRequirement)` 取代 `find_ship_after`；需求含等级与客户端扣的全部 14 项。
- 删去只有 `find_ship_after` 读的 `remodel_back_to`、`remodel_back_requirement`。

### U3 扣除

- `remodel_impl` 按新结构扣；补 4 改修資材（材料 Screw）与 104 工廠資材。

### U4 测试与文档

- 对拍测试：遍历 start2 每艘舰船（覆盖每行 shipupgrade 与每条 `api_aftershipid`），用独立写的客户端规则对比 `remodel_requirement`，无例外表。
- gameplay：Bismarck改→zwei 无设计图被拒、有则成功；大和改二重→改二 不扣设计图与ボイラー。狭霧改→改二 移到 #182（用户定，2026-10-16）：kcwiki 舰船数据没有 902，`new_ship` 与 `cal_ship_status` 都造不出它；需求本身由对拍测试覆盖。
- `docs/solutions/` 记录改装需求来源。
