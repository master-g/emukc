---
title: "Random Engagement Form - Plan"
type: fix
date: 2026-10-08
status: implemented
execution: code
---

# Random Engagement Form - Plan

## Problem

出击战斗的交战形态由 `engagement_for_cell` 按 `(海域 + 格子) % 4` 算出，同一个格子永远是同一种。
真实游戏里它每场随机，T字有利到 T字不利的攻击力从 120% 到 60%，所以固定值让每个格子的难度都偏离了真实情况。
用户实测发现，2026-10-08 确认要改。

## Decision

- 每场出击战斗抽一次：同航戦 45%、反航戦 30%、T字有利 15%、T字不利 10%。
  来源：wikiwiki.jp「戦闘について」的交戦形態表，2026-10-08 读取。
- 彩雲（54）、彩雲(東カロリン空)（212）、彩雲(偵四)（273）装在还有飞机的格子里时，抽到的 T字不利换成反航戦
  （反航戦因此变成 40%，同航戦与 T字有利不变）。联合舰队第一、第二舰队谁带都算。二式艦上偵察機与試製景雲(艦偵型)
  没有这个效果。来源：wikiwiki.jp「彩雲」页，同日读取。
- 开幕夜战（`sp_midnight`）照样抽，但彩雲不生效，同一页的记载。
- 这一次抽取用战斗自己的随机源，并且在战斗模拟之前，所以 `battle sim --seed` 仍可复现。
  昼战转夜战沿用昼战存下的交战形态，本来就是这样，不动。
- 演习同样抽取，彩雲同样生效（用户 2026-10-08 追加）。原先固定为同航戦。
- 抽取与彩雲判定放在 `game/battle/engagement.rs`，出击与演习共用。
- 不动：带彩雲的舰中途退避后效果消失的细节。

## Verification

- `engagement_follows_the_45_30_15_10_split`、`a_saiun_turns_only_t_disadvantage_into_head_on`、
  `only_a_saiun_with_planes_left_counts`。
- `tests/gameplay_tests/battle_golden.rs` 重新冻结：战斗前多抽一次随机数，后面的抽取全部后移；
  种子 1 下这一场从同航戦变成反航戦，单发伤害 25 变 17，要打两发才击沉。
- 演习有两条测试断言交战形态固定为 1，改成只断言阵形、交战形态在 1 到 4 之间。
- 三道质量门。
