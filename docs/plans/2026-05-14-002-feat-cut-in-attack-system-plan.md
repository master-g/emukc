---
title: "feat: Implement Cut-In attack system (day CI, DD night CI, carrier CI, special attacks)"
type: feat
status: active
date: 2026-05-14
origin: docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md
---

# feat: Implement Cut-In attack system

## Summary

Implement the complete Cut-In attack system across day and night battle phases. Work is sequenced in 4 phases by difficulty: Phase 1 — DD Night CI (U1-U2), Phase 2 — Day Artillery Spotting (U3-U5), Phase 3 — Carrier CI (U6-U7), Phase 4 — Flagship Special Attacks (U8-U10). Each phase follows the existing `night.rs` pattern of detect → trigger-roll → resolve.

---

## Problem Frame

Day shelling outputs only `api_at_type=0` (normal) or `7` (ASW). Night battle lacks DD CI (sp_list 7-14), carrier night CI, and all flagship special attacks. The client cannot play CI animations and damage calculations miss CI multipliers. (see origin: `docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md`)

---

## Requirements

- R1. Day shelling implements 弾着観測射撃 (api_at_type 2-6) with air superiority gate, seaplane condition, trigger rate, and damage multiplier
- R2. Day shelling implements carrier CI (api_at_type 7)
- R3. Night battle extends with DD CI (api_sp_list 7-14)
- R4. Night battle implements carrier night CI (api_sp_list 6)
- R5. Day/night implement flagship special attacks (api_at_type/sp_list 100-106)
- R6. All CI trigger rates are high-fidelity reproductions of official formulas
- R7. All CI types output correct api_at_type/sp_list/si_list/df_list/cl_list/damage for client animation

---

## Scope Boundaries

- レーザー攻撃 (at_type=1): deprecated, not implemented
- 瑞雲立体攻撃 (200), 海空立体攻撃 (201): deferred
- 潜水艦隊攻撃 (300-302): deferred
- 夜間瑞雲夜戦CI (200): deferred
- 対空CI (api_air_fire): separate system, not in scope

### Deferred to Follow-Up Work

- 瑞雲/海空立体攻撃: requires 瑞雲 equipment data completion
- 潜水艦隊攻撃: requires submarine fleet composition system
- 金剛改二丙型僚艦夜戦突撃 (sp_list 104): requires specific ship data verification

---

## Context & Research

### Official API Reference (apilist.txt)

`docs/apilist.txt` lines 2252-2270 (day at_type) and 2319-2342 (night sp_list) define the authoritative integer mappings:

**api_at_type (day):**
- 0=通常攻撃, 2=連撃, 3=主副カットイン, 4=主電カットイン, 5=主徹カットイン, 6=主主カットイン, 7=空母カットイン

**api_sp_list (night):**
- 0=通常攻撃, 1=連続射撃
- 2=カットイン(主砲/魚雷), 3=カットイン(魚雷/魚雷), 4=カットイン(主砲/主砲/副砲), 5=カットイン(主砲/主砲/主砲)
- 6=空母カットイン
- 7=駆逐カットイン(主砲/魚雷/電探), 8=駆逐カットイン(魚雷/見張員/電探), 9=駆逐カットイン(魚雷/水雷見張員/魚雷), 10=駆逐カットイン(魚雷/水雷見張員/ドラム缶)
- 11-14=2hit variants of 7-10
- 100-106=flagship special attacks, 200=夜間瑞雲, 300-302=潜水艦隊, 400-401=大和型特殊

### Pre-Existing Bug: Night CI sp_list Mapping Reversed

The existing `night.rs` `api_sp_list()` method maps incorrectly:
- `MainMainMain → 2` (should be **5**)
- `MainMainSec → 3` (should be **4**)
- `TorpTorpTorp → 4` (should be **3**)
- `MainTorpRadar → 5` (should be **2**)

The 2↔5 and 3↔4 pairs are swapped. Must be fixed before adding new CI types. Added as U0.

### Relevant Code and Patterns

- `crates/emukc_battle/src/simulation/night.rs` — complete night CI implementation to mirror: `NightAttackType` enum, `detect_night_attack_type()`, `night_ci_trigger_rate()`, `resolve_night_attack()`, `night_attack_display_ids()`
- `crates/emukc_battle/src/simulation/shelling.rs` — day shelling loop, currently hardcodes `at_type=0/7`
- `crates/emukc_battle/src/damage.rs` — `calculate_shelling_damage()` needs CI multiplier parameter
- `crates/emukc_battle/src/types/domain.rs:177` — `ShellingParams` struct (needs `air_state` field)
- `crates/emukc_battle/src/simulation/kouku.rs` — stores `api_disp_seiku` in kouku stage1
- `crates/emukc_battle/src/state.rs` — `BattleState` holds `kouku: Option<BattleKouku>`
- `crates/emukc_model/src/kc2/types/slotitem.rs` — `KcSlotItemType3` enum (SeaplanePersonnel=39 covers 見張員, TransportContainer=30 covers ドラム缶)

---

## Key Technical Decisions

