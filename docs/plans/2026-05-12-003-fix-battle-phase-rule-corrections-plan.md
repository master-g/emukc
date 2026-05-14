---
title: "fix: Battle phase rule corrections (Shelling2 BB-gate, closing torpedo chuuha, enemy slot alignment)"
type: fix
status: completed
date: 2026-05-12
---

# fix: Battle Phase Rule Corrections

## Summary

Correct three KanColle rule deviations in the battle simulator: (1) add a battle-start BB-class gate to Shelling2 so non-BB fleets only shell once, (2) reject chūha (HP ≤ 50%) ships from closing torpedo eligibility, and (3) preserve slot positions in `new_enemy_ship` so that abyssal carriers with sparse slot layouts participate in kouku instead of silently zeroing out. All three are bounded, low-risk fixes scoped to one crate each.

---

## Problem Frame

Three independent deviations from canonical KanColle battle rules were identified during a debug review:

1. **Shelling2 runs unconditionally.** Authoritative sources (wikiwiki 戦闘について, 神ゲー, アットウィキ, ぜかましねっと) agree: the 2nd shelling round requires ≥1 battleship-class ship (FBB / BB / BBV / XBB, including abyssal 鬼/姫 internal BB types) on *either* side at battle start. A BB sunk in round 1 still triggers round 2. Current `BattleFlow::SURFACE_DAY` (`crates/emukc_battle/src/config.rs`) lists Shelling2 unconditionally and `execute_shelling2` only gates on `any_alive`.

2. **Closing torpedo accepts chūha ships.** wikiwiki and blogger / 神ゲー agree: 閉幕雷撃 rejects ships with HP ≤ maxhp/2 (中破 or worse). 開幕雷撃 (opening torpedo), by contrast, is damage-agnostic. Current `can_closing_torpedo_ship` (`crates/emukc_battle/src/targeting.rs:231`) only filters by `is_sunk() || api_raisou[0] <= 0`.

3. **Abyssal carriers can silently drop out of kouku.** `Codex::new_enemy_ship` (`crates/emukc_model/src/codex/ship.rs:95`) tight-packs equipment into the `slot_items: Vec` by skipping empty slots, but leaves `api_onslot: [i64; 5]` in positional layout and sets `onslot[idx] = 0` for empty positions. Every kouku consumer (`crates/emukc_battle/src/simulation/kouku.rs:43, 61, 83, 119, 233, 283`) joins the two with `slot_items.iter().zip(api_onslot)`, which indexes aircraft counts by *Vec position*, not *slot position*. When an abyssal CV has e.g. `[item_id, 0, item_id, item_id, 0]` slot layout, its equipment reads aircraft counts from the wrong `api_onslot` cells and may see `0` where it should see a full slot — so the CV reports "no planes" and the kouku gate drops it.

---

## Requirements

- R1. Shelling2 runs only if ≥1 friendly-or-enemy ship of class FBB/BB/BBV/XBB was present at battle start, regardless of current survival.
- R2. `can_closing_torpedo_ship` rejects any ship with `hp() * 2 <= ship.api_maxhp` in addition to existing sunk / zero-torpedo filters.
- R3. `can_opening_torpedo_ship` remains damage-agnostic (regression guard — the current behavior is correct per rules).
- R4. Enemy ships built via `Codex::new_enemy_ship` maintain positional alignment between `slot_items` and `api_onslot` so every existing `slot_items.iter().zip(api_onslot)` consumer sees the correct aircraft count per slot.

---

## Scope Boundaries

- Issue 1 root-cause confirmation covers **H1c only**. H1a (map composition data), H1b (missing `enemy_ship_extra` for specific abyssal CVs), and H1d (Stage 2 AA linear-simplification over-killing enemy planes) are **not** in scope for this plan.
- No changes to `kouku.rs` Stage 2 damage formula. The existing `"known simplification"` comment at `kouku.rs:362-365` stands; a per-ship AA rewrite belongs in a separate plan.
- No changes to `can_opening_torpedo_ship`. Rule is intentional: 先制雷撃 ignores damage state.
- No changes to `config.rs::SURFACE_DAY.phases`. BB-gate is enforced at phase execution time, not by conditional phase inclusion, because the phase list is `&'static [BattlePhaseKind]` and the gate depends on runtime fleet composition.

