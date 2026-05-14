---
title: "fix: battle display damage bugs — kouku overkill and sinking protection display"
type: fix
status: completed
date: 2026-05-03
---

# fix: battle display damage bugs — kouku overkill and sinking protection display

## Summary

Fix two display-layer bugs in the battle simulation: (1) kouku phase uses capped `dealt` instead of raw damage in output arrays, preventing overkill damage display against enemies; (2) `display_damage()` returns raw pre-protection values for friendly defenders in sorties, causing client-side HP miscalculation when sinking protection triggers.

---

## Requirements

- R1. Kouku stage 3 `api_edam`/`api_fdam` arrays must show full raw damage (including overkill), matching original KanColle behavior
- R2. Shelling, torpedo, night, and ASW phases must show actual effective damage (after sinking protection) when enemy attacks friendly ships
- R3. Overkill display must work for attacks against enemy ships in all phases
- R4. Server-side HP tracking (via `apply_damage`) must remain unchanged — protection logic is correct

---

## Scope Boundaries

- Bug 3 (map routing) is excluded — handled separately
- `apply_damage` sinking protection logic is not modified (confirmed correct)
- Damage calculation formulas are not modified
- Client-side HP tracking is out of scope (server provides correct data)

---

## Key Technical Decisions

- **KDU1**: `display_damage` semantics change from `is_sortie`-based to `is_friendly`-based: return `raw` for enemy defenders (overkill effect), `dealt` for friendly defenders (actual HP change). This correctly handles both sortie and practice battles.
- **KDU2**: Kouku `execute_airstrike_phase` captures `raw_damage` from `apply_damage` return value instead of discarding it, uses `raw_damage` for `output.damage` accumulation. `dealt` was already used implicitly for HP (inside `apply_damage`).
- **KDU3**: Torpedo phases (opening/closing) that bypass `display_damage` and use `dealt` directly for enemy→friendly attacks need no change — they already show effective damage, which is correct for the client. But for friendly→enemy attacks they should use `display_damage` for consistency with the overkill pattern.

---

## Implementation Units

- U1. **Fix `display_damage` to distinguish friendly vs enemy defenders**

**Goal:** Change `display_damage` predicate from `is_sortie` to `!is_friendly` so enemy defenders get overkill display and friendly defenders get actual dealt damage.

**Requirements:** R2, R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/targeting.rs`

**Approach:**
- Change `display_damage` condition from `defender.is_sortie` to `!defender.is_friendly`
- This automatically handles: sortie enemy→friendly shows `dealt` (protection-safe), sortie friendly→enemy shows `raw` (overkill), practice shows `dealt` for both sides (practice `raw == dealt` when no protection, or `dealt = min(raw, hp)` which is the actual HP change)

**Patterns to follow:**
- Current `display_damage` signature and call sites in shelling.rs, asw.rs, torpedo.rs, night.rs

**Test scenarios:**
- Happy path: sortie friendly→enemy with raw=200, hp=50 returns 200 (overkill)
- Happy path: sortie enemy→friendly with raw=200, dealt=30 (protection) returns 30
- Happy path: practice enemy→friendly returns dealt (no protection modifier)
- Edge case: flagship protection returns dealt not raw

**Verification:**
- Existing tests pass (display_damage contract unchanged for enemy defenders in sortie)
- New test confirms friendly defenders in sortie get `dealt`

---

- U2. **Fix kouku overkill damage display**

**Goal:** Kouku `execute_airstrike_phase` uses raw damage in output arrays instead of capped `dealt`.

**Requirements:** R1, R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/simulation/kouku.rs`

**Approach:**
- Capture `raw_damage` from `apply_damage` return value (currently discarded with `_`)
- Use `raw_damage` for `output.damage` accumulation (the display array)
- HP tracking is unaffected (handled inside `apply_damage`)
- Apply to both dive-bombing and torpedo-bombing loops (lines ~269 and ~311)

**Patterns to follow:**
- Shelling phase pattern: `let (raw_dmg, dealt) = ...; let display = display_damage(..., raw_dmg, dealt);`

