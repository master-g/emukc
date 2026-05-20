---
title: "feat: Complete carrier night CI (sp_list=6) — exempt ships, 8 sub-types, trigger rate"
type: feat
status: active
date: 2026-05-20
origin: docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md
parent_plans:
  - docs/plans/2026-05-14-002-feat-cut-in-attack-system-plan.md (U7 deferred)
  - docs/plans/2026-05-20-001-fix-ci-attack-audit-findings-plan.md (deferred follow-up)
---

# feat: Complete carrier night CI (sp_list=6)

## Summary

Complete the carrier night CI implementation deferred from plan 2026-05-14-002 U7. The current code stubs `NightAttackType::CarrierNightCI` but it never actually fires in battle, has no sub-types, no exempt-ship handling, and a flat 1.25x multiplier. This plan ships the full feature: priority chain over 8 sub-types, exempt-ship list, dedicated 種別係数 reusing existing trigger-rate formula, and integration into the night attack resolver.

All ship/item IDs and coefficients in this plan are **verified** against `.data/codex/start2.json` (Codex bootstrap snapshot) and authoritative wikiwiki/zekamashi formula documentation. No deferred-to-implementation lookups remain.

---

## Problem Frame

`NightAttackType::CarrierNightCI` (sp_list=6) was added in commit `18c4de0` as a placeholder. Three concrete defects today:

1. **Never fires.** `crates/emukc_battle/src/simulation/night.rs:112` sets `ci_coefficient()` to `0.0` for `CarrierNightCI`. `night_ci_trigger_rate` (line 432-437) returns `0.0` for any non-DoubleAttack type with zero coefficient. `resolve_night_attack` (line 501-503) rolls against this rate and always falls through to DoubleAttack/Normal. Detection succeeds, resolution fails — CV never produces sp_list=6 in battle output.
2. **No sub-types.** Spec defines 8 sub-types with multipliers 1.18x–1.25x by night-plane composition. Current code returns a flat 1.25x default.
3. **No exempt ships.** `is_cv_night_ci_eligible` (line 222) always requires `AVIATION_PERSONNEL_IDS` (item 258/259). Saratoga Mk.II (`mst_id=545`), 赤城改二戊 (`mst_id=599`), 加賀改二戊 (`mst_id=610`), 龍鳳改二戊 (`mst_id=883`) should bypass the personnel requirement when night planes are present. Saratoga Mk.II Mod.2 (`mst_id=550`) is **not exempt** — losing 夜戦特性 after upgrade.

Combined, this means carrier night CI is completely non-functional today — a regression from official server behavior.

(see origin: `docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md` R4, Phase 3)

---

## Requirements

- R1. CarrierNightCI fires in `resolve_night_attack` at a non-zero trigger rate, replacing today's stub-and-fail behavior. Covers AE: CV with valid night-plane setup produces `api_sp_list=6` in `BattleHougeki` output.
- R2. Detection routes to one of 8 priority-ordered sub-types based on night-plane composition; selected sub-type sets the damage multiplier (1.18x or 1.20x or 1.25x) and trigger-rate 種別係数 (105 / 120 / 130).
- R3. Exempt ships (Saratoga Mk.II `mst_id=545`, 赤城改二戊 `599`, 加賀改二戊 `610`, 龍鳳改二戊 `883`) trigger carrier night CI without `夜間作戦航空要員` (item 258/259). Saratoga Mk.II Mod.2 (`550`) is **not** exempt.
- R4. Carrier night CI reuses existing `night_ci_trigger_rate` formula but with sub-type-specific 種別係数 (105/120/130 per wikiwiki). No new formula code needed.
- R5. 光電管彗星 (彗星一二型(三一号光電管爆弾搭載機)) item ID `320` is detected by item ID and counts toward the 戦彗/攻彗/爆彗 sub-types. Item is `api_type[3]=7` (regular dive bomber icon), so icon-based detection alone misses it.
- R6. Output (`api_sp_list=6`, `api_si_list`, `api_df_list`, `api_cl_list`, `api_damage`) matches the CarrierNightCI single-hit shape; multiplier reflects sub-type.
- R7. Existing `is_cv_night_ci_eligible` test at `night.rs:1372` still passes (regression guard for detection layer).

---

## Scope Boundaries

- Ark Royal Swordfish night attack (a separate non-CI mechanic with different damage formula) — not in scope.
- 加賀改二護 (`mst_id=646`), Lexington/改, Wasp/改, Graf Zeppelin/改, 大鷹型改二, Saratoga (未改造) — these are Type Ⅰ "无条件 night battle" carriers that use the **traditional** night attack formula, NOT 夜襲CI. They are out of scope.
- Night aviation attack power formula (`A × 搭載数 + B × (火力+雷装+爆装+対潜) × √(搭載数) + √(★改修度)`) — only the CI multiplier is applied here; the underlying base damage continues to use existing `calculate_night_damage` until a separate formula plan lands.
- KongouNightAssault (sp_list=104) — separate plan.

### Deferred to Follow-Up Work