### Deferred to Follow-Up Work

- H1a reproduction audit: once a user-provided battle input JSON surfaces a "no enemy airstrike" case, run `cargo run -- battle validate --input <battle.json>` and check `composition.ship_ids` against `api_stype ∈ {7, 11, 18}` for CVL/CV/CVB.
- H1b data gap: add `enemy_ship_extra` entries for any abyssal CVs that currently fall into the `build_manifest_only_sortie_enemy_ship` branch (detectable via the `warn!: enemy bootstrap data missing; using manifest-only sortie enemy fallback` log at `crates/emukc_gameplay/src/game/sortie/enemy_ship.rs:112`).
- H1d Stage 2 AA rewrite: per-ship AA contribution modeling, replacing the `friendly_fleet_aa / 400 * enemy_planes` linear approximation.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_battle/src/config.rs` — `BattleFlow::SURFACE_DAY.phases` is a `&'static [BattlePhaseKind]`. Gating at dispatch time (`simulation/mod.rs::simulate_day`) is the established pattern — compare `execute_opening_torpedo` at `simulation/mod.rs:96-97` which gates on `can_opening_torpedo(codex, &state.friendly) || can_opening_torpedo(codex, &state.enemy)`.
- `crates/emukc_battle/src/state.rs` — `BattleState` fields are `pub(crate)` for phase-read access with private setters. Battle-start invariants belong here, set once in `from_context` (see `is_sortie` threading pattern at `state.rs:41-51`).
- `crates/emukc_battle/src/targeting.rs:186-188` — `ship_type(codex, ship) -> Option<KcShipType>` is the canonical stype lookup used throughout phase eligibility code.
- `crates/emukc_battle/src/targeting.rs:231-253` — `can_closing_torpedo_ship` is the fix site for R2. Follows the shape: `is_sunk() || zero-stat` early-return, then `matches!(ship_type, Some(…))` stype whitelist.
- `crates/emukc_battle/src/types/runtime.rs:56` — `BattleRuntimeShip::hp()` is the canonical HP accessor; `ship.ship.api_maxhp` is the max. Existing chūha/taiha comparisons use `ship.api_maxhp / 2` or `ship.entry_hp * 4 <= api_maxhp` (see `night.rs:214`, `outcome.rs:67`).
- `crates/emukc_model/src/codex/ship.rs:95-148` — `new_enemy_ship` is the fix site for R4. Current loop at `ship.rs:108-133` conditionally `continue`s on empty slots, pushing only non-empty items, so the resulting `Vec` length ≠ slot index.
- `crates/emukc_model/src/kc2/types/ship.rs:4-50` — `KcShipType` enum: `FBB = 8`, `BB = 9`, `BBV = 10`, `XBB = 12`. Reference pattern in `crates/emukc_model/src/codex/repair.rs:54-62` for matching the battleship family.
- `crates/emukc_battle/src/test_utils.rs` — `sample_ship`, `first_ship_mst_by_type`, `make_test_ship_ctx`, `slotitem_with_mst_id` are the test-fixture helpers; `Codex::load_without_cache_source("../../.data/codex")` is the canonical codex load in battle tests.

### Institutional Learnings

- `docs/plans/2026-05-10-002-fix-fallback-enemy-and-kouku-damage-display-plan.md` touched adjacent enemy-ship build pathways. Reviewing for regression risk: that plan rewired `build_sortie_enemy_ship` to prefer `new_enemy_ship`, which makes R4 higher-impact now because more enemy ships flow through the misaligned zip.
- No prior `docs/solutions/` entry on slot-packing vs positional-`api_onslot`. This plan should leave a learnings artifact when U3 lands.

### External References

- wikiwiki 戦闘について (Japanese KC community wiki — authoritative) for all three rules. Four independent sources (wikiwiki, 神ゲー, アットウィキ, ぜかましねっと) concur on the BB-gate; two independent sources concur on closing-torpedo chūha rejection.