- **Day CI in separate module**: Create `crates/emukc_battle/src/simulation/day_cutin.rs` with `DayAttackType` enum + `detect_day_attack_type()` + `day_ci_trigger_rate()` + `resolve_day_attack()`. Same architectural shape as night.rs, different mechanics. Shelling loop calls into this module.
- **Day CI equipment conditions (verified from wikiwiki/zekamashi)**:
  - at_type=6 主主徹: 2× main gun + AP shell (type3=19)
  - at_type=5 主副徹: 1× main gun + 1× secondary gun (type3=4/95) + AP shell (type3=19)
  - at_type=4 主副電: 1× main gun + 1× secondary gun (type3=4/95) + radar (type3=12/13/93)
  - at_type=3 主副: 1× main gun + 1× secondary gun (type3=4/95)
  - at_type=2 連撃: 2× main gun (fallback when CI roll fails)
  - All require air_state AS/AS+ AND seaplane with onslot>0
- **Day CI damage multipliers are post-cap** (applied after daytime shelling soft cap of 180):
  - 主主徹=1.5x, 主副徹=1.3x, 主副電=1.2x, 主副=1.1x, 連撃=1.2x×2hits
  - Accuracy bonuses: 主主徹=1.20x, 主副徹=1.30x, 主副電=1.50x, 主副=1.30x, 連撃=1.10x
