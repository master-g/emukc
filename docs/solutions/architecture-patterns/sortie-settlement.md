---
title: "Sortie battle settlement: one write set, one transaction, one snapshot out"
date: 2026-09-19
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Adding or reordering a write that happens when a sortie battle ends"
  - "Adding a sortie path that must produce a battle result response"
  - "Writing a test that needs the full post-battle write set, not a handler"
tags: [sortie, settlement, transaction, write-set, rng, snapshot]
related_components: [emukc_model, emukc_battle]
---

# Sortie battle settlement: one write set, one transaction, one snapshot out

## Context

What happens at the end of a sortie battle used to be spread across
`Ctx::sortie_battle_result` and `sortie_result.rs`: the profile and ship stat
update, the map record, the ship drop roll, quest progress and the dependency
unlock were sequenced by the caller, and `SortieBattleResultResponse` was built
as a struct literal in more than one place. The mid-map gauge-clear path went
through a partly different sequence, so the two paths could disagree about the
order of writes — and the drop roll is the only RNG consumer in the set, so a
reorder is not merely cosmetic.

## Guidance

### One `_impl` owns the write set

```rust
pub(super) async fn settle_sortie_battle_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    definition: &MapDefinition,
    active: &ActiveSortieState,
    snapshot: SortieBattleResultSnapshot,
    final_enemy_nowhps: &[i64],
) -> Result<SortieSettlement, GameplayError>
```

The order inside is fixed: `update_sortie_result_stats` (which contains the
ship experience settlement) → `apply_sortie_map_result` → `try_grant_sortie_ship_drop`
→ collect outcomes → `check_and_unlock_dependencies_impl`. New post-battle
writes SHALL be added inside this function, not at its call site.

`definition` is looked up by the caller and passed in. That is what lets a test
drive a synthetic map through exactly the same settlement as a real sortie.

### The return value is a snapshot, not a response

`SortieSettlement` carries `cell_no`, the settled snapshot, `first_clear`,
`ship_drop`, `next_map_ids`, `dests`, `destsf` and the quest `outcomes`.
`SortieBattleResultResponse` is produced only by `From<SortieSettlement>`; there
SHALL be no second literal of that struct.

### What the `Ctx` method keeps

`Ctx::sortie_battle_result` is reduced to store bookkeeping: take the pending
result and session from `SortieStore`, `begin()`, settle, `observe` the
outcomes, `commit()`, then under the profile lock refresh the stage identity and
decide whether the sortie continues, and finally `settlement.into()`. Anything
that touches the database belongs on the other side of that boundary.

### Two enemy HP vectors, on purpose

`final_enemy_nowhps` is the session packet *after* any night battle and feeds
`api_dests` / `api_destsf`. The snapshot's own `enemy_nowhps` is frozen at the
day battle and keeps feeding the enemy-sunk quest events. They are different
numbers by design; do not collapse them.

### The mirror image: pre-battle setup

The same shape applies before the battle. `sortie/setup.rs::resolve_sortie_battle_setup_impl`
owns the active-state check, the profile read, the fleet construction and *all*
the guards, and `SortieBattleSetup::battle_input` / `result_snapshot` carry the
shared "execute" and "snapshot" steps. Day-start and night-start differ only in
which simulation they call, which is how night-start picked up the
`combined_type`, cell-existence and `event_kind` guards it was missing.

## Rationale

- The drop roll is the only RNG consumer; pinning the sequence in one function
  is what makes a seeded sortie reproducible across paths.
- A single transaction owner means a failure anywhere in the set rolls back all
  of it, including the quest progress observed just before the commit.
- Tests can call the `_impl` directly with a synthetic `MapDefinition` and
  assert on `SortieSettlement`, so the gauge-clear cases no longer need a
  handler or a hand-built response literal.

## Anti-patterns

- Adding a post-battle write in `Ctx::sortie_battle_result` "just for this one
  case". The next path will not have it.
- Building `SortieBattleResultResponse` by hand. Extend `SortieSettlement` and
  the `From` impl.
- Calling `quest` from inside the settlement. It returns `outcomes`; see
  [`quest-observation.md`](quest-observation.md).

## Applies when

- Any new end-of-battle side effect.
- Any new sortie entry point that ends in a battle result.

## See also

- `docs/plans/2026-09-18-002-refactor-deepen-shallow-modules-plan.md` — KTD2,
  KTD3, AE5, AE6; units U5 and U6.
- `ship-exp-settlement.md`, `quest-observation.md`.