---

## Key Technical Decisions

- **BB-gate lives on `BattleState`, snapshot at `from_context`, not re-derived each phase.** Rationale: the rule is "BB present *at battle start*" — sinking a BB in round 1 must not disable round 2. A snapshot field fits this semantic exactly, avoids repeated codex lookups, and mirrors the existing `battle_type` / `is_sortie` pattern already on `BattleState`.
- **BB-class check uses `ship_type()` from `targeting.rs`, not direct `api_stype` inspection.** Rationale: `ship_type` already handles the `find::<ApiMstShip>().ok()` lookup and returns a typed `KcShipType`. Direct inspection would duplicate the unwrap ladder. One module-level helper `fleet_has_bb_class(codex, ships)` defined in `targeting.rs` keeps the battleship-family enum match (`FBB | BB | BBV | XBB`) co-located with the rest of the stype whitelists.
- **U3 fix point is `new_enemy_ship` (producer), not the six kouku consumers.** Rationale: every current consumer assumes positional alignment. Fixing the producer is one contract change; fixing the consumers is six behavior changes with a higher regression surface. The fix is to push a sentinel `KcApiSlotItem` entry (e.g., `api_slotitem_id: 0`) for empty slots so `slot_items.len() == api_slotnum` and positional alignment holds. All existing consumers already guard on `slot_item.api_slotitem_id > 0` or `codex.find::<ApiMstSlotitem>(...)` succeeding (verify during implementation), so sentinel entries are transparent.
- **Alternative considered: pad `slot_items` with `Option<KcApiSlotItem>` or track slot indices separately.** Rejected: changes the public shape of `KcApiShip`/`KcApiSlotItem` and propagates across unrelated serializers. Sentinel-with-id-0 is a contained change inside `new_enemy_ship`.
- **R2 boundary: `hp() * 2 <= api_maxhp` matches the canonical 中破 threshold (HP ≤ 50%).** Multiplying (`hp * 2`) instead of dividing (`maxhp / 2`) avoids integer-division ambiguity at odd max HP (e.g., maxhp=7 → `/2 = 3`, so hp=3 must reject; `hp * 2 = 6 <= 7` ✓).

---

## Open Questions

### Resolved During Planning

- **Where does the BB-gate execute?** At `execute_shelling2` dispatch, reading `state.has_bb_class_at_start`. `config.rs::SURFACE_DAY.phases` stays untouched — the phase list is static; the gate is runtime.
- **Do we need to gate Shelling1 too?** No. 1 round is unconditional; only round 2 requires BB.
- **Does R4 require changes to friendly-ship construction (`new_ship`)?** No. `new_ship` at `crates/emukc_model/src/codex/ship.rs:50-92` (verify offsets at implementation time) follows a different code path; friendly ships come from user fleet state with `api_slot[idx]` directly populated. Only the enemy-build path is affected.

### Deferred to Implementation

- **Exact sentinel predicate for `new_enemy_ship` consumers.** The fix pushes `KcApiSlotItem { api_slotitem_id: 0, ... }` for empty slots. Implementer should run `rg 'slot_items.iter\(\)' crates/ --include='*.rs'` and verify every consumer either (a) filters by `api_slotitem_id > 0` or (b) falls through benignly when `codex.find::<ApiMstSlotitem>(&0)` returns `None`. Most should; any consumer that does `codex.find::<ApiMstSlotitem>(&id).unwrap()` is a latent panic to fix during U3.
- **Whether U1 snapshot needs `ApiMstShip` lookups that could fail.** `ship_type()` returns `Option<KcShipType>`; a missing manifest entry yields `None`. Production behavior: unknown-type ships contribute nothing to BB-gate (safer under-trigger than over-trigger). Implementer decides whether to `warn!` on `None` for an enemy ship during `from_context`.

---

## Implementation Units

### U1. Add BB-class gate to Shelling2