- Type Ⅰ 无条件 night battle carriers (Ark Royal, Lexington, Saratoga 未改, Graf Zeppelin, 大鷹型改二, 加賀改二護, Wasp) — separate dedicated plan; uses traditional 装備込み火力+雷装 formula.
- Night aviation attack power formula refinement (slot-capacity-based base damage with A/B coefficients and `夜間飛行機搭載補正`).
- Future game updates adding new exempt CVs — extend `EXEMPT_NIGHT_CV_IDS` at that time.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_battle/src/simulation/night.rs:46-114` — `NightAttackType::api_sp_list`, `damage_multiplier`, `ci_coefficient` (must extend for sub-types).
- `crates/emukc_battle/src/simulation/night.rs:194-218` — Existing constants `NIGHT_FIGHTER_ICON=45`, `NIGHT_ATTACKER_ICON=46`, `NIGHT_BOMBER_ICON=58`, `NIGHT_SUISEI_ICON=51`, `AVIATION_PERSONNEL_IDS=[258, 259]`, helper `count_night_planes_by_icon`. **All 4 icon constants align with wikiwiki — no new icon needed.**
- `crates/emukc_battle/src/simulation/night.rs:222-244` — `is_cv_night_ci_eligible` (extend with exempt ships).
- `crates/emukc_battle/src/simulation/night.rs:426-472` — `night_ci_trigger_rate` already implements the canonical 運/Lv formula with 配置/損傷/装備 modifiers. Reuse directly — no new formula function needed.
- `crates/emukc_battle/src/simulation/night.rs:475-517` — `resolve_night_attack` flow (must route CarrierNightCI through trigger roll, not the current dead path).
- `crates/emukc_battle/src/simulation/special_attack.rs:23-40` — Pattern for ship ID constants. Mirror this style for exempt CV IDs.
- `crates/emukc_battle/src/damage.rs::is_cv_type` — Used to gate carrier-only paths.

### Institutional Learnings

- DD CI sub-type pattern (`night.rs:251-290`) is the right template: internal enum (`DdCiType`) drives detection; resolved variant carries hit-count differentiation. Mirror this with `CarrierNightCiSubType`.
- Item-ID-based detection helper `has_slotitem_id` (used for `SKILLED_LOOKOUT_ID`) is the pattern for 光電管彗星, Swordfish, and 岩井爆戦.
- `count_night_planes_by_icon` already excludes shot-down slots (`onslot <= 0`). Reuse — do not re-implement.

### Verified Codex Data

| Concept | Codex source | Value | Notes |
|---|---|---|---|
| Saratoga Mk.II | `api_mst_ship[].api_id` | **545** | Verified `.data/codex/start2.json` |
| Saratoga Mk.II Mod.2 | `api_mst_ship[].api_id` | 550 | **NOT exempt** — loses 夜戦特性 |
| 赤城改二戊 | `api_mst_ship[].api_id` | **599** | Verified |
| 加賀改二戊 | `api_mst_ship[].api_id` | **610** | Verified |
| 龍鳳改二戊 | `api_mst_ship[].api_id` | **883** | Verified — added per wikiwiki recent update |
| 加賀改二護 | `api_mst_ship[].api_id` | 646 | Type Ⅰ 无条件 (different mechanic, out of scope) |
| 光電管彗星 (彗星一二型 三一号光電管爆弾搭載機) | `api_mst_slotitem[].api_id` | **320** | `api_type[3]=7` (regular bomber icon) |
| 零戦62型(爆戦/岩井隊) | `api_mst_slotitem[].api_id` | 154 | Counts as 夜間飛行機 only |
| Swordfish | `api_mst_slotitem[].api_id` | 242 | 夜間飛行機 |
| Swordfish Mk.II(熟練) | `api_mst_slotitem[].api_id` | 243 | 夜間飛行機 |
| Swordfish Mk.III(熟練) | `api_mst_slotitem[].api_id` | 244 | 夜間飛行機 |
| F4U-2 Night Corsair | `api_mst_slotitem[].api_id` | 473 | 夜戦 (icon 45) — auto-detected by icon |
| TBM-3W+3S | `api_mst_slotitem[].api_id` | 389 | 夜攻 (icon 46) — auto-detected |
| 九九式練爆二二型改 | `api_mst_slotitem[].api_id` | 552 | 夜爆 (icon 58) — auto-detected |
| 零式艦戦62型改(夜間爆戦) | `api_mst_slotitem[].api_id` | 557 | 夜爆 (icon 58) — auto-detected |
| 零式艦戦62型改(熟練/夜間爆戦) | `api_mst_slotitem[].api_id` | 558 | 夜爆 (icon 58) — auto-detected |

### External References

- `docs/plans/2026-05-14-002-feat-cut-in-attack-system-plan.md` U7 — original deferred work specification.
- `docs/apilist.txt:2319-2342` — sp_list authoritative mapping.
- wikiwiki 夜戦 page (`https://wikiwiki.jp/kancolle/夜戦`) — 8 sub-types table, 種別係数 (105/120/130), exempt ship list, equipment classification.
- zekamashi.net 空母夜戦カットイン (`https://zekamashi.net/kancolle-kouryaku/yasyuu-cutin/`) — confirmed Saratoga Mk.II Mod.2 loses 夜戦特性.

---

## Key Technical Decisions