- **Pass air_state via ShellingParams**: Add `air_state: Option<AirState>` to `ShellingParams`. Populated from `state.kouku.api_stage1.api_disp_seiku` before shelling phases execute.
- **DD CI detection uses KcSlotItemType3**: 見張員 = `SeaplanePersonnel` (type3=39), ドラム缶 = `TransportContainer` (type3=30). No need for item-ID matching for basic detection. 水雷戦隊熟練見張員 specifically needs item-ID check (it's a subset of type3=39, item ID 412).
- **DD CI uses multiroll mechanism**: DD CI types are rolled independently in order GTR→TRL→TTL→DTL. If all fail, standard night CI is then rolled. NOT a priority chain like original CI.
- **DD CI trigger rate uses Level-based formula**: Different from standard night CI's Luck-based formula. Luck<50: `0.75×sqrt(Level)+Luck`, Luck≥50: `0.80×sqrt(Level)+50+sqrt(Luck-50)`. Divided by per-type base_attack threshold.
- **DD CI D-type gun bonuses**: GTR/TRL get multiplicative bonuses when 12.7cm連装砲D型改二/改三 equipped. GTR+D2=1.625x (vs base 1.3x), GTR+D3=1.706x, etc. Only certain DD classes can equip 2× D-type guns.
- **Special attacks in separate module**: `crates/emukc_battle/src/simulation/special_attack.rs` — complex enough to warrant isolation. Called before normal shelling when conditions met.
- **Damage multiplier timing differs by phase**: Day battle CI multipliers applied post-cap (after 180 cap). Night battle CI multipliers applied pre-cap (before 300 cap). Special attack multipliers: day=post-cap, night=pre-cap. This means `calculate_shelling_damage` applies CI multiplier AFTER the cap check, while `calculate_night_damage` applies BEFORE the cap check.

---

## Open Questions

### Resolved During Planning

- **Q1. Air state available to shelling?** Yes — `state.kouku.api_stage1.api_disp_seiku` stores it. Need to extract and pass via `ShellingParams`.
- **Q2. 見張員 type3?** `SeaplanePersonnel = 39`. 水雷戦隊熟練見張員 is a specific item within that type3 — needs item-ID check (item ID 412 per NGA source).
- **Q3. Day CI equipment conditions?** Verified from wikiwiki/zekamashi: at_type=6 needs AP shell, at_type=5 needs secondary+AP, at_type=4 needs secondary+radar. See KTD.
- **Q4. Day CI multiplier timing?** Post-cap (after 180), confirmed from en.kancollewiki.net and zekamashi.
- **Q5. DD CI trigger mechanism?** Multiroll (GTR→TRL→TTL→DTL), not priority chain. Uses Level-based formula.
- **Q6. Night CI standard multipliers?** 主主主=2.0x, 主主副=1.75x, 魚魚=1.5x, 主魚=1.3x, 連撃=1.2x×2 (verified from en.kancollewiki.net)
- **Q7. Carrier night CI sp_list?** sp_list=6 per apilist.txt. Has 5 sub-types based on night fighter/night attacker combinations.
- **Q8. Richelieu/QE ship count?** Both are 3-ship attacks (positions 1,2,3), not 2-ship. Verified from kankorekore table.
- **Q9. Nelson Touch applicable ships?** Nelson OR Rodney (any remodel). Colorado applicable ships: Colorado OR Maryland (any remodel).

### Deferred to Implementation

- Precise flagship special attack ship ID lists (need Codex data verification)
- DD CI D-type gun bonus equipment ID list (12.7cm連装砲D型改二/改三 specific item IDs)
- Night fighter/night attacker specific item IDs for carrier night CI sub-type detection
- 光電管彗星 (彗星一二型三一号光電管爆弾搭載機) item ID for carrier night CI type 3

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification.*

```
Day Battle Flow (with CI):
  kouku phase → air_state determined
  ↓
  shelling phase:
    for each attacker:
      1. check special_attack eligibility → if yes, execute special attack
      2. else: detect_day_attack_type(ship, air_state) → DayAttackType
      3. if CI detected: roll trigger rate → success/fallback
      4. calculate damage with CI multiplier
      5. output api_at_type + multi-hit if applicable

Night Battle Flow (extended):
  for each attacker:
    1. detect_night_attack_type(ship) → NightAttackType (now includes DD CI + carrier CI)
    2. roll trigger → success/fallback chain
    3. calculate damage with multiplier
    4. output api_sp_list
```

---

## Phased Delivery

### Phase 0: Fix pre-existing sp_list bug (U0)
Must be fixed first — existing night CI sends wrong animations to client.

### Phase 1: DD Night CI (U1-U2)
Extends existing night.rs. Lowest risk, fastest delivery.

### Phase 2: Day Artillery Spotting (U3-U5)
New system but clear mechanics. Requires air_state plumbing.

### Phase 3: Carrier CI (U6-U7)
Day + night carrier CI. Moderate complexity.

### Phase 4: Flagship Special Attacks (U8-U10)
Most complex. Multi-ship coordination, specific ship ID conditions.

---

## Implementation Units

### U0. Fix pre-existing night CI sp_list mapping bug

**Goal:** Correct the reversed sp_list values in existing `night.rs` so client plays correct CI animations.

**Requirements:** R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- Fix `api_sp_list()` mapping: `MainMainMain → 5`, `MainMainSec → 4`, `TorpTorpTorp → 3`, `MainTorpRadar → 2`
- Per apilist.txt: 2=主砲/魚雷, 3=魚雷/魚雷, 4=主砲/主砲/副砲, 5=主砲/主砲/主砲
- Also verify `night_attack_display_ids()` returns correct si_list for each type

**Patterns to follow:**
- `docs/apilist.txt` lines 2319-2342 is authoritative

**Test scenarios:**
- Regression: existing night battle tests still pass
- New test: assert each `NightAttackType` variant maps to correct sp_list integer per apilist.txt

**Verification:** `cargo test -p emukc_battle` passes; sp_list values match apilist.txt

---

### U1. Extend NightAttackType with DD CI variants

**Goal:** Add DD-specific night CI types (sp_list 7-14) to the existing night battle system.

**Requirements:** R3, R6, R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`
- Modify: `crates/emukc_battle/src/targeting.rs` (if DD-type gate needed)

**Approach:**
- Extend `NightAttackType` enum with: `DdMainTorpRadar`(7/11), `DdTorpLookoutRadar`(8/12), `DdTorpSkilledLookoutTorp`(9/13), `DdTorpSkilledLookoutDrum`(10/14) — 1-hit and 2-hit variants share the same detection logic, sp_list determined by hit count after trigger succeeds
- Add detection logic in `detect_night_attack_type`: gate on ship type == DD, then check equipment combinations
- **Multiroll mechanism:** DD CI types are rolled independently in order: GTR→TRL→TTL→DTL. If all DD CI rolls fail, standard night CI (original type) is then rolled. This is NOT a priority chain — each DD CI type has its own independent trigger check.
- 見張員 detection: type3 == SeaplanePersonnel (39)
- 水雷戦隊熟練見張員: specific item ID 412 (per NGA source)
- ドラム缶: type3 == TransportContainer (30)
- DD CI damage multipliers (pre-cap, applied before night battle cap of 300):
  - GTR: 1.3x (base), up to 2.002x with dual D3 guns on qualifying DDs
  - TRL: 1.2x (base), up to 1.848x with dual D3 guns
  - TTL: 1.5x
  - DTL: 1.3x
- **D-type gun bonuses** (multiplicative on base multiplier, only for qualifying DD classes like 秋月型改二/Mogador型/Tashkent改):
  - D2 (12.7cm連装砲D型改二): ×1.25 on GTR, ×1.25 on TRL
  - D3 (12.7cm連装砲D型改三): ×1.3125 on GTR, ×1.3125 on TRL
  - 2×D2: ×1.40 on GTR, ×1.40 on TRL
- 2nd hit probability (separate from trigger): GTR~65% at LV80+, TRL~50%, TTL~87.5%, DTL~55%
- Add per-type base_attack thresholds: GTR=115, TRL=140, TTL=125, DTL=122

**Patterns to follow:**
- Existing `detect_night_attack_type()` priority chain
- Existing `night_ci_trigger_rate()` formula structure
- `count_equipment_type()` helper for type3 matching

**Test scenarios:**
- Happy path: DD with 主砲+魚雷+電探 → detects as DdMainTorpRadar (sp_list=7 or 11)
- Happy path: DD with 魚雷+見張員+電探 → detects as DdTorpLookoutRadar (sp_list=8 or 12)
- Happy path: DD with 魚雷+水雷戦隊熟練見張員+魚雷 → detects as DdTorpSkilledLookoutTorp (sp_list=9 or 13)
- Happy path: DD with 魚雷+水雷戦隊熟練見張員+ドラム缶 → detects as DdTorpSkilledLookoutDrum (sp_list=10 or 14)
- Edge case: Non-DD ship with same equipment → does NOT trigger DD CI, falls through to standard CI
- Edge case: DD CI multiroll — all 4 DD CI types fail → standard CI is attempted
- Happy path: 2-hit variant triggers when 2nd hit probability succeeds (sp_list 11-14)
- Happy path: DD with D-type gun bonus → multiplier increased (e.g. GTR+D2=1.625x)
- Integration: Full night battle pipeline with DD CI ship → correct sp_list and damage output

**Verification:**
- `cargo test -p emukc_battle` passes with new DD CI tests
- Night battle simulation with DD CI ship produces correct `api_sp_list` values (7-14)

### U2. DD Night CI trigger rate formula

**Goal:** Implement high-fidelity trigger rate calculation for DD CI types.

**Requirements:** R3, R6

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- DD CI uses a **different trigger rate formula** from standard night CI. Standard CI uses Luck-only; DD CI (multiroll) uses Level+Luck:
  - Luck<50: `Base_ship = 0.75 × sqrt(Level) + Luck`
  - Luck≥50: `Base_ship = 0.80 × sqrt(Level) + 50 + sqrt(Luck - 50)`
- Trigger rate = `floor(15 + Base_ship + Modifiers) / Base_attack × 100` (%)
- Per-type base_attack: GTR=115, TRL=140, TTL=125, DTL=122
- Modifiers: flagship +15, chuuha +18, 熟練見張員 +5, 水雷戦隊熟練見張員 (TSLO) on DD/CL(T) +8, 探照灯 +7, 照明弾 +4
- 2-hit variants have same trigger rate as their 1-hit counterparts — hit count is determined by a separate probability check after CI trigger succeeds

**Patterns to follow:**
- `night_ci_trigger_rate()` existing implementation

**Test scenarios:**
- Happy path: DD CI trigger rate increases with luck (same as existing test pattern)
- Edge case: DD with luck=0 → still has base trigger rate from level contribution
- Edge case: Flagship DD → gets +15 bonus
- Edge case: Chuuha DD with torpedo CI → gets +18 bonus (same as TorpTorpTorp)

**Verification:**
- Trigger rate values match documented formulas for each DD CI coefficient

### U3. Pass air_state to shelling phases

**Goal:** Plumb air superiority state from kouku phase into shelling params so day CI can gate on it.

**Requirements:** R1 (prerequisite)

**Dependencies:** None (can be done in parallel with U1-U2)

**Files:**
- Modify: `crates/emukc_battle/src/types/domain.rs` (add field to ShellingParams)
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (pass air_state when constructing ShellingParams)

**Approach:**
- Add `air_state: Option<AirState>` to `ShellingParams`
- Add `pub(crate) fn kouku(&self) -> Option<&BattleKouku>` getter to `BattleState` (field is currently private)
- In `execute_shelling1`/`execute_shelling2`, extract air state from `state.kouku().map(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku)).flatten()`
- Pass into ShellingParams construction
- No behavioral change yet — just plumbing

**Patterns to follow:**
- `NightBattleParams` already carries `air_state: Option<&AirState>`

**Test scenarios:**
- Happy path: After kouku with air supremacy, ShellingParams.air_state == Some(AirState::Supremacy)
- Edge case: No kouku phase (no planes) → air_state == None
- Edge case: Air parity → air_state == Some(AirState::Parity)

**Verification:**
- Existing tests still pass (no behavioral change)
- New test confirms air_state propagation

### U4. Day attack type detection (弾着観測射撃)

**Goal:** Implement `DayAttackType` enum and detection logic for artillery spotting CI.

**Requirements:** R1, R6, R7

**Dependencies:** U3

**Files:**
- Create: `crates/emukc_battle/src/simulation/day_cutin.rs`
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (add module)
- Modify: `crates/emukc_battle/src/simulation/shelling.rs` (call day_cutin detection and integrate CI into attack loop)
- Modify: `crates/emukc_battle/src/targeting.rs` (add seaplane-onslot check helper)

**Approach:**
- Create `DayAttackType` enum: Normal(0), DoubleAttack(2), MainSecCI(3), MainRadarCI(4), MainApSecCI(5), MainApMainCI(6), CarrierCI(7)
- Detection prerequisites: air_state must be Supremacy or Superiority AND ship has seaplane (type3=10 or 11) with onslot > 0 (must reflect post-kouku losses — verify that `BattleRuntimeShip.ship.api_onslot` is mutated during kouku simulation; if not, add onslot update as a prerequisite). Note: Water Fighter (水戦) does NOT qualify — only Water Recon (水偵) and Water Bomber (水爆).
- Priority order (highest first): MainApMainCI(6) > MainApSecCI(5) > MainRadarCI(4) > MainSecCI(3) > DoubleAttack(2)
- Equipment conditions (verified from wikiwiki/zekamashi):
  - 主主徹 MainApMainCI(6): 2× main gun + 1× AP shell (type3=19)
  - 主副徹 MainApSecCI(5): 1× main gun + 1× secondary gun (type3=4/95) + 1× AP shell (type3=19)
  - 主副電 MainRadarCI(4): 1× main gun + 1× secondary gun (type3=4/95) + 1× radar (type3=12/13/93)
  - 主副 MainSecCI(3): 1× main gun + 1× secondary gun (type3=4/95)
  - 連撃 DoubleAttack(2): NOT a separate detection path — fallback when a detected CI type fails its trigger roll and the ship still has 2+ main guns. See U5 for fallback logic.

**Patterns to follow:**
- `detect_night_attack_type()` in night.rs — same detect-then-roll pattern
- `count_equipment_type()` / `count_main_guns()` helpers

**Test scenarios:**
- Happy path: BB with 2 main guns + AP shell + seaplane + air supremacy → detects MainApMainCI (at_type=6)
- Happy path: BB with main gun + secondary + AP shell + seaplane + air superiority → detects MainApSecCI (at_type=5)
- Happy path: BB with main gun + secondary + radar + seaplane + air supremacy → detects MainRadarCI (at_type=4)
- Happy path: BB with main gun + secondary + seaplane + air superiority → detects MainSecCI (at_type=3)
- Edge case: BB with 2 main guns but NO AP shell → does NOT detect MainApMainCI(6)
- Edge case: Air parity → no CI detection (returns Normal)
- Edge case: No seaplane equipped → no CI detection
- Edge case: Seaplane equipped but onslot=0 (shot down) → no CI detection
- Edge case: Water Fighter (水戦) equipped instead of Water Recon/Bomber → no CI detection
- Edge case: Ship has both MainApMainCI and MainApSecCI conditions → MainApMainCI wins (higher priority)

**Verification:**
- All detection tests pass
- Priority ordering is correct

### U5. Day CI trigger rate and shelling integration

**Goal:** Implement trigger rate formula and integrate day CI into the shelling loop.

**Requirements:** R1, R6, R7

**Dependencies:** U3, U4

**Files:**
- Modify: `crates/emukc_battle/src/simulation/day_cutin.rs` (add trigger rate)
- Modify: `crates/emukc_battle/src/simulation/shelling.rs` (integrate CI into attack loop)
- Modify: `crates/emukc_battle/src/damage.rs` (add ci_multiplier parameter to calculate_shelling_damage)

**Approach:**
- **Day CI damage multipliers are post-cap** (applied after daytime shelling soft cap of 180): MainApMain=1.5x, MainApSec=1.3x, MainRadar=1.2x, MainSec=1.1x, DoubleAttack=1.2x×2hits
- Accuracy bonuses: MainApMain=1.20x, MainApSec=1.30x, MainRadar=1.50x, MainSec=1.30x, DoubleAttack=1.10x
- Trigger rate formula (complex, LoS-based):
  - Under AS: `Base_ship = floor(floor(sqrt(Luck)) + 0.6 × (1.2 × sum(Ship_LoS_Equip) + floor(sqrt(LoS_Fleet) + LoS_Fleet/10)))`
  - Under AS+: `Base_ship = floor(floor(sqrt(Luck)) + 0.7 × (1.6 × sum(Ship_LoS_Equip) + floor(sqrt(LoS_Fleet) + LoS_Fleet/10)) + 10)`
  - `Trigger% = (10 + Base_ship + Mod_Flag) / Base_attack × 100` where Mod_Flag=15 if flagship
  - Per-type Base_attack: MainApMain=150, MainApSec=140, MainRadar=130, MainSec=120, DoubleAttack=130
- On CI trigger failure: fall back to next eligible type in priority order. If all CI types fail and ship has 2+ main guns, attempt DoubleAttack roll. If DoubleAttack also fails, Normal attack.
- In shelling.rs: call `resolve_day_attack()` before damage calc, pass multiplier to damage function
- Extend `calculate_shelling_damage` signature with `ci_multiplier: Option<f64>` applied **after** the soft cap check (post-cap)
- Update `api_at_type` output, `api_df_list` (multi-target for DoubleAttack), `api_si_list` display IDs

**Patterns to follow:**
- `resolve_night_attack()` pattern: detect → roll → fallback
- `calculate_night_damage()` already takes optional multiplier

**Test scenarios:**
- Happy path: BB with MainApMainCI setup, air supremacy → at_type=6, damage includes 1.5x post-cap multiplier
- Happy path: DoubleAttack → at_type=2, 2 hits, each at 1.2x post-cap
- Happy path: CI trigger fails → cascading fallback to next eligible type, then DoubleAttack, then Normal
- Edge case: Trigger rate increases with luck (LoS-based formula)
- Edge case: Flagship gets +15 to trigger rate
- Edge case: AS+ gives higher trigger rate than AS (different formula coefficients)
- Integration: Full day battle with CI-eligible ship → correct api_at_type in hougeki output

**Verification:**
- `cargo test -p emukc_battle` passes
- Day battle simulation produces correct at_type values (2-6)
- Damage values reflect CI multipliers

### U6. Day carrier CI (api_at_type 7)

**Goal:** Implement carrier Cut-In for day battle (戦爆連合CI).

**Requirements:** R2, R6, R7

**Dependencies:** U3, U5

**Files:**
- Modify: `crates/emukc_battle/src/simulation/day_cutin.rs`
- Modify: `crates/emukc_battle/src/simulation/shelling.rs`

**Approach:**
- Add `CarrierCI` variant to `DayAttackType` (at_type=7)
- Detection: ship is CV/CVL/CVB/Graf Zeppelin + has dive bomber (type3=7) + has torpedo bomber (type3=8) + air supremacy/superiority. Note: 噴式艦載機 (jet) does NOT count as dive bomber. 爆戦 (fighter-bomber like 零戦62型爆戦) DOES count as dive bomber.
- Sub-types by priority (FBA > BBA > BA):
  - FBA: 1+ fighter (type3=6) + 1+ dive bomber (type3=7) + 1+ torpedo bomber (type3=8) → 1.25x post-cap
  - BBA: 2+ dive bomber (type3=7) + 1+ torpedo bomber (type3=8) → 1.2x post-cap
  - BA: 1+ dive bomber (type3=7) + 1+ torpedo bomber (type3=8) → 1.15x post-cap
- Carrier CI is mutually exclusive with artillery spotting (carriers don't use seaplanes for spotting)
- Multipliers are post-cap (after 180 cap), same timing as artillery spotting
- Trigger rate uses similar LoS-based formula as artillery spotting but with different base_attack values. Rate increases with luck; luck 50 is a threshold point.
- Ship must not be in attack-disabled state (e.g. chuuha non-armored CV)

**Patterns to follow:**
- `is_cv_type()` helper in damage.rs for carrier detection
- Day CI detection chain in day_cutin.rs

**Test scenarios:**
- Happy path: CV with fighter+bomber+attacker + air supremacy → at_type=7 (FBA)
- Happy path: CV with 2x bomber + attacker + air superiority → at_type=7 (BBA)
- Edge case: CV with only bombers (no attacker) → no carrier CI
- Edge case: Air parity → no carrier CI
- Edge case: CVL with correct equipment → carrier CI triggers (not limited to CV)

**Verification:**
- Carrier CI produces at_type=7 in day battle output
- Correct sub-type multiplier applied to damage

### U7. Night carrier CI (api_sp_list 6)

**Goal:** Implement carrier night CI with night aviation personnel requirement.

**Requirements:** R4, R6, R7

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`

