---
title: "refactor: harden debug-overlay bridge; defer owned-pass rewrite"
status: planned
created: 2026-06-24
type: refactor
origin: ce-brainstorm re-evaluation of owned-pass battle rewrite (docs/plans/archive/2026-06-22-010-refactor-event-sourced-battle-plan.md) — decision: keep bridge, harden weak points
---

# Refactor: Harden the Debug-Overlay Bridge (Defer Owned-Pass Rewrite)

## Decision Context

A `/ce-brainstorm` session re-evaluated the deferred owned-pass / event-sourced battle rewrite
(plan 010, units U2/U5/U6). Findings:

- The original motivation for owned-pass — debug features (`god_mode`, `one_hit_kill`) being hard to
  embed under `&mut` — is **already satisfied** by the shipped bridge (`debug_overlay` + `transforms` +
  `reducer`).
- The two blockers that killed *pure* event-sourcing (intra-phase HP dependencies; interleaved RNG)
  did **not** kill owned-pass; owned-pass was deferred for **cost/scope**, not feasibility.
- There is **no felt pain** today — the rewrite would be justified mainly by aesthetics.

**Decision:** keep the bridge; do **not** undertake the ~11k-line / 384-`&mut` / 202-test owned-pass
rewrite for cleanliness alone. Instead, pay down the bridge's two real, verified weak points and record
the no-go so plan 010 is not re-litigated without a real driver.

The bridge already carries ~29 unit tests across `debug_overlay.rs` / `transforms.rs` / `reducer.rs`, and
the documented learnings in `docs/solutions/architecture-patterns/debug-overlay-bridge.md` (Sunk filter,
`can_midnight` recompute) shipped with coverage. So this hardening is **narrow** by design — two code/test
gaps plus a decision record, not a broad test-writing exercise.

---

## Problem Frame

Two verified weak points remain in the bridge:

1. **Ordering invariant is convention-only.** In `apply_day_debug` (`crates/emukc_battle/src/debug_overlay.rs:125-129`)
   and `apply_night_debug` (L146-148), `rebuild_*_packet_arrays` reads the **real** pre-override HP from
   `sim.friendly` / `sim.enemy`, and the immediately following `override_ships` zeroes that HP. Correctness
   depends solely on "rebuild runs before override." A reorder silently skips the `one_hit_kill` finishing-volley
   synthesis (all enemies already show HP=0), with no compile-time or test guard unless one is added.
