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

As of 2026-07-26, the public `execute_day` and `execute_night` facade owns
this sequence. Cross-crate callers provide the `Codex`, battle input, and RNG;
the facade runs the raw simulation, reads `god_mode` and `one_hit_kill` from
`Codex.game_cfg`, applies the bridge, and returns the final result. Raw
`simulate_*` and `apply_*_debug` are crate-internal so callers cannot skip or
reorder the overlay. (The event/reducer/transform support modules named here
until 2026-09-18 are gone; see the section at the end.)

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

- **god_mode**: zero every friendly-directed damage entry the client
  animates from. This is the **per-attacker / per-attack** array, not just the
  `api_fdam` summary: shelling/OASW `api_damage[i]` where `api_at_eflag[i]==1`,
  kouku `api_fdam`, opening torpedo `api_eydam_list_items`, closing torpedo
  `api_eydam`. Keep `api_fdam` zeroed too. (Zeroing only `api_fdam` for the
  torpedo phases left friendlies visibly taiha'd mid-animation — see
  [god-mode-torpedo-damage-not-zeroed](../logic-errors/god-mode-torpedo-damage-not-zeroed-2026-06-25.md).)
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

### 7. Execution facade owns the bridge boundary

The bridge is an implementation detail, not a menu of independently composable
public steps. `execute_day` and `execute_night` are the only cross-crate battle
execution entries. They preserve the raw simulation's RNG stream because the
overlay consumes no RNG. Crate-local equivalence tests compare the facade with
the former manual pipeline for disabled, individual, and combined debug flags;
golden and gameplay tests cover the external behavior.

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

## 2026-09-18 Event round-trip collapsed into a direct HP rule

**Driver:** plan `docs/plans/2026-09-18-002-refactor-deepen-shallow-modules-plan.md`
unit U10, on the user's ruling that the event skeleton has no future consumer —
the owned-pass rewrite is a standing no-go (see the 2026-06-24 re-evaluation
above), so the restart condition that would have justified keeping the
vocabulary alive never fired.

**What the bridge did:** `debug_overlay` derived a `Damage`/`Sunk` event log
from the HP diff, ran it through `god_mode_transform` / `one_hit_kill_transform`,
and reduced it back to a `DerivedState` — of which only `friendly_hp` and
`enemy_hp` were ever read. `friendly_sunk`, `enemy_sunk`, `any_alive` and `hp()`
had no callers, and `event.rs` / `reducer.rs` carried `#[allow(dead_code)]` to
stay compilable.

**What it does now:** `debug_hp` computes those two vectors directly from the
post-simulation ships, per fleet position `i`:

```
friendly[i] = god_mode     ? entry_hp : clamped_hp(ship)
enemy[i]    = one_hit_kill ? 0        : clamped_hp(ship)
clamped_hp  = is_sunk ? 0 : max(entry_hp - max(entry_hp - hp, 0), 0)
```

`clamped_hp` is exactly what the round trip computed for a side no debug flag
covers: the reducer subtracted a single `Damage` event of `max(entry_hp - hp, 0)`
from `entry_hp` with a floor of 0, then a `Sunk` event forced 0. The reducer's
"skip `Damage` on an already-sunk ship" branch never fired, because the derive
step emitted at most one `Damage` per ship and always ordered it before that
ship's `Sunk`.

**Proof before deletion:** a temporary differential test asserted
`debug_hp(...)` equal to `run_debug_transforms(...).{friendly_hp, enemy_hp}`
element-wise over 4 fleet configurations (2v2 lv99 sortie; 6v6 mixed-level
sortie; 6v6 lv1 practice with `is_sortie=false` so friendlies can actually sink;
1v6 sortie where most enemies survive) × seeds `0..1000` × day and night × the
three flag combinations `(true,false)`, `(false,true)`, `(true,true)` — 8000
simulations × 3 combos, all equal, 425 ms in release. The test was deleted with
the pipeline in the follow-up commit; `execution.rs`'s facade-versus-manual
equivalence tests and the gameplay end-to-end god_mode / one_hit_kill tests are
the standing regression net.

**Deleted:** `event.rs`, `reducer.rs`, `transforms.rs`, the two `dead_code`
allows in `lib.rs`, and `derive_events_from_ships` / `initial_state_from_ships` /
`run_debug_transforms` in `debug_overlay.rs`.

**Unaffected:** every learning above survives the change. Learning #1 (god_mode
must also revive a sunk friendly) is now the `god_mode ? entry_hp` branch, which
ignores `is_sunk` by construction. Learnings #2, #3, #5, #6 and #7 live in
`recompute_midnight`, `FinishingVolley`, `rebuild_*_packet_arrays`,
`synthesize_*_finishing_volley` and `execution.rs` — none of which the event
pipeline ever touched. Learning #4 is now moot: the vocabulary it described is
gone entirely, and an owned-pass rewrite would reintroduce it from scratch.

## Related

- `docs/plans/archive/2026-06-24-004-refactor-harden-debug-overlay-bridge-plan.md`
  — this hardening pass + no-go record
- `docs/plans/archive/2026-06-22-010-refactor-event-sourced-battle-plan.md` — origin (deferred)
- `docs/solutions/architecture-patterns/battle-damage-foundation.md` —
  client HP reconstruction invariant
- `crates/emukc_battle/src/execution.rs` — public execution facade and
  raw-versus-executed equivalence tests
- `crates/emukc_battle/src/debug_overlay.rs` — implementation
- `docs/plans/2026-09-18-002-refactor-deepen-shallow-modules-plan.md` — U10,
  the event round-trip collapse
