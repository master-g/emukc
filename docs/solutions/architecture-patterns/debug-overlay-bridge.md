---
module: battle
tags: [debug-overlay, event-transforms, god-mode, one-hit-kill, bridge-pattern]
problem_type: architecture_decision
created: 2026-06-23
---

# Debug Overlay Bridge Pattern

## Context

Plan 010 proposed a full owned-pass battle simulation rewrite (~11k lines,
202 tests) where phase functions emit events and a reducer derives state.
Doc review identified two architectural blockers that killed pure
event-sourcing: intra-phase HP dependencies (targeting needs real-time HP)
and interleaved RNG (sinking protection draws RNG mid-targeting).

## Decision

Instead of the owned-pass rewrite, a **bridge** was built: the simulation
runs normally (`&mut BattleState`), then a post-simulation `debug_overlay`
module derives events from HP diffs, applies transforms, and overrides the
packet/outcome. This delivers the functional goal (debug features as
event transforms, zero simulation branching) without the 11k-line rewrite.

## Key Learnings

### 1. god_mode must filter Sunk{Friendly}, not just Damage

The initial `god_mode_transform` filtered `Damage` and `ProportionalDamage`
for friendlies but not `Sunk`. A friendly that sank during simulation
(practice with `is_sortie=false`, or a sortie non-flagship taiha at entry)
kept its `Sunk` event, forcing HP=0 in the reducer. Fix: add `Sunk{Friendly}`
to the filter.

### 2. can_midnight conjunction rule

After one_hit_kill sinks all enemies, stale `can_midnight=true` /
`midnight_flag=1` would offer a night battle against an empty fleet. The
overlay has no `battle_type`, but `finalize_day` already encoded the
`matches!(Normal | AirBattle)` gate. Fix:

```
new_can_midnight = original_can_midnight && any_alive(friendly) && any_alive(enemy)
```

### 3. Packet array rebuild ordering (resolved 2026-06-24)

Originally `rebuild_*_packet_arrays` had to run BEFORE `override_ships`: the
finishing-volley synthesis read real simulation HP (alive enemies) to compute
remaining HP, so if `override_ships` ran first all enemies already had HP=0 and
the synthesis was silently skipped. This was a convention-only invariant.

**Now removed.** `apply_day_debug` / `apply_night_debug` capture the volley
inputs (attacker index + each alive enemy's remaining HP) into a
`FinishingVolley` snapshot *before* any override, and the synthesis consumes
that snapshot instead of the live ships. The synthesis no longer reads the
ships `override_ships` mutates, so the call order is irrelevant — verified by
reordering the two calls and observing every `debug_overlay` test stays green.

The snapshot was chosen over a `debug_assert!(enemies not all zero)` guard
because that guard would false-positive on a legitimate battle where the real
simulation already sank every enemy (one_hit_kill then correctly synthesizes
nothing).

### 4. Dead event vocabulary was deleted

The rich event vocabulary (`Targeted`, `PhaseStart/End`, `AirCombat`,
`TorpedoSalvo`, `ShellingExchange`) was built for the deferred owned-pass
rewrite. In the bridge, only `Damage`, `Sunk`, and `ProportionalDamage`
are emitted. The unused variants were deleted. They should be re-introduced
by the owned-pass rewrite (origin plan-010 U5/U6) when actually consumed.

### 5. Client animation consistency

The client reconstructs HP from cumulative per-phase damage arrays
(see `battle-damage-foundation.md`). After overriding `nowhps`, the
per-phase arrays must be rebuilt:

- **god_mode**: zero all friendly-directed damage entries
  (`api_fdam`, `api_damage[i]` where `api_at_eflag[i]==1`)
- **one_hit_kill**: synthesize finishing volley in `hougeki3` (day)
  or night hougeki tail, dealing exactly each still-alive enemy's
  remaining HP

### 6. one_hit_kill synthetic hougeki3 shape validation (closed 2026-06-24)

The synthetic `hougeki3` shape was validated against real client day-battle
payloads decoded from `~/Downloads/kcsapi/battle{,(2),(3)}.txt`. `hougeki3` is
the same `BattleHougeki` type as `hougeki1` / `hougeki2`, which the captures do
exercise: all seven arrays are length-aligned, `api_at_eflag` is `0` for
friendly→enemy attacks, `api_at_type` is `0` for normal attacks, and
`api_si_list` uses the integer `-1` sentinel for no-equipment normal attacks
(strings are reserved for cut-in / special attacks, `api_at_type==7`). The
synthetic volley matches all of these. Asserted by
`synthetic_finishing_volley_shape_matches_client` in `debug_overlay.rs`.

Data correctness (`enemy_nowhps==0`) was already guaranteed by the HP override;
this closes the animation-fidelity (shape) gap. **Residual gap:** none of the
three captures contains a non-null `hougeki3` specifically, so the validation is
against `hougeki1/2` of the identical type — a targeted capture of a real
`hougeki3` finishing tail would close it fully. Numeric values are out of scope.

## 2026-06-24 Re-evaluation: owned-pass rewrite stays deferred (no-go)

A `/ce-brainstorm` session re-evaluated the deferred owned-pass / event-sourced
rewrite (plan 010, units U2/U5/U6) to decide whether the bridge should be
replaced. Findings:

- The original motivation for owned-pass — debug features (`god_mode`,
  `one_hit_kill`) being hard to embed under `&mut` — is **already satisfied** by
  the shipped bridge (`debug_overlay` + `transforms` + `reducer`).
- The two blockers that killed *pure* event-sourcing (intra-phase HP
  dependencies; interleaved RNG) did **not** kill owned-pass; owned-pass was
  deferred for **cost/scope** (~11k lines, 384 `&mut`, 202 tests), not
  feasibility.
- There is **no felt pain** today — the rewrite would be justified mainly by
  aesthetics.

**Decision: keep the bridge; do not undertake the owned-pass rewrite for
cleanliness alone.** This hardening pass (2026-06-24) paid down the bridge's two
real weak points instead: the rebuild-before-override ordering is now structural
(Learning #3) and the synthetic `hougeki3` shape is client-validated (Learning
#6).

**Restart condition** — revisit owned-pass only on a real driver:

- a feature that needs authoritative per-phase battle events (e.g. re-introducing
  the `Targeted` / `PhaseStart` / `AirCombat` vocabulary, Learning #4), or
- the bridge actually causing bugs in production.

Absent one of those, plan 010 should not be reopened.

## Related

- `docs/plans/archive/2026-06-24-004-refactor-harden-debug-overlay-bridge-plan.md`
  — this hardening pass + no-go record
- `docs/plans/archive/2026-06-22-010-refactor-event-sourced-battle-plan.md` — origin (deferred)
- `docs/solutions/architecture-patterns/battle-damage-foundation.md` —
  client HP reconstruction invariant
- `crates/emukc_battle/src/debug_overlay.rs` — implementation
- `crates/emukc_battle/src/transforms.rs` — event transforms
- `crates/emukc_battle/src/reducer.rs` — pure state derivation
