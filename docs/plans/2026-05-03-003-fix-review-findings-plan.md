---
title: Fix Code Review Findings — Battle Crate Audit Round 2
status: completed
created: 2026-05-03
plan_depth: standard
review_run_id: "20260503-205753-a8cc0bab"
---

# Fix Code Review Findings — Battle Crate Audit Round 2

## Problem Frame

A 9-reviewer code review of the `fix/battle-crate-audit-findings` branch (commits 7bbf77c..e1bf9f9, 48 files) found 24 findings across P0-P3 severity. The most critical is a P0 kouku flag array bug where `api_fbak_flag`/`api_frai_flag` are sized for the wrong fleet and written with wrong indices, confirmed by client-decoded JavaScript (`main-decoder/out/modules/module-85977-air-war-stage3-model.js`). Several P1 findings affect gameplay correctness (married HP bonus, enemy damage_dealt tracking). The user also requests deeper kouku investigation into potential torpedo trail and air-capable ship filtering issues.

## Scope

**In scope:**
- Fix all P0 and P1 findings
- Fix actionable P2 findings (safe_auto and manual with clear fixes)
- Add test coverage for all fixed items
- Investigate kouku deeper issues (torpedo trail display, ship eligibility for airstrike phase)
- Advisory P2/P3 items are documented but not implemented

**Out of scope:**
- New features (airbattle/ld_airbattle battle types — noted as future work in kouku.rs comments)
- Night CI pre-cap vs post-cap multiplier verification (advisory, requires KanColle wiki verification)
- Fleet speed re-computation after kouku sinking (advisory, architectural change)
- Formation modifier table divergence monitoring (advisory)

## Implementation Units

### U1: Fix kouku flag array sizing and assignment [P0]

**Files:** `crates/emukc_battle/src/simulation/kouku.rs` (lines 379-414)

**Problem:** Two bugs confirmed by client code:

1. **Wrong sizes:** `api_fbak_flag` initialized with `enemy.len()`, `api_ebak_flag` with `friendly.len()`. Client reads `api_fbak_flag` by friendly-ship index and `api_ebak_flag` by enemy-ship index. Any fleet size mismatch (e.g., 3 friendly vs 6 enemy) causes client-side deserialization error or visual glitch.

2. **Wrong assignment:** In `execute_airstrike_phase`, `bak_flags[target_idx] = 1` marks a defender as bombed. The first call (friendly attacks enemy) passes `api_fbak_flag` as `bak_flags` — but these are enemy defenders, so the flag should go to `api_ebak_flag`. Similarly the second call swaps them.

**Client evidence** (`module-85977-air-war-stage3-model.js`):
```javascript
AirWarStage3Model.prototype.getBak = function(_0x165407) {
    var _0x25cacc = this._friend ? "api_fbak_flag" : "api_ebak_flag";
    return numArray[_0x165407];
};
```

**Fix:**
```rust
// Line 379-382: swap sizes
let mut api_erai_flag = vec![0i64; enemy.len()];      // was friendly.len()
let mut api_ebak_flag = vec![0i64; enemy.len()];      // was friendly.len()
let mut api_frai_flag = vec![0i64; friendly.len()];    // was enemy.len()
let mut api_fbak_flag = vec![0i64; friendly.len()];    // was enemy.len()

// Lines 387-414: swap flag assignments between the two calls
// First call (friendly attacking enemy) — defenders are enemy
execute_airstrike_phase(friendly, enemy, false, &mut AirstrikeOutput {
    damage: &mut api_edam,
    bak_targets: &mut api_fbak,
    rai_targets: &mut api_frai,
    bak_flags: &mut api_ebak_flag,    // enemy was bombed → ebak
    rai_flags: &mut api_erai_flag,    // enemy was torpedoed → erai
});

// Second call (enemy attacking friendly) — defenders are friendly
execute_airstrike_phase(enemy, friendly, true, &mut AirstrikeOutput {
    damage: &mut api_fdam,
    bak_targets: &mut api_ebak,
    rai_targets: &mut api_erai,
    bak_flags: &mut api_fbak_flag,    // friendly was bombed → fbak
    rai_flags: &mut api_frai_flag,    // friendly was torpedoed → frai
});
```