**Approach:**
- Add `CarrierNightCI` variant to `NightAttackType` (sp_list=6)
- **夜間作戦航空要員 requirement (conditional):**
  - Normal CV/CVL: requires item 258 (夜間作戦航空要員) or 259 (夜間作戦航空要員+熟練甲板員)
  - **Exempt ships** (only need night planes, no 航空要員): Saratoga Mk.II, 赤城改二戊, 加賀改二戊
  - Ark Royal: can attack at night with Swordfish equipped (different calculation)
- **Night plane detection:** 夜間戦闘機 and 夜間攻撃機 are detected by equipment icon type, not specific item IDs. Need to identify the icon type values for "夜戦" and "夜攻" categories.
- **5 carrier night CI sub-types** (checked in priority order):

  | Priority | 夜戦 | 夜攻 | 光電管彗星 | Other | Multiplier |
  |----------|------|------|------------|-------|------------|
  | 1 | 2 | 1 | | | 1.25x |
  | 2 | 1 | 1 | | | 1.20x |
  | 3 | 1 | | 1 | | 1.20x |
  | 4 | | 1 | 1 | | 1.20x |
  | 5 | 1 | | | 2 | 1.18x |

- Carrier night CI is a single-hit attack (unlike torpedo CI which is 2-hit)
- Priority: carrier night CI is checked before standard CI for carrier ships
- Night aviation attack power formula uses slot capacity bonuses: `A × 搭載数 + B × (火力+雷装+爆装+対潜) × √(搭載数) + √(★改修度)` where A/B differ by plane type
- Night battle cap is 300 (pre-cap for CI multiplier)
- Damage restriction: normal CV chuuha+ cannot night attack; armored CV (CVB) chuuha can, taiha cannot
- Carriers that don't meet night CI conditions but have 航空要員 can still perform normal night aviation attack (sp_list=6 but non-CI)

