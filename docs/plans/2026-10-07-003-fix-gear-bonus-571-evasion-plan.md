---
title: "Gear Bonus 571 Evasion Follows KC3Kai - Plan"
type: fix
date: 2026-10-07
status: implemented
execution: code
---

# Gear Bonus 571 Evasion Follows KC3Kai - Plan

## Problem

装備ボーナス的订正层一律以客户端 `SlotItemEffectUtil` 的显示代码为准。其中一条把
53cm連装魚雷改(酸素魚雷)（571）★9 以上的回避从 KC3Kai 的每件 +1 订正成每件 +6，对 576 艘舰生效。
一件鱼雷改到 ★9 就给几乎所有舰回避 +6，和同类装备的幅度不相称；KC3Kai 的 +1 是玩家对照服务器数值记的。
客户端这个值更像笔误，而服务器模拟器该给的是服务器的值。

复查过全部分歧（55 种装备 89 行，`.data/temp/gear_bonus_conflicts.md`，未入库）。其余多数只差 1 点，
离线判断不了谁对，继续以客户端为准。用户 2026-10-07 定：只改这一条。

## Decision

- 删掉 `main-decoder/gear-bonus-corrections.json` 里 571 的订正，资产回到 KC3Kai 的 +1。
- `make gear-bonus-oracle` 因此报 3 个条目与客户端不同：571 本身，以及探针里同时带 571 的 503、530。
  三条写进 `main-decoder/gear-bonus-known-diffs.json`，注明是有意保留。
- 不动：其余 69 条订正。

## Verification

`make gear-bonus-oracle` 只报已登记的 3 条；三道质量门。