**Test file:** `crates/emukc_battle/src/simulation/kouku.rs` (mod tests)
- Test: kouku flag arrays match fleet sizes when friendly and enemy fleets have different sizes (e.g., 3 friendly, 6 enemy)
- Test: kouku flag arrays have correct entries after airstrike (api_fbak_flag[friendly_idx] == 1 when friendly ship was bombed, api_ebak_flag[enemy_idx] == 1 when enemy ship was bombed)

### U2: Fix married HP bonus write path [P1]

**Files:**
- `crates/emukc_gameplay/src/game/ship/mod.rs` (line 773)
- `crates/emukc_gameplay/src/game/compose/powerup.rs` (line 381)
- `crates/emukc_gameplay/src/game/presets/slot.rs` (line 360)

**Problem:** `update_ship_from_api_impl` sets `am.married = ActiveValue::NotSet`, meaning the married DB column is never updated. When a ship reaches level 100 through battle, the married column stays false. Additionally, `powerup.rs` and `presets/slot.rs` hardcode `married=false` in `cal_ship_status()` calls, so married ships lose their HP bonus during modernization and equipment changes.

**Fix:**
1. In `update_ship_from_api_impl` (ship/mod.rs:773): set `am.married = ActiveValue::Set(api_ship.api_lv > 99)` — restore the married write from level
2. In `powerup.rs:381`: change `false` to `ship.married` (read from DB model, which is now correctly maintained)
3. In `presets/slot.rs:360`: change `false` to `ship.married`

**Test file:** `crates/emukc_model/src/codex/ship.rs`
- Test: level 100 ship with `married=true` gets HP bonus
- Test: level 100 ship with `married=false` does not get HP bonus
- Test: level 99 ship with `married=true` gets HP bonus

### U3: Fix enemy damage_dealt tracking [P1, pre-existing]

**Files:**
- `crates/emukc_battle/src/simulation/torpedo.rs` (lines 55-56 pattern, missing at ~79)
- `crates/emukc_battle/src/simulation/asw.rs` (line 73)

**Problem:** Enemy torpedo and ASW loops apply damage to defenders but don't accumulate `ship.damage_dealt += dealt` on the attacker. The friendly loops do this correctly. The night battle enemy loop was fixed in this diff (night.rs:436), confirming it's a known oversight.

**Fix:**
1. In `torpedo.rs` opening torpedo enemy loop: add `ship.damage_dealt += dealt;` after `apply_damage`
2. In `torpedo.rs` closing torpedo (raigeki) enemy loop: add `ship.damage_dealt += dealt;`
3. In `asw.rs` enemy OASW loop: add `ship.damage_dealt += dealt;`

**Test file:** `crates/emukc_battle/src/simulation/torpedo.rs` (new mod tests)
- Test: enemy ships that deal torpedo damage have `damage_dealt > 0`

**Test file:** `crates/emukc_battle/src/simulation/asw.rs` (extend existing tests)
- Test: enemy ships that deal ASW damage have `damage_dealt > 0`

### U4: Extract display_damage helper [P1, maintainability]

**Files:**
- `crates/emukc_battle/src/targeting.rs` (add helper)
- `crates/emukc_battle/src/simulation/shelling.rs` (line 61)
- `crates/emukc_battle/src/simulation/asw.rs` (line 46)
- `crates/emukc_battle/src/simulation/night.rs` (line 380)
- `crates/emukc_battle/src/simulation/torpedo.rs` (lines 49, 128)

**Problem:** Overkill display logic uses inconsistent predicates across phases. Shelling checks `attacker_is_enemy || !defender.is_sortie`, while ASW/night/torpedo check `defender.is_sortie`. Both produce the same result for current use cases but the inconsistency is a maintenance hazard.