**Goal:** Shelling2 executes only if the battle started with ≥1 FBB/BB/BBV/XBB on either side.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/targeting.rs` (add `fleet_has_bb_class` helper)
- Modify: `crates/emukc_battle/src/state.rs` (add `has_bb_class_at_start: bool` field; set in `from_context`; expose read accessor; update `for_night` to default to `false`)
- Modify: `crates/emukc_battle/src/simulation/mod.rs` (gate `execute_shelling2`)
- Test: `crates/emukc_battle/src/simulation/mod.rs` (new tests in the existing `mod tests` block)

**Approach:**
- Add `pub(crate) fn fleet_has_bb_class(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool` to `targeting.rs`, co-located with the other fleet-level eligibility functions (`can_opening_torpedo`, `can_closing_torpedo`, `any_alive`). Matches on `ship_type(codex, ship) == Some(FBB | BB | BBV | XBB)`. Does **not** filter by `is_alive` — we want battle-start presence.
- In `BattleState::from_context`, after constructing `friendly` and `enemy`, compute `has_bb_class_at_start = fleet_has_bb_class(&codex, &friendly) || fleet_has_bb_class(&codex, &enemy)`. But note: `from_context` does not currently take `&Codex` — this means either threading codex through `from_context`, or computing the flag in `simulate_day` after `BattleState::from_context` and injecting via a new `state.set_has_bb_class_at_start(bool)` setter. Prefer the setter approach — it keeps `BattleState` codex-agnostic, matching its current design. For `for_night`, default to `false` (night battles don't run day Shelling2).
- In `simulate_day` (`simulation/mod.rs`), after `let mut state = BattleState::from_context(context);`, compute and set the flag before entering the phase loop.
- Change `execute_shelling2` signature to also read the flag from `state`, or pass it explicitly. Simplest: add `if !state.has_bb_class_at_start() { return; }` as the first line of the function body (before the `any_alive` check).

**Patterns to follow:**
- `fleet_has_bb_class` mirrors `can_closing_torpedo` / `can_opening_torpedo` at `targeting.rs:437-445`.
- `BattleState` field + setter follows the existing `opening_taisen_flag` / `stage_flag` pattern at `state.rs:34-35, 133-151`.

**Test scenarios:**
- Covers R1. Happy path: friendly DD + enemy DD, no BB on either side → `simulation.packet.hougeki2` is `None` and `packet.hourai_flag[2] == 0`.
- Covers R1. Happy path: friendly BB + enemy DD → `hougeki2.is_some()` and `hourai_flag[2] == 1`.
- Covers R1. Happy path: friendly DD + enemy BB → `hougeki2.is_some()`. Confirms the gate checks *either* side.
- Covers R1. Edge case — BB sunk in round 1 still triggers round 2: friendly BB with `api_nowhp = 1` and overwhelming enemy firepower such that BB sinks in Shelling1; assert `hougeki2.is_some()`. Use deterministic `SeededRng` and configure enemy karyoku / BB armor so sinking is guaranteed (mirror the sinking-setup pattern used in the existing `sortie_day_battle_enables_midnight_when_both_sides_survive` test at `mod.rs:290-320`).
- Covers R1. Edge case: CVL + DD fleet (no BB-class) → `hougeki2.is_none()` even though CVL is capital ship. Confirms the whitelist is exactly `{FBB, BB, BBV, XBB}`.
- Edge case: FBB (`KcShipType::FBB`) triggers the gate.
- Edge case: BBV triggers the gate.
- Edge case: unknown/sunken `api_ship_id` with no manifest entry does not panic and is treated as non-BB (safe under-trigger).

**Verification:**
- All new tests pass.
- Existing simulation tests that use BB fleets still see 2 shelling rounds.
- `cargo clippy --workspace --all-targets` is clean.

---

### U2. Reject chūha ships from closing torpedo

**Goal:** Ships at HP ≤ 50% cannot participate in 閉幕雷撃. Opening torpedo is untouched.

**Requirements:** R2, R3

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_battle/src/targeting.rs` (tighten `can_closing_torpedo_ship`)
- Test: `crates/emukc_battle/src/targeting.rs` (extend existing `mod tests`)

**Approach:**
- In `can_closing_torpedo_ship` (`targeting.rs:231`), add `if ship.hp() * 2 <= ship.ship.api_maxhp { return false; }` after the existing `is_sunk() || api_raisou[0] <= 0` early-return and before the stype whitelist.
- Leave `can_opening_torpedo_ship` (`targeting.rs:219`) untouched. Add a regression test anyway to pin behavior.

**Patterns to follow:**
- `hp() * 2 <= api_maxhp` multiply-form matches the integer-safe comparison idiom; compare with `was_taiha_at_entry` formula in `crates/emukc_battle/src/types/runtime.rs:95`.
- Test-fixture helper: `make_test_ship_ctx` (from `test_utils.rs:86`) constructs a `BattleRuntimeShip` with explicit `nowhp` / `maxhp`. For stype-specific tests, combine with `sample_ship(&codex, dd_mst, 50)` then override `api_nowhp` directly, following the pattern at `targeting.rs:653-677`.

**Test scenarios:**
- Covers R2. Happy path: healthy DD (hp = maxhp, raisou > 0) → `can_closing_torpedo_ship` returns `true`.
- Covers R2. Edge case — chūha boundary exact: DD with `hp = maxhp / 2` (integer) → returns `false`. E.g. maxhp=10, hp=5.
- Covers R2. Edge case — shouha (still ≥ 50% + 1): DD with `hp = maxhp / 2 + 1` → returns `true`. E.g. maxhp=10, hp=6.
- Covers R2. Edge case — odd maxhp boundary: maxhp=7, hp=3 → `3*2=6 ≤ 7` → `false`. maxhp=7, hp=4 → `4*2=8 > 7` → `true`. Confirms multiply-form is integer-safe.
- Edge case — zero hp: already rejected by `is_sunk()` pre-existing check; confirm this test still passes.
- Covers R3 (regression guard). Opening torpedo accepts chūha: chūha CLT (maxhp=10, hp=3) with positive `api_raisou[0]` → `can_opening_torpedo_ship` returns `true`. Damage-agnostic rule is intentional.
- Regression: chūha DD with `api_raisou[0] = 0` returns `false` for both opening and closing (pre-existing raisou filter still wins).
- Regression: BB-class ship (not in closing-torpedo whitelist) → `can_closing_torpedo_ship` remains `false` regardless of HP.

**Verification:**
- New tests pass; existing targeting tests still pass.
- `cargo clippy -p emukc_battle --all-targets` is clean.

---

### U3. Preserve slot position in `new_enemy_ship`

**Goal:** `slot_items` produced by `Codex::new_enemy_ship` stays positionally aligned with `api_onslot` so `slot_items.iter().zip(api_onslot)` reads the correct aircraft count per slot.

**Requirements:** R4

**Dependencies:** None (independent of U1, U2)

**Files:**
- Modify: `crates/emukc_model/src/codex/ship.rs` (fix packing in `new_enemy_ship`)
- Test: `crates/emukc_model/src/codex/ship.rs` (new unit test) and `crates/emukc_battle/src/simulation/kouku.rs` (new integration test targeting an abyssal CV with sparse slot layout, if such fixture is available)

**Approach:**
- In `new_enemy_ship` at `crates/emukc_model/src/codex/ship.rs:95-148`, replace the conditional `continue` on empty/invalid slots with a sentinel push. For empty slots (`slot_info.item_id <= 0`) and for missing-manifest slots, push `KcApiSlotItem { api_id: 0, api_slotitem_id: 0, api_locked: 0, api_level: 0, api_alv: None }` and still zero `onslot[idx]`. Result: `slot_items.len() == basic.slots.len()`, positional alignment guaranteed.
- Audit all `slot_items.iter()` consumers to confirm sentinel (`api_slotitem_id == 0`) is transparent:
  - `crates/emukc_battle/src/simulation/kouku.rs` — every `zip(api_onslot)` call filters by looking up the slotitem via `codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()` and dropping on `None`. `api_slotitem_id == 0` returns `None` from manifest lookup → consumer correctly sees "no plane". Verified at `kouku.rs:43-48` (verify exact predicate at implementation time).
  - Other call sites: run `rg 'slot_items\.iter\(\)' crates/ --include='*.rs'` and confirm each handles `id == 0` or missing manifest gracefully. Any call site that `unwrap()`s the manifest lookup is a bug to fix.
- No changes to `new_ship` (friendly-ship builder) — its code path is unaffected.

**Patterns to follow:**
- Sentinel with `api_slotitem_id == 0` mirrors `KcApiShip::api_slot = [-1; 5]` convention — out-of-band value signals "empty slot".
- Test codex-load pattern: `Codex::load_without_cache_source("../../.data/codex")` from `emukc_model/src/codex/…` test modules (verify path prefix since this crate's test working dir may differ from `emukc_battle`; use the existing convention in `ship.rs` tests if present).

**Test scenarios:**
- Covers R4. Happy path — dense slots: abyssal ship with all 5 slots populated (`basic.slots = [id1, id2, id3, id4, id5]`, all `item_id > 0`) → `slot_items.len() == 5`, `api_slot` values match input, `api_onslot` matches `basic.maxeq`. (Already the behavior today; pin it.)
- Covers R4. Edge case — sparse slots: abyssal ship with `basic.slots = [{item_id: 100}, {item_id: 0}, {item_id: 200}, {item_id: 0}, {item_id: 300}]` → `slot_items.len() == 5`, `slot_items[1].api_slotitem_id == 0`, `slot_items[3].api_slotitem_id == 0`, `slot_items[0/2/4]` carry the real item ids, and `api_onslot` has zeros at positions 1 and 3.
- Covers R4. Edge case — trailing empty slots: `basic.slots = [{item_id: 100}, {item_id: 200}, {item_id: 0}, {item_id: 0}, {item_id: 0}]` → positions 2-4 are sentinels.
- Covers R4. Edge case — leading empty slot: `basic.slots = [{item_id: 0}, {item_id: 100}]` → `slot_items[0]` is sentinel, `slot_items[1].api_slotitem_id == 100`. Confirms the zip index starts at 0 and the first real plane is at slot position 1 (not 0).
- Covers R4. Edge case — slot item missing from manifest: synthesize a `basic.slots` entry with `item_id` that does not exist in `codex.manifest.api_mst_slotitem`. Expectation: sentinel is pushed at that index (not skipped), `api_onslot[idx] = 0`, existing `warn!` log still fires.
- Integration (if fixture available): construct an abyssal CV with sparse slots containing a 艦攻 at slot index 2 and empty at index 0; run `kouku::has_any_air_combat_planes` → must return `true`. Without the fix, current code sees `slot_items[0] = 艦攻, api_onslot[0] = 0` (wrong positional join) and returns `false`.
- Integration: same abyssal CV fleet runs `simulate_day`; `simulation.packet.kouku.is_some()` and `packet.stage_flag == [1, 1, 1]`.

**Verification:**
- New unit tests pass.
- Existing `new_enemy_ship` callers (e.g., `build_sortie_enemy_ship_uses_new_enemy_ship_for_abyssal_id` at `crates/emukc_gameplay/src/game/sortie/enemy_ship.rs:483` region) still pass — sentinel should be transparent to existing assertions.
- `cargo test -p emukc_model` and `cargo test -p emukc_battle` pass.

---

## System-Wide Impact

- **Interaction graph:**
  - U1 touches `BattleState` construction + `simulate_day` phase dispatch. `simulate_night` / `finalize_night` already build `BattleState` via `for_night` — set `has_bb_class_at_start = false` there since night battles do not run day Shelling2.
  - U3 touches every `slot_items.iter()` call site across the battle crate and any other downstream consumer. The main known consumers are in `crates/emukc_battle/src/simulation/kouku.rs` (6 sites). Must audit the full crate before landing.
- **Error propagation:** Fixes are branch-tightening, not new error paths. No new `Result` surfaces.
- **State lifecycle risks:**
  - U1: the snapshot-at-start invariant must survive `BattleState` copies/moves. Setting the flag after `from_context` but before the phase loop in `simulate_day` is the only write site. `execute_shelling2` is read-only on it.
  - U3: `slot_items.len()` change (from ≤ 5 to exactly `api_slotnum`, capped at 5) may affect any serializer that writes `slot_items` verbatim. `api_slotitem_id == 0` should serialize as "empty" in KC2 convention; verify no serializer emits position-0 sentinel as a real equipment entry.
- **API surface parity:** `new_enemy_ship` is `pub` on `Codex`. Any external consumer of `slot_items` length must tolerate the new invariant. Known callers: `crates/emukc_gameplay/src/game/sortie/enemy_ship.rs:46` (also iterates `slot_items.iter().take(5).enumerate()` which handles the new shape correctly) and the battle crate consumers listed above.
- **Integration coverage:** U3 has the highest integration risk — existing unit tests using `sample_ship` (friendly-ship path) will not exercise the sparse-slot abyssal path. An end-to-end kouku test with a sparse-slot abyssal CV is the only way to prove the fix; see U3 test scenarios.
- **Unchanged invariants:**
  - `api_slot[idx] == -1` for empty slots remains the canonical KC2 "no equipment" marker at the `KcApiShip` level.
  - `can_opening_torpedo_ship` damage-agnostic behavior is preserved (R3 regression guard).
  - `BattleFlow::SURFACE_DAY.phases` phase list is unchanged — gating is runtime.
  - `new_ship` (friendly-ship builder) is unchanged.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| U3 sentinel `api_slotitem_id == 0` is not handled gracefully by some `slot_items.iter()` consumer and panics via `unwrap()` on manifest lookup | Audit all call sites with `rg 'slot_items\.iter\(\)' crates/ --include='*.rs'` before landing; replace any `unwrap()` with `ok()` filtering; add a kouku integration test with sparse-slot fixture. |
| U3 sentinel affects a serializer that emits `slot_items` to a client (e.g., battle packet or save state), producing garbage `api_slotitem_id: 0` on the wire | Confirm `BattleShipInput.slot_items` is runtime-only (not serialized to client). `KcApiShip.api_slot` is what the client sees for equipment IDs. |
| U1 BB-gate via `from_context` accidentally threads `&Codex` through `BattleState` construction, creating a larger API change than needed | Compute the flag outside `from_context` in `simulate_day`; add a setter method on `BattleState`. Same pattern as existing `stage_flag`. |
| U3 test requires a real `.data/codex` codex with a known sparse-slot abyssal CV fixture, which may not exist in test data | If no natural fixture exists, synthesize a test-only `EnemyShipExtra` by direct struct construction in a unit test colocated with `ship.rs`, bypassing codex load. |
| Integer-overflow edge case in U2: `hp() * 2` for `hp = i64::MAX / 2` is non-representative for battle HP ranges (real HP is < 10,000) | Ignore. Real `api_nowhp` is bounded by `api_maxhp` which is bounded by a few thousand; overflow is not reachable in the battle domain. |

---

## Sources & References

- **Origin:** Debug summary from user (no upstream requirements doc).
- Related code:
  - `crates/emukc_battle/src/config.rs` (SURFACE_DAY phase list)
  - `crates/emukc_battle/src/simulation/mod.rs` (`simulate_day`, `execute_shelling2`)
  - `crates/emukc_battle/src/state.rs` (`BattleState::from_context`)
  - `crates/emukc_battle/src/targeting.rs` (`can_closing_torpedo_ship`, `ship_type`)
  - `crates/emukc_model/src/codex/ship.rs` (`new_enemy_ship`)
  - `crates/emukc_model/src/kc2/types/ship.rs` (`KcShipType` enum)
  - `crates/emukc_battle/src/simulation/kouku.rs` (zip consumers)
- Related plans:
  - `docs/plans/2026-05-10-002-fix-fallback-enemy-and-kouku-damage-display-plan.md` (prior work on enemy-ship build fallback; increased relevance of U3)
- External references: KanColle battle rule sources (wikiwiki 戦闘について, 神ゲー, アットウィキ, ぜかましねっと) — user-supplied; not re-fetched.