**Patterns to follow:**
- `detect_night_attack_type()` existing structure
- Item-ID based detection (similar to how 水雷戦隊熟練見張員 needs specific ID)

**Test scenarios:**
- Happy path: CV with 夜間作戦航空要員 + 2×夜戦 + 1×夜攻 → sp_list=6, 1.25x multiplier (priority 1)
- Happy path: CV with 航空要員 + 1×夜戦 + 1×夜攻 → sp_list=6, 1.20x (priority 2)
- Edge case: CV without 航空要員 → cannot attack at night (normal behavior)
- Edge case: Saratoga Mk.II without 航空要員 but with night planes → carrier night CI triggers (exempt)
- Edge case: 加賀改二戊 without 航空要員 but with night planes → carrier night CI triggers (exempt)
- Edge case: CV with 航空要員 but no night planes → normal attack, not CI
- Edge case: CVL with night equipment → carrier night CI triggers
- Edge case: Normal CV chuuha → cannot night attack; CVB chuuha → can night attack

**Verification:**
- Night battle with carrier night CI ship produces sp_list=6
- Correct multiplier applied

### U8. Flagship special attack detection framework

**Goal:** Create the special attack module with condition checking and multi-ship attack execution.

**Requirements:** R5, R7

**Dependencies:** None (no functional dependency on U3/U5 — special attacks have independent trigger mechanics and do not gate on air_state. Implement after U5 to avoid shelling.rs merge conflicts.)