- **CarrierNightCI carries sub-type information.** Replace flat `CarrierNightCI` with `CarrierNightCI(CarrierNightCiSubType)`. Damage multiplier and 種別係数 become functions of the sub-type, not the bare variant. Outputs still emit `sp_list=6` regardless of sub-type — sub-type only affects multiplier and trigger-rate denominator.
- **Reuse `night_ci_trigger_rate`.** The wikiwiki research confirms carrier night CI uses the **same** 運/Lv formula as standard night CI (`15 + 運 + 0.75×√Lv` for 運<50, `65 + √(運-50) + 0.8×√Lv` for 運≥50, with 配置/損傷/装備 modifiers). Only 種別係数 differs. Adding sub-type coefficient to `ci_coefficient()` is sufficient — no separate formula function needed. **This is a major simplification from initial plan.**
- **Exempt ships via constant slice.** `EXEMPT_NIGHT_CV_IDS: &[i64] = &[SARATOGA_MK2_ID, AKAGI_K2E_ID, KAGA_K2E_ID, RYUUHOU_K2E_ID]`. `is_cv_night_ci_eligible` short-circuits the personnel requirement when `EXEMPT_NIGHT_CV_IDS.contains(&ship.ship.api_ship_id)`. Plane requirement still enforced. **Saratoga Mk.II Mod.2 (550) explicitly NOT in this list.**
- **光電管彗星 by item ID, not icon.** Item 320 has `api_type[3]=7` (dive bomber icon), so `count_night_planes_by_icon(58)` does NOT match it. Need separate `count_kouden_suisei` helper checking `api_slotitem_id == 320`.
- **"夜間飛行機" is broader than icon coverage.** Counts: 夜戦 (icon 45) + 夜攻 (icon 46) + 夜爆 (icon 58) + 光電管彗星 (item 320) + Swordfish系 (items 242/243/244) + 岩井爆戦 (item 154). Used only for the priority-8 fallback "戦他他" (1.18x).
- **Priority chain checked top-down, first match wins.** 8 priorities. Once a sub-type matches, the trigger roll runs against that sub-type's coefficient. No multiroll.
- **Wikiwiki annotates 戦彗/攻彗/戦爆/攻爆/爆彗 coefficients with `120?` (uncertain).** Treat all five as 120 until contradicting evidence; document the uncertainty in code comment.

### Sub-Type Table (8 priorities, verified from wikiwiki)

| # | Codename | 夜戦 | 夜攻 | 夜爆 | 光電管彗星 | 夜間飛行機 (others) | Multiplier | 種別係数 |
|---|---|---|---|---|---|---|---|---|
| 1 | NF2NA (戦戦攻) | ≥2 | ≥1 | — | — | — | 1.25 | 105 |
| 2 | NF1NA (戦攻) | ≥1 | ≥1 | — | — | — | 1.20 | 120 |
| 3 | NF1KK (戦彗) | ≥1 | — | — | ≥1 | — | 1.20 | 120 |
| 4 | NA1KK (攻彗) | 0 | ≥1 | — | ≥1 | — | 1.20 | 120 |
| 5 | NF1NB (戦爆) | ≥1 | — | ≥1 | — | — | 1.20 | 120 |
| 6 | NA1NB (攻爆) | — | ≥1 | ≥1 | — | — | 1.20 | 120 |
| 7 | NB1KK (爆彗) | — | — | ≥1 | ≥1 | — | 1.20 | 120 |
| 8 | NF1OTHER (戦他他) | ≥1 | — | — | — | total 夜間飛行機 ≥2 (incl. self) | 1.18 | 130 |

Priority order is fixed top-to-bottom; first match selects the sub-type.

---

## Open Questions

### Resolved During Planning

- Q1. Is CarrierNightCI actually triggered today? **No** — `coefficient=0.0` makes the roll always fail (verified `night.rs:112,432-437,501-503`). This plan fixes that.
- Q2. Where does `is_cv_night_ci_eligible` live? `night.rs:222`, already filters by CV type + personnel + valid plane combo.
- Q3. Are night plane icon constants defined? Yes, `night.rs:194-197` — covers 夜戦/夜攻/夜爆/夜間瑞雲. **All 4 align with wikiwiki classifications.**
- Q4. Are exempt ships handled today? No — `has_personnel` always required.
- Q5. Are exempt ship IDs verified? **Yes** — Codex bootstrap confirms 545/599/610/883. Saratoga Mk.II Mod.2 (550) explicitly NOT exempt per zekamashi (loses 夜戦特性 after upgrade).
- Q6. Is 加賀改二護 (646) exempt? **No** — it's Type Ⅰ 无条件 night battle (different mechanic), uses traditional formula, not 夜襲CI. Out of scope.
- Q7. Is 光電管彗星 item ID 320? **Yes** — verified Codex; `api_type[3]=7` (NOT a night-plane icon — needs item-ID detection).
- Q8. Trigger rate formula: separate or shared with standard CI? **Shared** — wikiwiki confirms identical 運/Lv formula, only 種別係数 differs (105/120/130).
- Q9. Is 龍鳳改二戊 exempt? **Yes** — wikiwiki 2025/03 update. Verified `mst_id=883`.
- Q10. How many sub-types? **8** — wikiwiki latest. Initial plan had 5 from older zekamashi article; updated.
- Q11. Does `damage::is_cv_type` cover CVB? **Yes** — `damage.rs:268` matches `CV | CVL | CVB`. All 4 exempt IDs covered.
- Q12. Should the eligibility predicate stay independent of the sub-type detector? **No** — original `night.rs:238-243` predicate cannot recognize Nf1Kk/Na1Kk/Nb1Kk/Nf1Other compositions involving 光電管彗星 / Swordfish / 岩井爆戦. U1 delegates eligibility to the sub-type detector to avoid two divergent predicates.
- Q13. 夜間瑞雲 (icon=51) handling: 夜間瑞雲 is referenced by existing `is_cv_night_ci_eligible` (line 235 `count_night_planes_by_icon(NIGHT_SUISEI_ICON)`) but **none of the 8 wikiwiki sub-types reference 夜間瑞雲**. Decision: 夜間瑞雲 is **not part of the 8 carrier night CI sub-types**. The wikiwiki "夜間瑞雲夜戦カットイン" (sp_list=200) is a separate mechanic deferred per parent plan U7. After U1 delegates eligibility to the sub-type detector, 夜間瑞雲-only setups will return `None` from `detect_carrier_night_ci_sub_type` and fall through to standard night attack — correct behavior.