**Test scenarios:**
- Happy path: kouku against enemy with HP=50, raw damage=200 → `api_edam` shows 200
- Happy path: kouku against enemy with HP=200, raw damage=50 → `api_edam` shows 50
- Integration: kouku + sinking protection: enemy kouku on protected friendly ship, `api_fdam` shows raw (client handles protection)
- Edge case: multiple bomber slots hitting same target — damage accumulates correctly with raw values

**Verification:**
- Existing kouku tests pass
- New test verifies `api_edam` can exceed enemy HP

---

- U3. **Unify torpedo damage display with `display_damage`**

**Goal:** Torpedo phases (opening/closing) use `display_damage` for consistent overkill display when friendly attacks enemy, instead of raw `dealt`.

**Requirements:** R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_battle/src/simulation/torpedo.rs`

**Approach:**
- Friendly→enemy attacks: already use `display_damage` (lines 49, 129) — verify they work correctly after U1
- Enemy→friendly attacks (lines 79, 159): currently use `dealt` directly. After U1, `display_damage` for friendly defenders returns `dealt`, so switching to `display_damage` is semantically identical but more consistent. Optionally leave as-is since `dealt` is already correct for client HP tracking.
- Only change if needed for consistency; the current `dealt` usage for enemy→friendly is correct.

**Patterns to follow:**
- Shelling phase: all attacks go through `display_damage`

**Test scenarios:**
- Happy path: opening torpedo friendly→enemy shows overkill damage
- Happy path: opening torpedo enemy→friendly with protection shows dealt damage
- Integration: closing torpedo enemy→friendly does not overstate damage

**Verification:**
- Existing torpedo tests pass
- Display values match expected overkill/dealt pattern

---

- U4. **Verify and update existing tests**

**Goal:** Ensure all existing tests reflect the new display damage semantics, add regression tests for both bugs.

**Requirements:** R1, R2, R3, R4

**Dependencies:** U1, U2, U3

**Files:**
- Modify: `crates/emukc_battle/src/targeting.rs` (test module)
- Modify: `crates/emukc_battle/src/simulation/kouku.rs` (test module)
- Possibly: `crates/emukc_battle/src/types/runtime.rs` (existing sinking protection tests)

**Approach:**
- Add `display_damage` unit test: friendly defender in sortie returns `dealt`, enemy defender returns `raw`
- Add kouku integration test: `api_edam` exceeds enemy HP (overkill)
- Add kouku integration test: `api_fdam` shows raw damage from enemy air strike on protected friendly ship
- Verify existing sinking protection tests still pass (R4 — HP tracking unchanged)
- Run full `cargo test -p emukc_battle` to catch regressions

**Test scenarios:**
- Regression: all existing `emukc_battle` tests pass
- New: display_damage friendly defender sortie → dealt
- New: display_damage enemy defender sortie → raw
- New: kouku api_edam overkill
- New: day battle integration with sinking protection — protected ship survives, display values correct

**Verification:**
- `cargo test -p emukc_battle` passes
- `cargo test -p emukc_gameplay` passes (sortie battle validation tests)

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `display_damage` change affects practice battles | Practice has `is_sortie=false`; new condition `!is_friendly` is independent of `is_sortie`, so practice behavior unchanged for enemy→friendly. For friendly→enemy in practice, `is_friendly=false` on enemy defenders returns `raw`, which equals `dealt` in practice (no overkill expected). |
| Kouku raw damage makes client show wrong HP | Original KanColle client handles kouku damage independently from HP tracking — `api_edam`/`api_fdam` have always shown raw values. Server-side HP (`current_hp`) is correct. |
| Torpedo consistency change introduces subtle display difference | Enemy→friendly torpedo already uses `dealt`, which equals `display_damage` result after U1. Semantic no-op. |

---

## Sources & References

- Root cause analysis: prior conversation diagnostic findings
- Related: `crates/emukc_battle/src/types/runtime.rs` (apply_damage sinking protection)
- Related: `crates/emukc_battle/src/targeting.rs` (display_damage)
- Related: `crates/emukc_battle/src/simulation/kouku.rs` (kouku damage output)
- Related: `crates/emukc_battle/src/simulation/torpedo.rs` (torpedo damage display)
- Related: `crates/emukc_battle/src/simulation/shelling.rs` (reference pattern for display_damage usage)