2. **`one_hit_kill` synthetic `hougeki3` is not client-validated** (bridge doc learning #6, medium risk). Data
   correctness (`enemy_nowhps==0`) is guaranteed by the HP override, but the synthesized finishing-volley
   packet *shape* has never been checked against a real KC client render.

---

## Requirements

- R1. Reordering `rebuild_*_packet_arrays` relative to `override_ships` must not be able to silently break
  the finishing-volley synthesis — it must fail a test (and ideally a `debug_assert`).
- R2. The `one_hit_kill` synthetic `hougeki3` field shape must be verified against real client battle
  captures; discrepancies fixed, or the medium-risk item closed with evidence.
- R3. The owned-pass no-go decision is recorded durably so plan 010 is not reopened without a real driver.
- R4. The non-debug battle path is unchanged — golden transcript stays frozen.

---

## Key Technical Decisions

- **KTD1 — Remove the ordering dependency, don't just document it.** Preferred: snapshot the real HP that
  `rebuild_*` needs *before* any override (into `DerivedState` or a local), so `rebuild_*` no longer reads
  the same ships `override_ships` mutates. Fallback if that refactor is too invasive: a `debug_assert!` in
  the `one_hit_kill` rebuild branch that enemy HP is not already all-zero, plus a regression test. Either
  way R1 is met by a test that fails on reorder.
- **KTD2 — Use the existing official captures as the `hougeki3` shape oracle.** `~/Downloads/kcsapi/battle{,(2),(3)}.txt`
  contain real day-battle `api_hougeki3` payloads. Validate shape (field presence, array alignment,
  `api_at_eflag` direction, si_list type per plan 2026-06-24-001), not numeric values.
- **KTD3 — Keep owned-pass deferred.** Revisit only on a real driver: a feature that needs authoritative
  phase events, or the bridge actually causing bugs.

---

## Implementation Units

### U1. Structurally lock the rebuild-before-override ordering

**Goal:** Make `rebuild_*_packet_arrays` independent of its call order relative to `override_ships`, so a reorder cannot silently drop the `one_hit_kill` finishing-volley synthesis.

**Requirements:** R1, R4

**Dependencies:** none

**Files:**
- `crates/emukc_battle/src/debug_overlay.rs` (`apply_day_debug`, `apply_night_debug`, `rebuild_day_packet_arrays`, `rebuild_night_packet_arrays`, `override_ships`)

**Approach:** Implement KTD1. Preferred: capture the pre-override real HP snapshot before any override and
feed it to `rebuild_*`. Fallback: `debug_assert!` guard in the `one_hit_kill` branch. Add a regression test
either way. No change to the non-debug return-early path (both flags false → `sim` returned unchanged).

**Patterns to follow:** the existing bridge test module in `debug_overlay.rs` (the `#[cfg(test)]` block, tests around the one_hit_kill / can_midnight cases).

**Test scenarios:**
- Happy path: `one_hit_kill` day battle with still-alive enemies → `hougeki3` synthesizes a finishing volley dealing exactly each alive enemy's remaining HP.
- Regression (R1): a test that fails if `rebuild_*` is moved after `override_ships` — e.g. assert the finishing volley exists / damages sum to entry HP; with the fallback, assert the `debug_assert` path panics when fed already-zeroed enemies (debug build).
- Night path: `apply_night_debug` with `one_hit_kill` → night hougeki tail synthesis unaffected by the change.
- god_mode unaffected: friendly-directed damage still zeroed; no finishing volley synthesized.

**Verification:** `cargo test -p emukc_battle` green; deliberately reordering the two calls makes the new regression test fail.

### U2. Validate the one_hit_kill synthetic hougeki3 against real captures

**Goal:** Close bridge-doc risk #6 — confirm the synthesized finishing-volley packet shape matches a real client render, or fix the shape.

**Requirements:** R2

**Dependencies:** U1 (validate the final shape after any U1 refactor)

**Files:**
- `crates/emukc_battle/src/debug_overlay.rs` (only if a shape discrepancy is found in `rebuild_day_packet_arrays`)
- test added near the `debug_overlay.rs` test module (or the battle test layer)

**Approach:** Use `~/Downloads/kcsapi/battle{,(2),(3)}.txt` `api_hougeki3` as the shape oracle (KTD2). Compare
the synthetic volley's field shapes against the real payload. Consistent → record the result and downgrade/close
#6; inconsistent → fix the shape, then assert.

**Test scenarios:**
- Synthetic `hougeki3` arrays (`api_at_eflag`, `api_df_list`, `api_damage`, `api_cl_list`, `api_at_list`) are length-aligned and same-shaped as the captured real payload (shape assertions, not numeric).
- `api_at_eflag` entries for the finishing volley mark friendly→enemy direction.
- `api_si_list` entries for the synthetic volley honor the JSON-string-vs-int rule (see plan 2026-06-24-001).

**Verification:** `cargo test -p emukc_battle` green; PR notes which captures were compared and the #6 risk conclusion.

### U3. Record the owned-pass no-go decision

**Goal:** Durably capture the re-evaluation so plan 010 is not reopened without a real driver.

**Requirements:** R3

**Dependencies:** none

**Files:**
- `docs/solutions/architecture-patterns/debug-overlay-bridge.md` (append a "2026-06-24 re-evaluation" section)
- `docs/plans/archive/2026-06-22-010-refactor-event-sourced-battle-plan.md` (one-line status note pointing to this decision)

**Approach:** Record: owned-pass re-evaluated 2026-06-24; no pain beyond aesthetics → keep the bridge, do not
rewrite; restart condition = a feature needing authoritative events, or the bridge causing real bugs. Link back.

**Test scenarios:** Test expectation: none — documentation.

**Verification:** doc renders; frontmatter matches the `docs/solutions/architecture-patterns/` convention; links resolve.

---

## Scope Boundaries

### In Scope
- Ordering-safety of the bridge's packet-array rebuild (U1).
- Client-shape validation of the `one_hit_kill` finishing volley (U2).
- Recording the no-go decision (U3).

### Out of Scope
- The non-debug simulation path (`&mut BattleState`) — untouched; golden transcript frozen.
- Numeric battle outcomes — only debug-overlay shape/ordering is in scope.

### Deferred to Follow-Up Work
- **Owned-pass / event-sourced rewrite (plan 010 U2/U5/U6)** — stays deferred; revisit only on a real driver (KTD3).
- **HP-diff → event reconstruction consistency `debug_assert`** — existing ~29 tests cover enough; add a reconciliation assert only if U1/U2 surface a mismatch.
- **Re-introducing the rich event vocabulary** (`Targeted`, `PhaseStart`, `AirCombat`, …) — belongs to the owned-pass rewrite, deferred.

---

## Risks & Dependencies

- **R-Risk1 — U1 refactor scope creep.** The snapshot approach touches `rebuild_*` signatures. Mitigation:
  fallback `debug_assert`+test path is available if the refactor grows; either satisfies R1.
- **R-Risk2 — captures may not cover the exact CI/finishing-volley case.** The three `battle*.txt` are day
  battles; if none exercises a comparable `hougeki3` tail, U2 validates shape at the field level and notes the
  residual gap honestly (request a targeted capture) rather than claiming full validation.
- Internal to `emukc_battle` plus two docs; no external dependencies.

---

## Verification

```bash
cargo test -p emukc_battle              # U1/U2 regressions
cargo test                              # golden transcript unchanged (R4)
cargo fmt --all --check
cargo clippy --workspace -- -W warnings
```

Optional end-to-end: run a sortie/practice with `one_hit_kill` enabled and confirm the client renders the
finishing volley without artifacts (the U2 shape check is the cheaper proxy).