### Deferred to Implementation

- 戦彗/攻彗/戦爆/攻爆/爆彗 種別係数 marked `120?` by wikiwiki. Initial implementation uses 120; refine if integration tests reveal mismatch with verified battle samples.
- 装備補正 (探照灯 +7, 照明弾 +4, etc.) — these modifiers are **not** known to apply to carrier night CI (wikiwiki shows them only under standard night CI). Initial impl skips them; revisit if data emerges.
- Mid-damage (中破 +18) modifier — applies to standard 魚雷CI per wikiwiki. Whether it applies to carrier night CI is undocumented; initial impl reuses `night_ci_trigger_rate` chuuha logic which gives +5 for non-TorpTorpTorp types — acceptable approximation.

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification.*

```text
resolve_night_attack(ship)
  if is_submarine_target → Normal
  if DD → resolve_dd_night_attack (existing)
  detected = detect_night_attack_type(ship)
    if CV-eligible: detect_carrier_night_ci_sub_type(ship) → Some(sub) ⇒ CarrierNightCI(sub)
    else: existing path
  if detected == Normal | DoubleAttack → return as-is
  rate = night_ci_trigger_rate(ship, detected, is_flagship)   ← reused, sub-type provides coefficient via ci_coefficient()
  roll → detected | fallback (Normal for CV, existing for non-CV)

is_cv_night_ci_eligible(ship)
  if not is_cv_type → false
  has_personnel = AVIATION_PERSONNEL_IDS contains any slot
  is_exempt = EXEMPT_NIGHT_CV_IDS contains ship.api_ship_id
  if not (has_personnel || is_exempt) → false
  ...remaining plane combination checks (≥1 night fighter or attacker, etc.)

detect_carrier_night_ci_sub_type(ship) → Option<CarrierNightCiSubType>
  count nf, na, nb, kk, other_yakanhikouki
  if nf>=2 && na>=1 → Nf2Na
  else if nf>=1 && na>=1 → Nf1Na
  else if nf>=1 && kk>=1 → Nf1Kk
  else if nf==0 && na>=1 && kk>=1 → Na1Kk
  else if nf>=1 && nb>=1 → Nf1Nb
  else if na>=1 && nb>=1 → Na1Nb
  else if nb>=1 && kk>=1 → Nb1Kk
  else if nf>=1 && total_yakanhikouki>=2 → Nf1Other
  else None
```

---

## Implementation Units

### U1. Add exempt-ship handling and extend eligibility predicate

**Goal:** Saratoga Mk.II / 赤城改二戊 / 加賀改二戊 / 龍鳳改二戊 trigger carrier night CI eligibility without `航空要員`. Eligibility predicate also extended to recognize sub-types 3-8 plane combinations.

**Requirements:** R3, R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- Add ship ID constants near `AVIATION_PERSONNEL_IDS`:
  ```rust
  const SARATOGA_MK2_ID: i64 = 545;
  const AKAGI_K2E_ID: i64 = 599;
  const KAGA_K2E_ID: i64 = 610;
  const RYUUHOU_K2E_ID: i64 = 883;
  const EXEMPT_NIGHT_CV_IDS: &[i64] = &[
      SARATOGA_MK2_ID, AKAGI_K2E_ID, KAGA_K2E_ID, RYUUHOU_K2E_ID,
  ];
  ```
  IDs verified against Codex bootstrap (`.data/codex/start2.json`).
- Modify `is_cv_night_ci_eligible`:
  - Replace personnel check: `let qualifies = has_personnel || EXEMPT_NIGHT_CV_IDS.contains(&ship.ship.api_ship_id);`
  - **Replace** the existing plane combination check at `:238-243` (current `nf>=1 && (na||nb||suisei)` etc. — only recognizes icon-based combos and rejects Nf1Kk / Na1Kk / Nb1Kk / Nf1Other sub-types involving 光電管彗星 / Swordfish / 岩井爆戦). New predicate: simply delegate to the sub-type detector — if `detect_carrier_night_ci_sub_type(codex, ship).is_some()` returns true, eligibility succeeds. This avoids two redundant predicates and ensures eligibility ↔ detection are coherent.
  - Resulting structure: `is_cv_type` && `qualifies` (personnel or exempt) && `detect_carrier_night_ci_sub_type(...).is_some()`.
- Saratoga Mk.II Mod.2 (550) is **deliberately excluded** — add a doc comment explaining the upgrade-loses-夜戦特性 reasoning and citing `https://zekamashi.net/kancolle-kouryaku/yasyuu-cutin/`.
- Confirmed: `damage::is_cv_type` at `damage.rs:268` matches `CV | CVL | CVB` — covers all 4 exempt IDs.

