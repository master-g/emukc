---
title: "Fleet LOS for Artillery Spotting - Plan"
type: fix
date: 2026-10-22
status: implemented
execution: code
---

# Fleet LOS for Artillery Spotting - Plan

Issue #290。来源 `kancolle-replay@69097abc`：`kcships.js:101-117`（`Fleet.fleetLoS`）、`:1487-1498`（`ASchance`，`combinedAll` 时加上组合的另一支舰队）、`:1033-1038`（`statsBase`）；`kcsim.js:3814-3815`、`kcsimcombined.js:271-273`、`:931-933`、`:1390-1393`（开幕雷击之后清缓存）；`kcsim.js:1578-1597`、`:1648`（`shellPhaseC` 的 `combinedAll`）；`kcEQDATA.js`（`B_RECON`）。

**基线**：`main`（#289 之后，PR #295）。
**非目标**：
- 来源的 `LOSeq`（`ship_los_from_equipment`）与 `ASchance` 其余各项。
- 12 对 12（我方联合对敌方联合）：引擎没有这条路径。
- 来源的跨轮缓存：用户定沉没舰不再计入（2026-10-22），每个炮击轮开头重算，见 U2。

## 事实

- 来源 `fleetLoS`：未沉、未退避的每艘舰，`statsBase.LOS`（总值减装备与装備ボーナス，即本体）加 `B_RECON` 装备的 `LOS × floor(√机数)`。`B_RECON` 只有 `SEAPLANE`（10）、`SEAPLANEBOMBER`（11）两类，没有单件装备另设 `btype`；与 codex `api_type[2]` 为 10、11 的 50 件逐件相同，索敌值全等（2026-10-22 对拍）。
- 来源的缓存在 `resetBattle` 与开幕雷击之后清空，之后每支舰队第一次被问到时算，整场炮击沿用：第 1 轮沉的舰第 2 轮仍算。本项目每轮开头重算，且取的是总索敌（含全部装备与装備ボーナス）。真实游戏怎么算没有找到证据（検証 Wiki 写着验证中）。
- `combinedAll`：`shellPhaseC` 里的每次攻击都带，舰队索敌加上 `combinedWith` 的。本引擎里对应「单舰队对敌联合」的第 3 轮（敌主力开火，两支 deck 都被打）：敌方是主力加护卫，我方单舰队没有另一支。我方联合对敌单舰队走 `shellPhase`，各 deck 各算，与现状相同。
- 引擎没有退避建模。

## Implementation Units

### U1 舰队索敌

- `shelling::fleet_los(codex, ships)`：未沉舰的本体索敌，加水偵、水爆每槽 `api_saku × floor(√api_onslot)`。本体索敌深海舰取 `enemy_ship_extra.los`，我方取 `stat_at_level(lv, ship_extra.saku)`（索敌没有近代化改修），与 `targeting::bare_asw` 同法。不从总值减装备，所以不受演习对手总值不含装备（#291）影响。
- 测试：手算一支舰队，雷达不计、水偵按机数、沉没舰不计、深海舰取本体。

### U2 按 deck 取

- 编排层的 `spotting_los` 每个炮击轮开头取各方开火 deck 的舰队索敌，`combined_all` 的一轮取整支舰队。哪支 deck 开火（`Volley::deck`）、一支 deck 有哪些舰（`deck_ships`）、哪一轮两支都算（`Volleys::combined_all`，只有单舰队对敌联合的第 3 轮）写在 `volley.rs`，与其余 deck 的决定放在一处（审查时改）。`ShellingRound` 带 `fleet_los`（我方、敌方）。
- 偏离来源：来源整场沿用第一次算的值，第 1 轮沉的舰第 2 轮仍算；这里每轮重算，沉没舰从下一轮起不计（用户定，2026-10-22）。一轮之内仍用开头的值。
- 测试：第 1 轮沉的舰第 2 轮不再计入；第 3 轮敌方为两支之和。

### U3 golden 与文档

- golden 若变：临时恢复旧 `fleet_los`，看各份是否逐字节复原，写进提交说明。
- 结果：`crates/emukc_battle/tests/golden` 8/180 变（ctf 3、stf 2、te 3），`battle_golden.rs` 不变。旧公式配新管线时 180 份与原版逐字节相同，变化全来自新公式。曾按来源跨轮沿用时多变 `day_seed_04`（第 1 轮沉的舰计入第 2 轮，多出一次連撃击沉末舰，`midnight_flag` 1 → 0），改为每轮重算后与原版相同。
- `abyssal-ship-stats.md` 的「Her own figures」表加一行。

### 验证

- fmt、严格 clippy、`cargo test --workspace --no-fail-fast`。
