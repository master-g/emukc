---
title: "Battle kouku Stage3: per-attacker target index arrays"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing or modifying the kouku (aerial combat) Stage3 output struct"
  - "Mapping bomber-slot attacks to target positions"
tags: [battle, kouku, stage3, aerial-combat, api-frai, api-erai, api-fbak, api-ebak]
related_components: [emukc_gameplay]
---

# Battle kouku Stage3: per-attacker target index arrays

## Context

`BattleKoukuStage3` is the Phase 3 (damage) output of the aerial-combat
stage. Its fields describe who attacked whom, and how much damage accumulated
per defender. The indexing convention for the target fields (`api_frai`,
`api_erai`, `api_fbak`, `api_ebak`) must be per-attacker, not per-defender —
this is the shape the KanColle client expects.

## Guidance

The following invariants hold for `BattleKoukuStage3`:

- **`api_frai`/`api_erai` are per-attacker target-index arrays.** Each entry
  corresponds to one friendly/enemy attacker (a bomber slot). The value is the
  target *position* on the opposing side, or `-1` if that attacker did not
  attack.
- **`api_fbak`/`api_ebak` are per-attacker target-index arrays.** Same shape
  as the raid arrays: one entry per attacker, value is the hit target
  position or `-1` for no attack.
- **`api_fdam`/`api_edam` remain per-defender cumulative damage.** These are
  indexed by defender position and hold the *accumulated* damage across all
  attackers targeting that defender; they are NOT per-attacker.
- **Bomber-slot mapping is exact.** Each bomber slot's attacker ship index and
  its target defender index MUST map correctly to the entries above — a
  mismatch between the attacker slot and the reported target position corrupts
  the client's per-plane hit/miss animation.

## Why This Matters

The client animates each bomber slot's attack independently using the
per-attacker arrays. If `api_frai` is mis-indexed as per-defender, the client
cannot tell which plane attacked which target, and the hit/miss overlay
detaches from the actual sortie. The per-defender `api_fdam` must stay
per-defender because the client sums it for the HP bar.

## When to Apply

- When adding or modifying fields on `BattleKoukuStage3`.
- When wiring bomber-slot iteration into the Stage3 output.
- When reviewing aerial-combat output for client compatibility.

## Examples

Conceptual shape (friendly attackers hitting enemy defenders):

```
// 3 friendly bomber slots: slot0 -> enemy pos 1, slot1 -> no target, slot2 -> enemy pos 0
api_frai = [1, -1, 0]   // per-attacker (length == attacker count)
api_fdam = [35, 0, 12]  // per-defender (length == defender count), cumulative
```

## Related

- `crates/emukc_battle/src/simulation/kouku.rs` — Stage3 struct population.
- `docs/solutions/architecture-patterns/battle-damage-foundation.md` — how `api_fdam` effective-damage reporting interacts with this per-defender array.