**Test scenarios:**
- Happy: Saratoga Mk.II (mst=545) without 航空要員 + 1 night fighter + 1 night attacker → eligible.
- Happy: 赤城改二戊 (mst=599) without 航空要員 + 2 night fighters + 1 night attacker → eligible.
- Happy: 加賀改二戊 (mst=610) without 航空要員 + 1 night attacker + 1 光電管彗星 (item 320) → eligible (this is Na1Kk; old predicate would have rejected).
- Happy: 龍鳳改二戊 (mst=883) without 航空要員 + 1 night fighter + 2 Swordfish → eligible (this is Nf1Other; old predicate would have rejected).
- Edge: Saratoga Mk.II Mod.2 (mst=550) without 航空要員 → **not eligible** (regression guard against upgrade loophole).
- Edge: Saratoga (non-Mk.II, mst<545) without 航空要員 → not eligible.
- Edge: Standard CV with 航空要員 + valid Nf2Na combo → still eligible (regression guard for existing test at `night.rs:1372`).
- Edge: Saratoga Mk.II without any night plane → not eligible (sub-type detector returns None).

**Verification:** `cargo test -p emukc_battle is_cv_night_ci` passes; existing carrier night CI detection test stays green; new predicate + sub-type detector are tested as a coherent pair.

### U2. Define `CarrierNightCiSubType` and night plane detection helpers

**Goal:** Introduce sub-type enum (8 variants), item-ID detectors for non-icon-coverage planes, priority-chain detector. **No double-counting.**

**Requirements:** R2, R5

