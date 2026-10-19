---
title: "AP Shell Accuracy and Power - Plan"
type: feat
date: 2026-10-19
status: implemented
execution: code
---

# AP Shell Accuracy and Power - Plan

Issue #196（按 triage 评论修正：特殊攻击自身的命中倍率已做，只剩徹甲弾）。来源 `kancolle-replay@69097abc`：`kcships.js:550-555`（`APtype`：带 AP 弾且带主炮才有；雷达+副炮 4、副炮 3、雷达 2、否则 1），`:1500-1518`（`APmod` 1.08/1.1/1.15/1.15，`APacc` 1.1/1.25/1.2/1.3，目标不 `APweak` 时为 1），`:1878`、`:1885`、`:1898`、`:1930`、`:1964`、`:2116`（`APweak`：CA、BB 及继承它的 FBB、CAV、BBV、CV 及继承它的 CVB；CVL 为 false）。装备种类按 `kcEQDATA.js` 的 `btype`：主炮 1、2、3、38，副炮 4、95，雷达 12、13、93，AP 19，与 `targeting.rs` 现有判定一致。

用处只在 `kcsim.js` `shell`（昼战炮击，特殊攻击也经 `shellPhaseAttack` 走这里）：`:450` `accMod2 = APacc`，在 `hitRate` 与 `ACCfit` 之后乘（`:522-524`）；`:602-606` 威力 `floor(softCap(...))` → `× postModCI` → 目标 `APweak` 时 `floor(pow × APmod)` → `× postModExtra`（反设施 post-cap 等）。目标 `APweak` 时无论有无 AP 弾都会取整。夜战、雷击、对潜、航空、支援炮击（`:1050` 注释掉）不变。

**范围**：U1，一个 PR。
**非目标**：特殊攻击自身的倍率（已做）；大和型 400/401；AP 弾的反设施加成（已做）。

## Implementation Units

### U1 AP 档位进命中与威力

- `damage.rs`：`armour_piercing(codex, attacker, defender)`，目标不 AP-weak 时为 `None`，否则给出该档的命中与威力倍率（无档位为 1）。
- `accuracy.rs`：`hit_value` 取 defender，`Shelling` 时把命中倍率并入 `day_cut_in`；`roll_attack` 一处覆盖普通炮击与特殊攻击。
- `damage.rs` `calculate_shelling_damage`：CI 倍率之后、反设施 post-cap 之前，目标 AP-weak 时 `floor(pow × APmod)`。
- 单测：四档与有 AP 无主炮；二档打战舰命中 ×1.25、夜战不变；二档打战舰威力 ×1.1 取整，打 DD、CVL 不变。
- 文档：`battle-hit-and-critical-rolls.md`（Not modelled）删去 AP 弾命中与特殊攻击命中两条。
- golden：180 份中 4 份变化（practice 1、stf 1、te 2）。验证：`armour_piercing` 恒为 `None` 时 180 份与旧版逐字节一致；只保留取整（档位恒为 1）时输出与完整实现逐字节一致，只保留命中时无变化。即变化全部来自「目标 AP-weak 时 CI 后威力取整」，golden 里没有吃到档位的攻击方。`battle_golden.rs` 未变。