**Fix:** Extract a shared helper in `targeting.rs`:
```rust
/// Choose the display damage value for the battle animation log.
/// In sortie (is_sortie=true), show raw pre-protection damage for the overkill visual effect.
/// In practice (is_sortie=false), show the actual dealt (capped) damage.
pub(crate) fn display_damage(defender: &BattleRuntimeShip, raw: i64, dealt: i64) -> i64 {
    if defender.is_sortie { raw } else { dealt }
}
```

Then replace all 5 call sites. The shelling variant that also checks `attacker_is_enemy` can simplify to the same logic since `is_sortie` already encodes the context.

**Test file:** `crates/emukc_battle/src/targeting.rs` (extend tests)
- Test: sortie defender returns raw damage
- Test: practice defender returns dealt damage

### U5: Kouku deeper investigation [P1, user-requested]

**Files:**
- `crates/emukc_battle/src/simulation/kouku.rs`
- `crates/emukc_battle/src/targeting.rs`

**User concern:** "航空战存在不少的问题，雷击轨迹，敌方没有空袭能力的舰船也参与了空袭阶段等问题"

**Investigation tasks:**

5a. **Torpedo trail display (api_frai/api_erai semantics):** The `bak_targets` and `rai_targets` arrays are indexed by attacker ship position. Verify against client code and wikiwiki that the client uses these for torpedo/bomb trail animation. Confirm the indices are attacker-relative (1-based in the client). Current code at kouku.rs:271 sets `bak_targets[ship_idx] = target_idx as i64` — this is 0-based attacker index → 0-based defender index. Check if the client expects 1-based indices.

5b. **Non-air-capable ships in airstrike:** `execute_airstrike_phase` iterates ALL attacker ships, but skips slots where `is_airstrike_attack_type` returns false (kouku.rs:244). This correctly filters non-bomber equipment. However, check if ships with zero airstrike-capable slots still appear in `api_plane_from`. The `attack_plane_from` function (kouku.rs:77-93) only includes ships that have at least one slot with airstrike-capable equipment, which is correct. Verify that enemy ships composed of BB/CA/DD (which have no bomber equipment slots) are correctly excluded from damage dealing in stage 3.

5c. **api_plane_from field ordering:** The field is `[attack_plane_from(friendly), attack_plane_from(enemy)]`. Verify this matches the client expectation.

**Expected outcome:** Document findings. If torpedo trail indices are wrong, add a fix unit. If non-air-capable ships are correctly filtered (current evidence suggests they are), document the confirmation.

### U6: Fix P2 safe_auto items

**6a. Remove dead NightBattleInput.is_sortie field [P3 → safe_auto]**

**Files:**
- `crates/emukc_battle/src/types/runtime.rs` (line 396)
- Call sites in `simulation/night.rs`, `game/battle/sortie/orchestrate.rs`, `game/battle/practice/orchestrate.rs`

**Fix:** Remove `pub is_sortie: bool` from `NightBattleInput`. Remove the argument from all 3 call sites.

**6b. Update cal_ship_status doc comment [P2, safe_auto]**

**File:** `crates/emukc_model/src/codex/ship.rs` (line 185)

**Fix:** Update doc comment to document the `married: bool` parameter.

**6c. Add debug_assert to set_hourai_flag [P2, gated_auto]**

**File:** `crates/emukc_battle/src/state.rs` (line 153)

**Fix:** Add `debug_assert!(index < 4, "hourai_flag index out of bounds: {index}");`

### U7: Add critical test coverage

**Test file:** `crates/emukc_battle/src/damage.rs`
- CV shelling formula test: CV with known bomber slots and onslot values, verify `1.5 * bomber_plane_count + 55` basic power (tests reviewer finding: bomber_slot_count → bomber_plane_count change has no test)
- ASW cumulative type_bonus test: ship with both ASW aircraft and depth charge, verify type_bonus = 21 (8+13)

**Test file:** `crates/emukc_model/src/codex/repair.rs` (new mod tests)
- CT repair time uses reduced formula (no sqrt term)
- Non-CT ship at same level uses standard formula