**Dependencies:** None (but co-developed with U1 since U1's eligibility delegates here)

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- Add item ID constants near `SKILLED_LOOKOUT_ID`:
  ```rust
  const KOUDENKAN_SUISEI_ID: i64 = 320; // 彗星一二型(三一号光電管爆弾搭載機) — type[3]=7, NOT a night-plane icon
  const IWAI_FUKUSHU_ID: i64 = 154;     // 零戦62型(爆戦/岩井隊)
  const SWORDFISH_IDS: &[i64] = &[242, 243, 244]; // Swordfish / Mk.II(熟練) / Mk.III(熟練)
  ```
  All IDs verified from Codex.
- Add helpers, **all applying `onslot > 0` guard** (mirror existing `count_night_planes_by_icon` at `:204-218`):
  ```rust
  fn count_kouden_suisei(ship) -> usize    // counts item 320 with onslot>0
  fn count_swordfish_iwai(ship) -> usize   // counts items 242/243/244/154 with onslot>0
  ```
  These two are **disjoint** from `count_night_planes_by_icon` results because their items have non-night-plane icons (光電管彗星 type[3]=7 dive bomber; Swordfish type[3]=8 torpedo bomber; 岩井爆戦 type[3]=7 dive bomber).
- Define internal enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum CarrierNightCiSubType {
      Nf2Na,    // 戦戦攻 1.25x, coeff 105
      Nf1Na,    // 戦攻   1.20x, coeff 120
      Nf1Kk,    // 戦彗   1.20x, coeff 120
      Na1Kk,    // 攻彗   1.20x, coeff 120
      Nf1Nb,    // 戦爆   1.20x, coeff 120
      Na1Nb,    // 攻爆   1.20x, coeff 120
      Nb1Kk,    // 爆彗   1.20x, coeff 120
      Nf1Other, // 戦他他 1.18x, coeff 130
  }
  ```
- `impl CarrierNightCiSubType { fn damage_multiplier(self) -> f64; fn coefficient(self) -> f64 }` returning the table values.
- Add `fn detect_carrier_night_ci_sub_type(codex, ship) -> Option<CarrierNightCiSubType>`. Walk priorities top-down with **disjoint counts**:
  ```rust
  let nf = count_night_planes_by_icon(NIGHT_FIGHTER_ICON);   // icon 45 only
  let na = count_night_planes_by_icon(NIGHT_ATTACKER_ICON);  // icon 46 only
  let nb = count_night_planes_by_icon(NIGHT_BOMBER_ICON);    // icon 58 only
  let kk = count_kouden_suisei(ship);                        // item 320 only
  let sf_iwai = count_swordfish_iwai(ship);                  // items 154/242/243/244 only
  // 夜間飛行機 (per wikiwiki) = nf + na + nb + kk + sf_iwai (no overlap)
  let total_yakanhikouki = nf + na + nb + kk + sf_iwai;

  if nf >= 2 && na >= 1: return Some(Nf2Na);
  if nf >= 1 && na >= 1: return Some(Nf1Na);
  if nf >= 1 && kk >= 1: return Some(Nf1Kk);
  if na >= 1 && kk >= 1: return Some(Na1Kk);
  if nf >= 1 && nb >= 1: return Some(Nf1Nb);
  if na >= 1 && nb >= 1: return Some(Na1Nb);
  if nb >= 1 && kk >= 1: return Some(Nb1Kk);
  if nf >= 1 && total_yakanhikouki >= 2: return Some(Nf1Other);
  None
  ```
  Each priority's predicate is positive-only (no `nf == 0` exclusions); the top-down chain naturally selects the highest-priority match.

**Test scenarios:**
- Happy: 2× F6F-3N (item 254, icon 45) + 1× TBM-3D (item 257, icon 46) → `Nf2Na`.
- Happy: 1× 夜戦 + 1× 夜攻 + 1× 夜爆 → `Nf1Na` (priority 2 wins).
- Happy: 1× 夜戦 + 1× 光電管彗星 (item 320) → `Nf1Kk`.
- Happy: 0× 夜戦 + 1× 夜攻 + 1× 光電管彗星 → `Na1Kk`.
- Happy: 1× 夜戦 + 0× 夜攻 + 1× 夜爆 → `Nf1Nb`.
- Happy: 0× 夜戦 + 1× 夜攻 + 1× 夜爆 → `Na1Nb`.
- Happy: 0× 夜戦/夜攻 + 1× 夜爆 + 1× 光電管彗星 → `Nb1Kk`.
- Happy: 1× 夜戦 + 2× Swordfish → `Nf1Other` (1.18x). total = 1+0+0+0+2 = 3 ≥ 2.
- Happy: 1× 夜戦 + 1× 光電管彗星 + 1× Swordfish → `Nf1Kk` (priority 3 wins; Nf1Other tie-broken by priority).
- Edge: 1× 夜戦 only → `None` (no sub-type — total = 1, fails Nf1Other).
- Edge: 光電管彗星 with onslot=0 → `kk = 0`, not counted.
- Edge: Swordfish with onslot=0 → `sf_iwai = 0`, not counted.
- Edge: 夜攻 + 光電管彗星 with no 夜戦 → `Na1Kk` (priority 4; priority 3 fails because nf=0).

**Verification:** `cargo test -p emukc_battle carrier_night_ci_subtype` covers each sub-type and priority precedence; disjoint-count assertions verify no double counting.

### U3. Carry sub-type through `NightAttackType` and audit all callers

**Goal:** `CarrierNightCI` carries `CarrierNightCiSubType`; multiplier and coefficient are per sub-type. Update **all 8 reference sites** in `night.rs`.

**Requirements:** R2, R6

**Dependencies:** U2

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- Change variant: `CarrierNightCI` → `CarrierNightCI(CarrierNightCiSubType)`.
- **Full audit of `CarrierNightCI` reference sites** (verified via `rg`, all in `night.rs`):
  | Site | What to update |
  |------|---------------|
  | `:42` enum declaration | Variant becomes tuple-shaped |
  | `:62` `api_sp_list` arm | `CarrierNightCI(_) => 6` |
  | `:78` `damage_multiplier` arm | `CarrierNightCI(sub) => sub.damage_multiplier()` |
  | `:92` `hit_count` arm (`\|`-fused with other 1-hit variants) | **Split** the fusion: `CarrierNightCI(_) => 1`, then keep other arms |
  | `:112` `ci_coefficient` arm (`\|`-fused with `DoubleAttack \| Normal`) | **Split** the fusion: `CarrierNightCI(sub) => sub.coefficient()`, then `DoubleAttack \| Normal => 0.0` |
  | `:396` `detect_night_attack_type` return | Return `CarrierNightCI(sub)` from sub-type detector result (see U4) |
  | `:596` `night_attack_display_ids` arm (`\|`-fused with `Normal`) | **Split** the fusion: `CarrierNightCI(_) => extend_limit(...) \| Normal => extend_limit(...)` |
  | `:1372` test assertion | `assert!(matches!(attack, NightAttackType::CarrierNightCI(_)))` |
- The three `\|`-fused arms (`:92`, `:112`, `:596`) are the highest-risk sites — fusion silently groups CarrierNightCI with semantically unrelated variants and may compile if the body is identical, hiding a logic merge.

**Patterns to follow:** DD CI variants already pair base type + 2-hit flag. Sub-type as enum payload is the same shape applied to a single variant.

**Test scenarios:**
- Happy: `CarrierNightCI(Nf2Na).damage_multiplier()` == 1.25 and `.ci_coefficient()` == 105.
- Happy: `CarrierNightCI(Nf1Other).damage_multiplier()` == 1.18 and `.ci_coefficient()` == 130.
- Happy: All 8 sub-types `.api_sp_list()` == 6.
- Regression: existing test at `night.rs:1372` still passes after re-shaping (must update to tuple-variant pattern match).

**Verification:** `cargo test -p emukc_battle` passes; `cargo build` clean (no missing match arms — exhaustiveness checker enforces full coverage).

### U4. Integrate into `resolve_night_attack` (no new trigger formula)

**Goal:** Replace dead-rate path with reuse of existing `night_ci_trigger_rate` parameterized by sub-type coefficient. Update `detect_night_attack_type` to return `CarrierNightCI(sub)` directly.

**Requirements:** R1, R4

**Dependencies:** U2, U3

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- In `detect_night_attack_type` at `:395-397`, replace:
  ```rust
  if is_cv_night_ci_eligible(codex, ship) {
      return NightAttackType::CarrierNightCI;
  }
  ```
  with:
  ```rust
  if is_cv_night_ci_eligible(codex, ship) {
      if let Some(sub) = detect_carrier_night_ci_sub_type(codex, ship) {
          return NightAttackType::CarrierNightCI(sub);
      }
      // Eligible but no sub-type combo (shouldn't happen if U1 predicate delegates to detector;
      // defensive Normal fallback.)
  }
  ```
- In `resolve_night_attack` at `:475-517`, the existing trigger roll path **already** calls `night_ci_trigger_rate(ship, detected, is_flagship)` and rolls. With U3's `ci_coefficient` returning 105/120/130 instead of 0.0, the formula now produces a valid rate. **No new trigger function needed.**
- For CV failure path (CI fails to roll): the existing `else` branch at `:506` falls into `is_double_attack_eligible` check then either `DoubleAttack` or `Normal`. **Add early-return inside the `else` branch, before the double-attack check**:
  ```rust
  } else {
      // CV failed CI -> Normal (CV does not artillery double-attack at night)
      if matches!(detected, NightAttackType::CarrierNightCI(_)) {
          return NightAttackType::Normal;
      }
      // Existing DD/standard fallback
      let main_guns = count_main_guns(codex, ship);
      ...
  }
  ```

**Test scenarios:**
- Happy: CV with valid Nf2Na setup at flagship + Lv99 luck=20 → trigger rate via existing formula with coefficient=105: `(15 + 20 + 7 + 15) / 105 = 0.543 ≈ 54.3%`.
- Happy: Higher luck → higher trigger rate (existing formula behavior).
- Happy: Flagship → +15 bonus reflected (existing formula behavior).
- Edge: Same setup but `Nf1Other` sub-type at coefficient 130 → `(15+20+7+15)/130 = 0.438 ≈ 43.8%` — lower than Nf2Na as expected.
- Edge: CI roll fails → `Normal`, NOT `DoubleAttack` (CV-specific early return).
- Integration: Lv99 luck=99 setup: `(65 + sqrt(49) + 8 + 15) / 105 = 95/105 = 0.905`. With seed-controlled RNG drawing 0.5 → CI fires; drawing 0.95 → fails to Normal.

**Verification:** `cargo test -p emukc_battle carrier_night_ci_trigger` passes; integration test confirms `api_sp_list=6` appears in hougeki output for valid CVs at the predicted rate.

### U5. End-to-end battle integration test

**Goal:** A real night-battle simulation produces `api_sp_list=6` with the correct sub-type multiplier for representative CV configurations. Use **rate-forcing** (high luck + Lv99) to make trigger rate clamp to 1.0, avoiding flaky seed-window dependence.

**Requirements:** R1, R2, R3, R6

**Dependencies:** U1, U2, U3, U4

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs` (test section)

