---
title: "Ship experience settlement: one pure function, three persistence carriers"
date: 2026-09-19
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: domain_rule
severity: high
applies_when:
  - "Granting ship experience from a new source (sortie, practice, expedition, an event reward)"
  - "Changing the level cap or the exp-to-level mapping"
  - "Deciding whether a settlement helper should write to the database"
tags: [ship, experience, level-cap, marriage, settlement, pure-function]
related_components: [emukc_model]
---

# Ship experience settlement: one pure function, three persistence carriers

## Context

Sortie, practice and expedition each inlined the same four steps — add the gain,
map exp to a level, clamp to the level cap, recompute the next-level threshold
and the progress bar. Three copies meant three chances to get the cap wrong, and
one of them did: `expedition.rs` clamped the *level* but wrote the raw
accumulated experience, so an unmarried ship crossing Lv.99 on an expedition
came back with `exp_next` pointing at Lv.100 and a non-zero progress bar, while
the same ship crossing Lv.99 in a sortie was pinned. The divergence was
invisible until someone compared the two responses.

## Guidance

### The rule lives in `game/ship/exp.rs`

```rust
pub(crate) fn settle_ship_exp(exp_now: i64, gain: i64, married: bool) -> ShipExpSettlement
```

`ShipExpSettlement` carries `{ level, exp_now, exp_next, progress }`. At the cap
(99 unmarried, 175 married) `exp_now` is pinned to the cap requirement and both
`exp_next` and `progress` are zero. Every caller SHALL pass the *gain* and take
all four fields from the returned struct. A caller MUST NOT re-derive a level,
re-clamp, or keep its own copy of the cap.

`calculate_admiral_exp` and `build_exp_lvup_vector` live in the same file for
the same reason: they were byte-identical in two modules.

### Settlement does not persist

`settle_ship_exp` takes no connection and writes nothing. That is deliberate,
not an oversight: the three callers persist through different carriers. Sortie
and practice fold the result into a `KcApiShip` that is written once by
`update_ship_impl` together with fuel and ammo; expedition writes a
`ship::Model` through `recalculate_ship_status_with_model`. A settlement
function that wrote for itself would make the same ship take two writes in one
transaction.

### What stays per-domain

How much experience a ship earns is *not* shared. `calculate_sortie_ship_exp`
and practice's `calculate_ship_exp` differ in real ways — practice applies
`practice_exp_boost`, sortie gates sunk ships out — and they take different
input types. Only the settlement of a gain is common.

## Rationale

- The cap is a single number applied in a single place, so "unmarried ships stop
  at 99" is one assertion in one test module rather than three that can drift.
- Keeping the function pure makes the cap testable without a database: the
  boundary cases (crossing the cap, already at the cap, married at 99 where
  Lv.99 and Lv.100 share the same requirement) are plain `#[test]`s.
- Callers keep ownership of their write, which is what lets each domain batch
  the exp update with the rest of its row.

## Anti-patterns

- Clamping the level without pinning the experience. That is exactly the
  expedition bug; the level looks right and the progress bar lies.
- Adding a `&C: ConnectionTrait` parameter to the settlement function so it can
  "just save it". The carrier differs per caller.
- Copying the cap constant into a new XP source. Call `settle_ship_exp`.

## Applies when

- Any new source of ship experience.
- Any change to `level::ship_level_cap` or `level::ship_level_required_exp`.

## See also

- `docs/plans/2026-09-18-002-refactor-deepen-shallow-modules-plan.md` — KTD1,
  AE4; units U4 (`fix(expedition):` then `refactor(gameplay):`).
- `sortie.md` — *Unmarried ship level cap enforcement*.