**Files:**
- Create: `crates/emukc_battle/src/simulation/special_attack.rs`
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (add module, call before normal shelling)
- Modify: `crates/emukc_battle/src/simulation/shelling.rs` (skip ship if special attack already fired)

**Approach:**
- Define `SpecialAttackType` enum: NelsonTouch(100), NagatoClassBroadside(101), NagatoMutsuBroadside(102), ColoradoBroadside(103), KongouNightAssault(104), RichelieuAttack(105), QueenElizabethAttack(106)
- Each type has: eligible flagship ship IDs, fleet composition requirements, formation requirements, trigger conditions
- **Universal conditions for all special attacks:**
  - Flagship is always fleet position 1 (index 0) in the friendly fleet array
  - **Damage restriction:** flagship must be shouha or less (HP > 75%); companion ships must be chuha or less (HP > 50%)
  - **Modifier timing:** day = post-cap (after 180), night = pre-cap (before 300)
  - Special attacks fire during the flagship's normal shelling turn (replaces it), not prepended before the shelling loop
  - Multi-ship attacks: multiple attackers hit multiple defenders in sequence
- **Formation requirements per type:**
  - Nelson Touch, Richelieu: 複縦陣 OR 連合艦隊第二警戒航行序列
  - Nagato, Colorado, QE: 梯形陣 OR 連合艦隊第二警戒航行序列