**Approach:**
- **Rate-forcing strategy.** Existing `night_ci_trigger_rate` returns `total / coefficient` clamped to `[0.0, 1.0]`. To force CI to fire deterministically: set `api_lucky[1] = 99`, `api_lv = 99`, `is_flagship = true`. For Nf1Other (worst case, coefficient=130): `(65 + sqrt(49) + 8 + 15) / 130 = 95/130 = 0.731`. To exceed 1.0, use `api_lucky[1] = 200` → `(65 + sqrt(150) + 8 + 15) / 130 ≈ (65+12+23)/130 = 0.769` — still < 1.0. Better approach: set luck high enough that `total / 130 ≥ 1.0`. With luck=300: `(65 + sqrt(250) + 8 + 15)/130 = (65+15+23)/130 = 0.79`. Luck cannot reliably push past 1.0 alone — instead, use `MockBattleRng` returning `random_f64_range = 0.0` so any non-zero rate fires. Add this mock in the test module if not already available.
- **Alternative**: existing carrier night CI test at `night.rs:1361-1372` already constructs CV fixtures via `BattleRuntimeShip::from(BattleShipInput)`. Extend this pattern.
- Build `BattleRuntimeShip` fixtures via `test_utils.rs` helpers (`sample_ship`, `slotitem_with_mst_id`, `first_ship_mst_by_type`):
  - Standard CV with 航空要員 (item 258) + Nf2Na setup (F6F-3N×2 item 254 + TBM-3D item 257)
  - Saratoga Mk.II (mst=545) without 航空要員 + Nf1Na (F6F-3N + TBM-3D)
  - 加賀改二戊 (mst=610) without 航空要員 + Na1Kk (TBM-3D + 光電管彗星 item 320)
  - 龍鳳改二戊 (mst=883) without 航空要員 + Nf1Other (F6F-3N + Swordfish item 242 + Swordfish Mk.II 243)
  - Saratoga Mk.II Mod.2 (mst=550) without 航空要員 + Nf1Na — should fail eligibility (negative case)
  - CV with personnel but no night planes (negative case)
- Use **always-fire RNG mock** (`random_f64_range` returns 0.0) to make CI roll deterministically succeed when rate > 0; use **always-fail RNG mock** (returns 0.999) to verify failure-fallback path returns Normal not DoubleAttack.
- Run the night-battle entry point that exercises `resolve_night_attack` + `night_attack_display_ids` (existing test at `:855` `night_battle_sp_list_nonzero_for_ci_ship` is the closest pattern).
- Assert: `api_sp_list[idx] == 6`, damage applied with correct multiplier, `api_si_list` non-empty.

**Test scenarios:**
- Covers AE: Standard CV + 航空要員 + Nf2Na → `sp_list=6`, multiplier 1.25x in damage path.
- Covers AE: Saratoga Mk.II (no personnel) + Nf1Na → `sp_list=6`, multiplier 1.20x.
- Covers AE: 加賀改二戊 + Na1Kk path with 光電管彗星 → `sp_list=6`, multiplier 1.20x.
- Covers AE: 龍鳳改二戊 + Nf1Other (Swordfish) → `sp_list=6`, multiplier 1.18x.
- Negative: Saratoga Mk.II Mod.2 (mst=550) without 航空要員 → no `sp_list=6`, falls back to standard night attack.
- Negative: CV with 航空要員 but only night fighters (no attacker, no 光電管彗星, no other night plane) → no `sp_list=6` (Nf only → total_yakanhikouki=1, fails Nf1Other).
- Negative: Non-exempt CV without 航空要員 → no `sp_list=6`.
- Failure path: Standard CV + valid Nf2Na + always-fail RNG → returns `Normal`, NOT `DoubleAttack` (CV-specific early-return verification).

**Verification:** `cargo test -p emukc_battle carrier_night_ci_integration` passes. Manually inspect one test's hougeki output to confirm shape.

### U6. Documentation and codex helper

