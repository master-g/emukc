# 連合艦隊 (Combined Fleet) Reference

> Source: [wikiwiki.jp/kancolle/連合艦隊](https://wikiwiki.jp/kancolle/%E9%80%A3%E5%90%88%E8%89%A6%E9%9A%8A),
> fetched 2026-09-21. Community-verified attack-order and correction tables.
> This file exists because `docs/battle/research.md` §8.2 deferred the tables
> ("具体修正表需参考 wikiwiki.jp 的完整数据") and §15.5 carried only their range.
>
> Two further sources are cited inline below:
> `docs/apilist.txt` (in-repo, 4209 lines) for protocol field semantics — the
> decoded client gives field *names*, this gives their *meaning*; and
> [KC3Kai/kancolle-replay](https://github.com/KC3Kai/kancolle-replay) `js/kcsim.js`
> (read 2026-09-21) as an independently maintained simulator. Its
> `COMBINEDCF1-4` (`kcsim.js:9-12`) and `COMBINEDCONSTS` (`kcsim.js:13-51`)
> reproduce every cell of §Formations and §Combined fleet corrections below,
> so those two tables have two independent confirmations.

A combined fleet sorties deck 1 (本隊 / main) and deck 2 (護衛 / escort) as one
12-ship force. `profile.combined_type` selects the flavour: `1` = 空母機動部隊
(carrier task force), `2` = 水上打撃部隊 (surface task force), `3` = 輸送護衛部隊
(transport escort). `0` means a normal single fleet.

## Formations (警戒航行序列)

Combined battles replace the six normal formations with four. Note the order is
partly reversed against normal formations: the 1st is the ASW formation and the
4th is the shelling formation.

| ID | Formation | Normal analogue | Eligibility (deck 2 size) |
|----|-----------|-----------------|---------------------------|
| 11 | 第一警戒航行序列 (対潜警戒) | 単横 | no limit |
| 12 | 第二警戒航行序列 (前方警戒) | 複縦 | no limit |
| 13 | 第三警戒航行序列 (輪形陣) | 輪形 | ≥ 5 |
| 14 | 第四警戒航行序列 (戦闘隊形) | 単縦 | ≥ 4 |

Eligibility depends on deck 2's size only — deck 1 may hold any number. Escort
withdrawal (護衛退避) can shrink deck 2 below a threshold mid-sortie.

The multipliers are identical across all three combined types; only deck 1's
lack of a torpedo phase differs. Deck 1 never torpedoes, so its 雷撃 column is
not applicable.

| Formation | 砲撃 | 雷撃 (deck 2 only) | 対潜 | 対空 |
|-----------|------|--------------------|------|------|
| 第一 (対潜) | 0.8 | 0.7 | 1.3 | 1.1 |
| 第二 (前方) | 1.0 | 0.9 | 1.1 | 1.0 |
| 第三 (輪形) | 0.7 | 0.6 | 1.0 | 1.5 |
| 第四 (戦闘) | 1.1 | 1.0 | 0.7 | 1.0 |

Accuracy is broadly lower than in a normal battle for both sides; the wiki's
per-cell accuracy figures are marked uncertain (`?`) and are not reproduced
here. Do not encode them as if they were verified.

Night-start cells use the six normal formations, not these four.

## Combined fleet corrections (連合艦隊補正)

The correction is an additive term inside basic attack power, alongside the
improvement bonus:

```
shelling (non-carrier) = base firepower + equip firepower + equip bonus
                       + improvement + COMBINED_CORRECTION + 5
torpedo                = base torpedo  + equip torpedo   + equip bonus
                       + improvement + COMBINED_CORRECTION + 5
```

It varies by which side is combined, the combined type, the attack class
(shelling / torpedo / aerial) and which deck is attacking.

### Friendly combined vs enemy single

Aerial: 0 for both sides in every case.
Torpedo: −5 for both sides in every case.

| Shelling | 空母機動 | 水上打撃 | 輸送護衛 |
|----------|---------|---------|---------|
| deck 1 — friendly | +2 | +10 | −5 |
| deck 1 — enemy | +10 | +5 | +10 |
| deck 2 — friendly | +10 | −5 | +10 |
| deck 2 — enemy | +5 | −5 | +5 |

(Before the 2015 autumn event 空母機動部隊 deck 1 was 0, not +2.)

### Friendly single vs enemy combined

Aerial: friendly side only — −10 against the enemy main fleet, −20 against the
enemy escort fleet.
Torpedo: +10 for both sides in every case.

| Shelling | friendly | enemy |
|----------|----------|-------|
| vs enemy main fleet | +5 | +10 |
| vs enemy escort fleet | +5 | −5 |

### Combined vs combined

Aerial: unverified upstream (要検証). Do not guess a value — keep it 0 and mark
the gap.
Torpedo: +10 for both sides in every case.

| Shelling | 空母機動 | 水上打撃 | 輸送護衛 |
|----------|---------|---------|---------|
| deck 1 — friendly | +2 | +2 | −5 |
| deck 1 — enemy | +10 | +10 | +10 |
| deck 2 — friendly | −5 | −5 | −5 |
| deck 2 — enemy | −5 | −5 | −5 |

Daytime ASW and night battle (excluding ASW) use the plain single-fleet values —
no combined correction at all. Night ASW uses the daytime formula.

## Phase order

Deck 1 never performs opening ASW or opening torpedo; deck 2 does both. A
second shelling round happens only when either side fields a battleship-class
ship.

### Friendly combined vs enemy single — 空母機動 / 輸送護衛

1. Aerial combat (deck 1 + deck 2 planes; air superiority decided here)
2. Deck 2 opening ASW
3. Deck 2 + enemy opening torpedo
4. Deck 2 shelling (single round)
5. Deck 2 torpedo
6. Deck 1 shelling → optional 2nd round
7. Night battle: deck 2 only

### Friendly combined vs enemy single — 水上打撃

Same as above except shelling order swaps: deck 1 shelling (→ optional 2nd
round) runs *before* deck 2 shelling, and deck 2 torpedo follows deck 2
shelling.

1. Aerial combat
2. Deck 2 opening ASW
3. Deck 2 + enemy opening torpedo
4. Deck 1 shelling → optional 2nd round
5. Deck 2 shelling (single round)
6. Deck 2 torpedo
7. Night battle: deck 2 only

### Friendly single vs enemy combined

The enemy escort fleet is engaged first, then the enemy main fleet.

1. Aerial combat (friendly fleet vs enemy main fleet)
2. Friendly opening ASW
3. Friendly + enemy opening torpedo (targets both enemy decks)
4. Friendly shelling vs enemy **escort** fleet (one round, by range)
5. Friendly + enemy escort torpedo (targets both enemy decks)
6. Friendly shelling vs enemy **main** fleet (one round, by range)
7. Friendly shelling vs **all** (one round, top-down) — only when either side
   fields a battleship-class ship
8. Night battle vs whichever enemy deck the selection rule picks

Unlike the friendly-combined case, the enemy escort fleet's recon planes do
contribute to contact (触接). When a friendly fleet is combined, deck 2's planes
do **not** contribute to contact against a single enemy.

### Combined vs combined — 空母機動 / 輸送護衛

1. Aerial combat (both decks of both sides participate — carriers, aviation
   cruisers and landing ships alike)
2. Deck 2 opening ASW
3. Deck 2 + enemy opening torpedo (targets all)
4. Deck 1 shelling vs enemy **main** fleet (one round, by range)
5. Deck 2 shelling vs enemy **escort** fleet (one round, by range)
6. Deck 2 torpedo (targets all)
7. Deck 1 shelling vs **all** (one round, top-down) — battleship condition
8. Night battle: deck 2 vs the selected enemy deck

### Combined vs combined — 水上打撃

Same set, reordered: deck 1's two shelling rounds run back to back before deck
2 acts.

1–3 as above, then deck 1 shelling vs enemy main → deck 1 shelling vs all
(battleship condition) → deck 2 shelling vs enemy escort → deck 2 torpedo →
night battle.

## Night battle opponent selection

When the enemy is combined, the night battle is fought against either the enemy
main fleet or its escort fleet. Score the enemy escort fleet:

- flagship alive: +1, sunk: +0
- every ship including the flagship: 小破 or better +1, 中破 +0.7,
  大破 or sunk +0

Total ≥ 3 → fight the escort fleet. Below 3 → fight the main fleet. If the main
fleet is already wiped out at night entry, the rule is ignored and the escort
fleet is the opponent.

Upstream marks as unverified: whether the flagship scores +1 at 中破 and +0.7 at
大破, whether ≥ 5 surviving escorts force an escort night battle, and whether PT
boats and submarines score differently.

## Protocol fields

The client reads these combined-only fields (confirmed in `main.decoded.js`
6.3.5.0):

`api_combined_flag`, `api_combined_type`, `api_f_maxhps_combined`,
`api_f_nowhps_combined`, `api_e_maxhps_combined`, `api_e_nowhps_combined`,
`api_fParam_combined`, `api_eParam_combined`, `api_eSlot_combined`,
`api_ship_ke_combined`, `api_ship_lv_combined`, `api_stage3_combined`,
`api_escape_idx_combined`, `api_combat_ration_combined`,
`api_mvp_combined`, `api_get_ship_exp_combined`, `api_get_exp_lvup_combined`.

The 14 endpoints the client calls: `battle`, `battle_water`, `airbattle`,
`ld_airbattle`, `ld_shooting`, `each_battle`, `each_battle_water`, `ec_battle`,
`ec_midnight_battle`, `ec_night_to_day`, `midnight_battle`, `sp_midnight`,
`battleresult`, `goback_port`.

### Which shelling round carries which deck

The phase order in §Phase order says *when* each deck fires; this says *which
packet field* carries it. **The two endpoints are mirror images — do not carry
one's mapping over to the other.**

`battle` (空母機動 / 輸送護衛, `docs/apilist.txt:3008`):

| Field | Participants |
|---|---|
| `api_hougeki1` | 第一次砲撃戦 — 味方随伴 (deck 2) vs 敵 |
| `api_raigeki` | 雷撃戦 |
| `api_hougeki2` | 第二次砲撃戦 — 味方本隊 (deck 1) vs 敵 |
| `api_hougeki3` | 第三次砲撃戦 — 味方本隊 (deck 1) vs 敵 |

`battle_water` (水上打撃, `docs/apilist.txt:3164`):

| Field | Participants | Gate |
|---|---|---|
| `api_hougeki1` | 第一次砲撃戦 — 味方本隊 (deck 1) vs 敵 | `api_hourai_flag[0]` |
| `api_hougeki2` | 第二次砲撃戦 — 味方本隊 (deck 1) vs 敵 | `api_hourai_flag[1]` |
| `api_hougeki3` | 第三次砲撃戦 — 味方随伴 (deck 2) vs 敵 | `api_hourai_flag[2]` |
| `api_raigeki` | 雷撃戦 | `api_hourai_flag[3]` |

Two consequences. In `battle`, **deck 1's two rounds are slots 2 and 3, not 1
and 2**, and the torpedo phase sits *between* deck 2's shelling and deck 1's —
the single-fleet hougeki1/hougeki2 intuition is wrong here. In `battle_water`
the assignment reverses and 雷撃 returns to last position.

`api_hourai_flag[n]` gates each field positionally by field name; only
`battle_water` documents it explicitly, but it is the same mapping the
single-fleet path already uses.

`api_raigeki`'s `api_frai` / `api_fcl` / `api_fdam` / `api_fydam` are each
`[12]`, independently confirming the continuous 0–11 friendly index space
derived from `main.decoded.js` in the plan's U1 section.

### Request parameters

`docs/apilist.txt` carries no `Request.api_req_combined_battle/*` entry except
`goback_port` (`(情報なし)`). Per the file's own convention
("記載のないものについては api_req_sortie/battle などと同様") the request body is
the single-fleet one at `docs/apilist.txt:2031`: `api_formation`,
`api_recovery_type`, `api_supply_flag`, `api_ration_flag`, `api_smoke_flag`.

### `battleresult` null semantics

`docs/apilist.txt:3864`: `api_mvp_combined`, `api_get_ship_exp_combined` and
`api_get_exp_lvup_combined` are **`null`** for a single fleet, not absent.
`api_get_ship_exp` is `[7 or 8]` and starts with a `-1` dummy, which the
existing single-fleet `calculate_sortie_ship_exp` already emits.

## Other mechanics

**護衛退避 (escort withdrawal)** removes a heavily damaged ship from deck 2 for
the rest of the sortie, taking its escort with it; the withdrawn indices are
reported in `api_escape_idx` / `api_escape_idx_combined`. This shrinks deck 2
and can disqualify formations 13 and 14.

**Flagship protection (旗艦援護 / かばう)** has no verified upstream rate —
every かばう cell in the wikiwiki formation tables is `?`. `kcsim.js:2309-2319`
supplies a usable fitted default instead:

```js
if (!target.isflagship || target.isInstall || target.isescort || ...) return target;
var rate = [0,.45,.6,.75,.6,.6,.75][target.fleet.formation.id];
if (!rate) rate = .6;   // formation 11-14 fall through to 60%
```

Two rules follow. **Deck 2's flagship is never protected** (`target.isescort`
returns before the rate is consulted) — this is structural, not a fitted number.
And **formations 11–14 intercept at 60%**, which is KC3's fallback, not observed
game data; cite it as such wherever it lands. Either is closer to the game than
`targeting.rs:154`'s current `None` for 11–14, which never intercepts at all.

**Radar-fire cells (レーダー射撃マス)** skip formation *selection*, but the
friendly side is not formation-less: `docs/apilist.txt:3833`
(`api_req_combined_battle/ld_shooting`) states 「味方は第四警戒航行序列(14)固定」
and that `api_search` is absent from the response. This supersedes the earlier
wikiwiki-derived note here, which said both sides enter with no formation.