- **Equipment bonuses** (multiplicative per participating ship, applied to damage):
  - AP shell (type3=19): ×1.35
  - Surface radar (type3=12/13, LoS≥5): ×1.15
  - SG Radar (late model): additional ×1.15 + 5% trigger rate per ship (Colorado only)
  - **Exception:** Nelson Touch has NO equipment bonuses
- Output: at_type/sp_list = 100-106, multiple entries in hougeki arrays for each participating ship
- Framework: `check_special_attack_eligibility(fleet, formation, codex) -> Option<SpecialAttackType>` + `execute_special_attack(type, fleet, enemies, rng) -> Vec<HougekiEntry>`

**Patterns to follow:**
- Night battle's per-ship attack loop structure
- `BattleHougeki` output format

**Test scenarios:**
- Happy path: Nelson/Rodney as flagship + valid 1/3/5 composition + 複縦陣 → NelsonTouch eligible
- Happy path: 長門改二 flagship + BB at position 2 + 梯形陣 → NagatoClassBroadside eligible
- Edge case: Nelson not at flagship position → not eligible
- Edge case: Valid flagship but wrong formation → not eligible
- Edge case: Valid flagship but composition requirement not met → not eligible
- Edge case: Flagship chuuha (>25% damage) → not eligible
- Edge case: Companion ship chuuha → excluded from attack but flagship can still fire
- Edge case: Special attack fires during flagship's turn → other participating ships still get their own normal turns later
- Edge case: Nelson Touch with NO equipment bonuses (unlike other types)
- Edge case: 長門一斉射 with AP shell → ×1.35 equipment bonus applied

**Verification:**
- Special attack detection correctly identifies eligible compositions
- Output format matches expected api_at_type values

### U9. Implement 3-ship special attacks (Nelson, Nagato, Colorado)

**Goal:** Implement Nelson Touch (100), 長門一斉射 (101), 長門陸奥 (102), Colorado (103) — attacks involving 3 ships.

**Requirements:** R5, R6, R7

**Dependencies:** U8

**Files:**
- Modify: `crates/emukc_battle/src/simulation/special_attack.rs`

**Approach:**
- **Nelson Touch (100):**
  - Flagship: Nelson OR Rodney (any remodel) — both can trigger
  - Participants: positions 1, 3, 5. Companions cannot be carriers or submarines.
  - Multipliers: 2.0x per ship (normal), 2.5x per ship (T-disadvantage)
  - Nelson + Rodney both present bonus: flagship ×1.15, companion Rodney ×1.20 (multiplicative on base)
  - **NO equipment bonuses** for Nelson Touch
  - Trigger rate: `1.1 × sqrt(flagLv) + sqrt(3rdLv) + sqrt(5thLv) + 1.4 × sqrt(flagLuck) + 25`

- **長門一斉射 (101):**
  - Flagship: 長門改二 only
  - 2nd ship: any battleship type
  - 3 attacks: flagship ×2 (1.4x each), companion ×1 (1.1x)
  - Equipment bonuses: AP shell ×1.35, surface radar ×1.15 (per ship)
  - Trigger rate: `30 + sqrt(flagLv) + sqrt(2ndLv) + 1.2 × (sqrt(flagLuck) + sqrt(2ndLuck))`

- **長門陸奥 (102):**
  - Flagship: 長門改二
  - 2nd ship: **must be 陸奥改二**
  - 3 attacks: flagship ×2 (1.68x each), Mutsu ×1 (1.68x)
  - Equipment bonuses: AP shell ×1.35, surface radar ×1.15 (per ship)

- **Colorado (103):**
  - Flagship: Colorado OR Maryland (any remodel) — both can trigger
  - Participants: positions 1, 2, 3. All must be battleship type (BB/FBB/BBV).
  - Multipliers: flagship 1.5x, companions 1.3x each
  - **Big Seven companion bonus** (multiplicative on companion base): 2nd companion is Big7 → ×1.15, 3rd companion is Big7 → ×1.17. Big7 = Nagato, Mutsu, Nelson, Rodney, Colorado, Maryland
  - Equipment bonuses: AP shell ×1.35, surface radar ×1.15, SG Radar (late) additional ×1.15 + 5% trigger per ship
  - Ammo consumption: all participating ships consume ammo (same as other special attacks)

- Ship ID detection: lookup specific mst_id values from Codex

**Patterns to follow:**
- Multi-hit damage calculation from night CI (iterate hits, apply damage per hit)
- Ship type/ID detection from Codex

**Test scenarios:**
- Happy path: Nelson Touch fires with valid 1/3/5 composition → 3 attack entries, 2.0x each
- Happy path: Rodney as flagship → Nelson Touch also fires (Rodney eligible)
- Happy path: Nelson + Rodney both present → bonus multipliers applied (1.15x/1.20x)
- Happy path: 長門一斉射 fires → flagship 2 hits (1.4x) + 2nd ship 1 hit (1.1x)
- Happy path: 長門一斉射 + AP shell → ×1.35 equipment bonus on damage
- Happy path: 長門陸奥 fires with higher multipliers (1.68x all)
- Happy path: Colorado fires with 3 ships → flagship 1.5x, companions 1.3x
- Happy path: Maryland as flagship → Colorado special also fires
- Happy path: Colorado with Big7 companion → bonus multiplier applied
- Edge case: Flagship chuuha (HP ≤ 75%) → special attack does not fire
- Edge case: Required companion ship sunk → excluded from attack
- Edge case: Companion at position 3 is carrier → excluded (Nelson Touch)