**Goal:** Capture verified ship/item IDs, document sub-type table, link wikiwiki/zekamashi sources.

**Requirements:** R7

**Dependencies:** U1, U2

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs` (doc comments on new constants and types)

**Approach:**
- Add doc comments to `EXEMPT_NIGHT_CV_IDS` listing each ship's `mst_id`, remodel name, and citing wikiwiki source.
- Add doc comment to `KOUDENKAN_SUISEI_ID` explaining `api_type[3]=7` (NOT a night-plane icon — item-ID detection mandatory).
- Add doc comment to `CarrierNightCiSubType` enum referencing the sub-type table from this plan and citing wikiwiki 夜戦 page.
- Note in code comment that 戦彗/攻彗/戦爆/攻爆/爆彗 種別係数 is `120?` (uncertain per wikiwiki); revisit if real-world battle samples contradict.
- If the same ship-ID-set pattern recurs (already does in `special_attack.rs`), note in Deferred to consider extracting to a shared helper.

**Test expectation:** none — pure documentation.

**Verification:** `cargo doc -p emukc_battle` builds without warnings on new items.

---

## System-Wide Impact

- **Interaction graph:** `resolve_night_attack` gains a CV-specific branch. Carrier ships now have a working CI path for the first time. Standard CI flow for non-CV ships is unchanged.
- **API surface parity:** Battle responses for CV night attacks now emit `api_sp_list=6` matching official server output. Animation parity restored.
- **Damage parity:** CV night attack damage now reflects the 1.18x–1.25x sub-type multipliers (pre-cap, before night battle's 300 cap), increasing average damage output relative to the broken stub.
- **Persistent state:** None. All state computed within battle simulation.
- **Unchanged invariants:** Standard night CI (sp_list 0-5), DD CI (7-14), special attacks (100-106), torpedo phase, kouku phase — all untouched.

---

## Risks & Dependencies

| Risk | Mitigation |
|---|---|
| Ship IDs for exempt CVs may differ between codex versions | All 4 IDs (545/599/610/883) verified against `.data/codex/start2.json` snapshot; Codex bootstrap pins these. Add a regression test asserting each constant resolves to a real `ApiMstShip` so future codex schema drift fails loud |
| 光電管彗星 item ID 320 may differ post-codex-update | Verified Codex; integration test in U5 will catch a mis-ID since detection won't match |
| 戦彗/攻彗/戦爆/攻爆/爆彗 種別係数 marked uncertain (120?) by wikiwiki | Initial value 120 documented as approximation; integration test ground truth refines if needed. Worst case: rates skewed but functional |
| Saratoga Mk.II Mod.2 (550) accidental inclusion in exempt list | Negative test in U1 explicitly asserts mst=550 is NOT eligible without 航空要員 |
| Existing eligibility predicate (`night.rs:238-243`) is stricter than needed and rejects sub-types 3-8 | U1 replaces it by delegating to `detect_carrier_night_ci_sub_type` so eligibility ↔ detection are coherent |
| Helper double-counting (光電管彗星 in both `kk` and `count_other_yakanhikouki`) | U2 splits helpers: `count_kouden_suisei` (item 320 only) and `count_swordfish_iwai` (items 154/242/243/244 only) — disjoint sets |
| Match arm sprawl after re-shaping `CarrierNightCI(SubType)` — risk of silent fusion merges | U3 explicitly enumerates all 8 sites with notes on the three `\|`-fused arms (`:92`, `:112`, `:596`) that must be split |
| Existing `night.rs:1372` test asserts `attack == NightAttackType::CarrierNightCI` (unit variant) | Must update assertion to `matches!(attack, CarrierNightCI(_))` pattern; flagged as regression guard in U3 |
| Carrier night CI fallback should NOT use DoubleAttack but might if not gated | U4 adds explicit early-return inside the `else` branch at `:506` for `CarrierNightCI(_)` failure → `Normal` |
| Integration test seed-window dependence is flaky | U5 uses always-fire / always-fail RNG mocks instead of seed engineering; existing pattern at `:855` confirms this approach works |
| 夜爆 icon detection collision with non-夜爆 type-58 items | Verified `api_type[3]=58` items in Codex are exactly the three 夜爆 + 零式艦戦62型改 variants — no collision |
| 夜間瑞雲 (icon=51) ships may surface bugs | None of 8 sub-types reference 夜間瑞雲; sub-type detector returns `None` → falls through to standard night attack. Documented in Q13 |

---

## Sources & References

- Origin: `docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md` (R4, Phase 3)
- Parent plan: `docs/plans/2026-05-14-002-feat-cut-in-attack-system-plan.md` (U7 — original deferred work)
- Companion plan: `docs/plans/2026-05-20-001-fix-ci-attack-audit-findings-plan.md` (Deferred to Follow-Up Work)
- Implementation reference: `crates/emukc_battle/src/simulation/night.rs:222-244` (eligibility), `:251-290` (DD CI sub-type pattern template), `:426-472` (existing trigger rate formula being reused)
- API reference: `docs/apilist.txt:2319-2342`
- Authoritative formula source: `https://wikiwiki.jp/kancolle/夜戦` — 8 sub-types, 種別係数 105/120/130, exempt ship list including 龍鳳改二戊
- Cross-source: `https://zekamashi.net/kancolle-kouryaku/yasyuu-cutin/` — confirmed Saratoga Mk.II Mod.2 loses 夜戦特性
- Codex snapshot: `.data/codex/start2.json` (verified 2026-05-20)
