---
title: "feat: Implement Cut-In attack system (day CI, DD night CI, carrier CI, special attacks)"
type: feat
status: active
date: 2026-05-14
origin: docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md
---

# feat: Implement Cut-In attack system

## Summary

Implement the complete Cut-In attack system across day and night battle phases. Work is sequenced in 4 phases by difficulty: (1) extend night CI with DD-specific types, (2) add day artillery spotting (弾着観測射撃), (3) add carrier CI for day and night, (4) add flagship special attacks. Each phase follows the existing `night.rs` pattern of detect → trigger-roll → resolve.

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

- **Mirror night.rs pattern for day CI**: Create `DayAttackType` enum + `detect_day_attack_type()` + `day_ci_trigger_rate()` + `resolve_day_attack()` in shelling.rs. Same architectural shape, different mechanics.
- **Pass air_state via ShellingParams**: Add `air_state: Option<AirState>` to `ShellingParams`. Populated from `state.kouku.api_stage1.api_disp_seiku` before shelling phases execute.
- **DD CI detection uses KcSlotItemType3**: 見張員 = `SeaplanePersonnel` (type3=39), ドラム缶 = `TransportContainer` (type3=30). No need for item-ID matching for basic detection. 水雷戦隊熟練見張員 specifically needs item-ID check (it's a subset of type3=39).
- **Special attacks in separate module**: `crates/emukc_battle/src/simulation/special_attack.rs` — complex enough to warrant isolation. Called before normal shelling when conditions met.
- **Damage multiplier as parameter**: Extend `calculate_shelling_damage()` with optional `ci_multiplier: Option<f64>` applied post-cap (same pattern as `calculate_night_damage`).

---

## Open Questions

### Resolved During Planning

- **Q1. Air state available to shelling?** Yes — `state.kouku.api_stage1.api_disp_seiku` stores it. Need to extract and pass via `ShellingParams`.
- **Q2. 見張員 type3?** `SeaplanePersonnel = 39`. 水雷戦隊熟練見張員 is a specific item within that type3 — needs item-ID check.

### Deferred to Implementation

- Exact CI coefficient values for each DD CI type (need wikiwiki verification)
- Precise flagship special attack ship ID lists (need Codex data verification)
- Whether 水雷戦隊熟練見張員 detection should use item ID or a new type3 sub-category

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

### U1. Extend NightAttackType with DD CI variants

**Goal:** Add DD-specific night CI types (sp_list 7-14) to the existing night battle system.

**Requirements:** R3, R6, R7

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/night.rs`
- Modify: `crates/emukc_battle/src/targeting.rs` (if DD-type gate needed)

**Approach:**
- Extend `NightAttackType` enum with: `DdMainTorpRadar`(7), `DdTorpLookoutRadar`(8), `DdTorpSkilledLookoutTorp`(9), `DdTorpSkilledLookoutDrum`(10), and their 2-hit variants (11-14)
- Add detection logic in `detect_night_attack_type`: gate on ship type == DD, then check equipment combinations
- **Priority rule:** For DD ships, run standard CI detection first (MainMainMain, MainMainSec, TorpTorpTorp, MainTorpRadar). DD-specific CI is checked only when no standard CI qualifies.
- 見張員 detection: type3 == SeaplanePersonnel (39)
- 水雷戦隊熟練見張員: specific item IDs (need runtime lookup from Codex)
- ドラム缶: type3 == TransportContainer (30)
- Add CI coefficients and multipliers for each variant

**Patterns to follow:**
- Existing `detect_night_attack_type()` priority chain
- Existing `night_ci_trigger_rate()` formula structure
- `count_equipment_type()` helper for type3 matching

**Test scenarios:**
- Happy path: DD with 主砲+魚雷+電探 → detects as DdMainTorpRadar (sp_list=7)
- Happy path: DD with 魚雷+見張員+電探 → detects as DdTorpLookoutRadar (sp_list=8)
- Happy path: DD with 魚雷+水雷戦隊熟練見張員+魚雷 → detects as DdTorpSkilledLookoutTorp (sp_list=9)
- Happy path: DD with 魚雷+水雷戦隊熟練見張員+ドラム缶 → detects as DdTorpSkilledLookoutDrum (sp_list=10)
- Edge case: Non-DD ship with same equipment → does NOT trigger DD CI, falls through to standard CI
- Edge case: DD with standard CI equipment (3x main gun) → standard CI takes priority over DD CI
- Happy path: 2-hit variant triggers when conditions met (sp_list 11-14)
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
- DD CI shares the base luck formula with standard night CI but uses different coefficients
- Each DD CI type has its own coefficient value affecting trigger probability
- Chuuha bonus and flagship bonus apply same as standard CI
- 2-hit variants have same trigger rate as their 1-hit counterparts (the hit count is determined after trigger succeeds)

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
- In `execute_shelling1`/`execute_shelling2`, extract air state from `state.kouku.as_ref().map(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku)).flatten()`
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

**Approach:**
- Create `DayAttackType` enum: Normal(0), DoubleAttack(2), MainSecCI(3), MainRadarCI(4), MainApCI(5), MainMainCI(6), CarrierCI(7)
- Detection prerequisites: air_state must be Supremacy or Superiority AND ship has seaplane (type3=10 or 11) with onslot > 0 (must reflect post-kouku losses — verify that `BattleRuntimeShip.ship.api_onslot` is mutated during kouku simulation; if not, add onslot update as a prerequisite)
- Priority order (highest first): MainMain(6) > MainAP(5) > MainRadar(4) > MainSec(3) > DoubleAttack(2)
- Equipment conditions:
  - MainMain: 2+ main guns
  - MainAP: 1+ main gun + 1+ ArmorPiercingShell (type3=19)
  - MainRadar: 1+ main gun + 1+ radar (type3=12/13/93)
  - MainSec: 1+ main gun + 1+ secondary gun (type3=4/95)
  - DoubleAttack: 2+ main guns (same condition as MainMain but lower priority — triggered when MainMain CI roll fails)

**Patterns to follow:**
- `detect_night_attack_type()` in night.rs — same detect-then-roll pattern
- `count_equipment_type()` / `count_main_guns()` helpers

**Test scenarios:**
- Happy path: BB with 2 main guns + seaplane + air supremacy → detects MainMainCI
- Happy path: BB with main gun + AP shell + seaplane + air superiority → detects MainApCI
- Happy path: BB with main gun + radar + seaplane + air supremacy → detects MainRadarCI
- Happy path: BB with main gun + secondary + seaplane + air superiority → detects MainSecCI
- Edge case: Air parity → no CI detection (returns Normal)
- Edge case: No seaplane equipped → no CI detection
- Edge case: Seaplane equipped but onslot=0 (shot down) → no CI detection
- Edge case: Ship has both MainMain and MainAP conditions → MainMain wins (higher priority)

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
- Trigger rate formula: base = floor(sqrt(luck) + 10), air bonus (supremacy +10, superiority +0), flagship +15, equipment improvement bonuses
- On CI trigger failure: fall back to DoubleAttack if eligible, else Normal
- Damage multipliers: MainMain=1.5x, MainAP=1.3x, MainRadar=1.2x, MainSec=1.1x, DoubleAttack=1.2x×2hits
- In shelling.rs: call `resolve_day_attack()` before damage calc, pass multiplier to damage function
- Extend `calculate_shelling_damage` signature with `ci_multiplier: Option<f64>` applied post-cap
- Update `api_at_type` output, `api_df_list` (multi-target for DoubleAttack), `api_si_list` display IDs

**Patterns to follow:**
- `resolve_night_attack()` pattern: detect → roll → fallback
- `calculate_night_damage()` already takes optional multiplier

**Test scenarios:**
- Happy path: BB with MainMainCI setup, air supremacy → at_type=6, damage includes 1.5x multiplier
- Happy path: DoubleAttack → at_type=2, 2 hits, each at 1.2x
- Happy path: CI trigger fails → falls back to DoubleAttack or Normal
- Edge case: Trigger rate increases with luck
- Edge case: Flagship gets +15 to trigger rate
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
- Detection: ship is CV/CVL/CVB + has bomber (type3=7) + has torpedo bomber (type3=8) + air supremacy/superiority
- Sub-types: FBA (fighter+bomber+attacker), BBA (bomber+bomber+attacker), BA (bomber+attacker) — affect multiplier
- FBA=1.25x, BBA=1.2x, BA=1.15x (approximate — verify from wikiwiki)
- Carrier CI is mutually exclusive with artillery spotting (carriers don't use seaplanes for spotting)
- Trigger rate uses different formula than surface ship CI

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
- Detection: ship is CV/CVL/CVB + has 夜間作戦航空要員 (AviationPersonnel type3=35, specific item IDs for night variant) + has night fighter or night attacker
- Night fighter/attacker detection: specific item IDs (夜間戦闘機, 夜攻) — these don't have unique type3 values
- Multiplier depends on equipment combination
- Priority: carrier night CI is checked before standard CI for carrier ships
- Carriers that can't night CI still cannot attack at night (existing `can_attack_night_ship` gate)

**Patterns to follow:**
- `detect_night_attack_type()` existing structure
- Item-ID based detection (similar to how 水雷戦隊熟練見張員 needs specific ID)

**Test scenarios:**
- Happy path: CV with night aviation personnel + night fighter → sp_list=6
- Edge case: CV without night personnel → cannot attack at night (existing behavior)
- Edge case: CV with night personnel but no night planes → normal attack, not CI
- Edge case: CVL with night equipment → carrier night CI triggers

**Verification:**
- Night battle with carrier night CI ship produces sp_list=6
- Correct multiplier applied

### U8. Flagship special attack detection framework

**Goal:** Create the special attack module with condition checking and multi-ship attack execution.

**Requirements:** R5, R7

**Dependencies:** U3, U5

**Files:**
- Create: `crates/emukc_battle/src/simulation/special_attack.rs`
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (add module, call before normal shelling)
- Modify: `crates/emukc_battle/src/simulation/shelling.rs` (skip ship if special attack already fired)

**Approach:**
- Define `SpecialAttackType` enum: NelsonTouch(100), NagatoClassBroadside(101), NagatoMutsuBroadside(102), ColoradoBroadside(103), RichelieuAttack(105), QueenElizabethAttack(106)
- Each type has: eligible flagship ship IDs, fleet composition requirements, trigger conditions
- Special attacks fire during the flagship's normal shelling turn (replaces it), not prepended before the shelling loop. When the flagship's turn comes in the shelling order, check eligibility and execute special attack instead of normal attack.
- Multi-ship attacks: multiple attackers hit multiple defenders in sequence
- Output: at_type/sp_list = 100-106, multiple entries in hougeki arrays for each participating ship
- Framework: `check_special_attack_eligibility(fleet, codex) -> Option<SpecialAttackType>` + `execute_special_attack(type, fleet, enemies, rng) -> Vec<HougekiEntry>`

**Patterns to follow:**
- Night battle's per-ship attack loop structure
- `BattleHougeki` output format

**Test scenarios:**
- Happy path: Nelson as flagship + valid 1/3/5 composition → NelsonTouch eligible
- Happy path: 長門改二 flagship + BB at position 2 → NagatoClassBroadside eligible
- Edge case: Nelson not at flagship position → not eligible
- Edge case: Valid flagship but composition requirement not met → not eligible
- Edge case: Special attack fires during flagship's turn → other participating ships still get their own normal turns later (only flagship's turn is replaced)

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
- Nelson Touch (100): flagship + 3rd + 5th ship attack, each hits different target, 2.0x/2.0x/2.0x multiplier
- 長門一斉射 (101): flagship + 2nd ship, flagship 2 hits, 2nd ship 1 hit, multipliers vary
- 長門陸奥 (102): same as 101 but requires 陸奥改二 at position 2, higher multipliers
- Colorado (103): flagship + 2nd + 3rd ship, each fires once
- Ship ID detection: lookup specific mst_id values from Codex (Nelson, 長門改二, 陸奥改二, Colorado改)
- Trigger rate: each type has its own formula (generally luck-based + level-based)

**Patterns to follow:**
- Multi-hit damage calculation from night CI (iterate hits, apply damage per hit)
- Ship type/ID detection from Codex

**Test scenarios:**
- Happy path: Nelson Touch fires with valid 1/3/5 composition → 3 attack entries
- Happy path: 長門一斉射 fires → flagship 2 hits + 2nd ship 1 hit
- Happy path: 長門陸奥 fires with higher multipliers than generic 101
- Happy path: Colorado fires with 3 ships
- Edge case: Flagship taiha → special attack does not fire
- Edge case: Required companion ship sunk → special attack does not fire

**Verification:**
- All 4 attack types produce correct at_type values (100-103)
- Multi-ship damage output correctly formatted

### U10. Implement 2-ship special attacks (Richelieu, QE)

**Goal:** Implement Richelieu (105) and Queen Elizabeth (106) — attacks involving 2 ships.

**Requirements:** R5, R6, R7

**Dependencies:** U8

**Files:**
- Modify: `crates/emukc_battle/src/simulation/special_attack.rs`

**Approach:**
- Richelieu (105): flagship + 2nd ship coordinated attack
- QE (106): Warspite/Valiant flagship + sister ship at position 2
- Ship ID detection: Richelieu改, Warspite, Valiant mst_ids from Codex
- Simpler than 3-ship attacks — 2 participants, each fires once

**Patterns to follow:**
- Same framework as U9 but simpler coordination

**Test scenarios:**
- Happy path: Richelieu改 flagship + valid 2nd ship → at_type=105
- Happy path: Warspite flagship + Valiant at position 2 → at_type=106
- Edge case: Valiant flagship + Warspite at position 2 → also valid (either sister can be flagship)
- Edge case: 2nd ship is not a valid companion → not eligible

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
| CI coefficient values may be inaccurate | Use wikiwiki/kcwiki documented values; can be tuned post-implementation |
| Flagship special attack ship IDs may change with game updates | Store ship ID lists in a discoverable constant, not buried in logic |
| 水雷戦隊熟練見張員 item ID detection is fragile | Document which IDs are used; consider adding a Codex helper for "is skilled lookout" |
| Multi-ship special attacks have complex output format | Test against captured real API responses if available |

---

## Sources & References

- **Origin document:** [docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md](docs/brainstorms/2026-05-14-cut-in-attack-system-requirements.md)
- Related code: `crates/emukc_battle/src/simulation/night.rs` (pattern template)
- API reference: `docs/apilist.txt` lines 2252-2270 (day at_type), lines 2319-2342 (night sp_list)
- External: wikiwiki 弾着観測射撃 page (trigger rate formulas)