**Verification:**
- All 4 attack types produce correct at_type values (100-103)
- Multi-ship damage output correctly formatted

### U10. Implement 3-ship special attacks (Richelieu, QE)

**Goal:** Implement Richelieu (105) and Queen Elizabeth (106) — attacks involving 3 ships.

**Requirements:** R5, R6, R7

**Dependencies:** U8

**Files:**
- Modify: `crates/emukc_battle/src/simulation/special_attack.rs`

**Approach:**
- **Richelieu (105):**
  - Flagship: Richelieu Deux or Richelieu Kai
  - Participants: positions 1, 2, 3 (3 attacks total)
  - 2nd ship: **must be Jean Bart Kai** (required)
  - 3rd ship: any battleship type
  - Multipliers: flagship 1.3x, Jean Bart 1.3x, 3rd ship 1.24x
  - Equipment bonuses: AP shell ×1.35, surface radar ×1.15 (per ship)
  - 38cm quad gun Kai Deux: trigger rate bonus when equipped on participating ships
  - Speed: high-speed fleet compatible
  - Formation: 複縦陣

- **Queen Elizabeth (106):**
  - Flagship: Warspite Kai or Valiant Kai
  - Participants: positions 1, 2, 3 (3 attacks total)
  - 2nd ship: the other sister (Valiant Kai if flagship is Warspite, or Warspite Kai if flagship is Valiant)
  - 3rd ship: any battleship type
  - Multipliers: all shots 1.24x
  - Equipment bonuses: AP shell ×1.35, surface radar ×1.15 (per ship)
  - Speed: low speed (slow fleet)
  - Minimum fleet: 6 surface ships required
  - Formation: 梯形陣

- Ship ID detection: lookup mst_ids from Codex

**Patterns to follow:**
- Same framework as U9 but simpler coordination

**Test scenarios:**
- Happy path: Richelieu改 flagship + Jean Bart Kai at position 2 + BB at position 3 → at_type=105, 3 attacks (1.3x/1.3x/1.24x)
- Happy path: Warspite Kai flagship + Valiant Kai at position 2 + BB at position 3 → at_type=106, 3 attacks (1.24x each)
- Edge case: Valiant Kai flagship + Warspite Kai at position 2 → also valid (either sister can be flagship)
- Edge case: Richelieu without Jean Bart Kai at position 2 → not eligible (Jean Bart Kai required)
- Edge case: 2nd ship is not a valid companion → not eligible
- Edge case: Richelieu + AP shell → ×1.35 equipment bonus
- Edge case: QE fleet is high-speed → not eligible (QE requires slow fleet)

**Verification:**
- Both attack types produce correct at_type/sp_list values
- `cargo test -p emukc_battle` passes

---

## System-Wide Impact

- **Interaction graph:** Shelling phase now depends on kouku phase output (air_state). Special attacks interact with normal shelling order (ships that fired special attack skip normal turn).
- **Error propagation:** If air_state is None (no kouku), day CI simply doesn't trigger — graceful degradation.
- **State lifecycle risks:** None — all CI state is computed within the battle simulation, no persistence.
- **API surface parity:** Day and night battle API responses both affected. Client expects correct at_type/sp_list values for animation playback.
- **Unchanged invariants:** Torpedo phases, ASW, kouku damage calculation — all unchanged. Night battle basic CI (sp_list 0-5) behavior unchanged.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| CI coefficient values may be inaccurate | Verified from wikiwiki/zekamashi/en.kancollewiki; can be tuned post-implementation |
| Flagship special attack ship IDs may change with game updates | Store ship ID lists in a discoverable constant, not buried in logic |
| 水雷戦隊熟練見張員 item ID 412 needs Codex verification | Document which IDs are used; consider adding a Codex helper for "is skilled lookout" |
| Multi-ship special attacks have complex output format | Test against captured real API responses if available |
| Day CI trigger rate formula is complex (LoS-based) | Implement incrementally: start with simplified formula, refine with full LoS calculation |
| DD CI multiroll mechanism differs from standard CI priority chain | Test each DD CI type independently; verify multiroll→standard fallback chain |
| Night fighter/night attacker icon type detection needs verification | Check Codex for icon type values; may need specific item ID lists as fallback |
| DD CI D-type gun bonus requires equipment ID matching | Start without D-type bonuses (base multipliers only); add in follow-up commit |
| Carrier night CI has special carriers exempt from 航空要員 | Ship ID list for exempt carriers must be maintained from Codex |
| Pre-existing sp_list mapping bug affects all night CI | Fix in U0 before adding new CI types |

---

## Sources & References

- **Origin document:** [docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md](docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md)
- Related code: `crates/emukc_battle/src/simulation/night.rs` (pattern template)
- API reference: `docs/apilist.txt` lines 2252-2270 (day at_type), lines 2319-2342 (night sp_list)
- External: wikiwiki 弾着観測射撃 page (trigger rate formulas)