**Test file:** `crates/emukc_battle/src/simulation/night.rs`
- Enemy night battle damage_dealt > 0 for attacking enemy ships

### U8: Practice win_rank round-trip fix [P2]

**File:** `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs` (line 160-177)

**Problem:** `KcSortieResultRank` is converted to `String` for snapshot, then immediately parsed back via `match snapshot.win_rank.as_str()`. Fallthrough to `KcSortieResultRank::E` is silent data corruption.

**Fix:** Change `PracticeBattleResultSnapshot.win_rank` from `String` to `KcSortieResultRank` (derive Serialize/Deserialize). Remove the string-to-enum match. Store `simulation.outcome.win_rank` directly.

**Test:** Add test verifying all rank variants survive the snapshot round-trip.

## Advisory Items (document, don't implement)

| Finding | Severity | Notes |
|---------|----------|-------|
| Night chuuha +18 for TorpTorpTorp not gated on DD ship type | P2 | Confidence 50%. Research suggests all TorpTorpTorp ships are DDs in practice, but a ship-type guard would be safer. Requires codex lookup in night_ci_trigger_rate. |
| Fleet speed computed once at battle start | P2 | Architectural. Move enemy_shells_first computation after kouku/OASW/torpedo. Large change. |
| Night CI pre-cap vs post-cap multiplier for DoubleAttack | P2 | Requires KanColle wiki verification. DA 1.2x may be post-cap. |
| Map routing random fallback | P1 | Changed from deterministic to random. Needs verification against actual map data. Add log warning when triggered. |
| NightBattleParams dead fields | P3 | Remove or document. |
| BattleOutcome.can_midnight dead code | P3 | Verify if consumed. |
| Torpedo payload dynamic sizing (no test for combined fleet 12 ships) | P3 | Low priority. |

## Key Technical Decisions

**KD1: Married flag restored to `api_lv > 99` in update_ship.** The original fix (NotSet) was intended to stop overwriting married from level, but it introduced a regression where married ships never get the flag set. Restoring `api_lv > 99` is safe because KanColle's marriage system is strictly tied to level 100+. The explicit `married` parameter on `cal_ship_status` remains — it decouples the computation from the level check.

**KD2: display_damage helper normalizes overkill display.** The inconsistent `attacker_is_enemy` check in shelling is subsumed by the simpler `defender.is_sortie` check. In sortie, all defenders on the friendly side have `is_sortie=true`, so showing raw damage for them is always correct regardless of attacker side.

**KD3: Kouku flag arrays follow "f = about friendly fleet, e = about enemy fleet" convention.** `api_fbak_flag[i]` = "was friendly ship #i bombed?" — sized for friendly fleet. `api_ebak_flag[i]` = "was enemy ship #i bombed?" — sized for enemy fleet. The `execute_airstrike_phase` function sets flags by defender index, so defenders-of-enemy → `api_ebak_flag`, defenders-of-friendly → `api_fbak_flag`.

## Dependencies

- U1 (kouku flags) is independent — can start immediately
- U2 (married) and U6b (doc comment) are independent
- U3 (enemy damage_dealt) is independent
- U4 (display_damage helper) is independent
- U5 (kouku investigation) should run after U1 (context from the fix informs the investigation)
- U6a (is_sortie removal) depends on verifying no call site uses it — quick check
- U6c (debug_assert) is independent
- U7 (tests) should run after U1-U3 to test the fixes
- U8 (practice round-trip) is independent

## Suggested Execution Order

1. U1: Fix kouku flag arrays (P0, highest priority)
2. U2: Fix married HP bonus (P1)
3. U3: Fix enemy damage_dealt (P1)
4. U5: Kouku deeper investigation (informs any additional fixes)
5. U4: Extract display_damage helper (P1, cross-cutting)
6. U6: Fix P2 safe_auto items
7. U7: Add critical test coverage
8. U8: Practice win_rank round-trip
