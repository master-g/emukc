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

### 3. Packet array rebuild ordering

`rebuild_*_packet_arrays` must run BEFORE `override_ships`. The finishing
volley synthesis needs to see real simulation HP (alive enemies) to
calculate remaining HP. If `override_ships` runs first, all enemies already
have HP=0 and the synthesis is skipped.

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

### 6. one_hit_kill synthetic hougegi3 needs client validation

The synthetic `hougeki3` shape has not been validated against a real KC
client render. The data correctness (`enemy_nowhps==0`) is guaranteed by
the HP override regardless. The array rebuild is for animation fidelity
only and is a medium-risk item for client compatibility.

## Related

- `docs/plans/2026-06-22-010-refactor-event-sourced-battle-plan.md` — origin
- `docs/solutions/architecture-patterns/battle-damage-foundation.md` —
  client HP reconstruction invariant
- `crates/emukc_battle/src/debug_overlay.rs` — implementation
- `crates/emukc_battle/src/transforms.rs` — event transforms
- `crates/emukc_battle/src/reducer.rs` — pure state derivation
