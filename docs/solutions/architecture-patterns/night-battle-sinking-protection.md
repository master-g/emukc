---
title: "Night battle sinking protection (轟沈ストッパー) and sortie context"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Modifying simulate_night or the NightBattleInput contract"
  - "Adjusting sortie vs practice sinking-protection behavior"
  - "Wiring the orchestrate layer's is_sortie flag"
tags: [night-battle, sinking-protection, sortie, practice, nightbattleinput, simulate-night]
related_components: [emukc_gameplay]
---

# Night battle sinking protection (轟沈ストッパー) and sortie context

## Context

Night battle sinking protection prevents friendly ships from being sunk under
specific conditions during sorties. The behavior is controlled by
`NightBattleInput.is_sortie`, distinguishing sortie (protected) from practice
(unprotected) battles. Migrated from
`openspec/specs/night-battle-sinking-protection/spec.md`.

## Guidance

### Night battle sinking protection applies during sorties

When `simulate_night` is called for a sortie battle, sinking protection
(轟沈ストッパー) SHALL apply to friendly ships exactly as during day battles.
`NightBattleInput.is_sortie` controls this behavior.

- Non-taiha friendly ship survives lethal damage in sortie night battle: when
  `is_sortie == true` and a friendly ship that was NOT in taiha (HP > 25% max)
  at entry receives lethal damage, the ship SHALL survive with HP ≥ 1; the
  damage applied SHALL be proportional:
  `floor(0.5 × entry_hp + 0.3 × rand(0..entry_hp))`.
- Flagship always survives in sortie night battle: when `is_sortie == true` and
  the flagship (index 0) receives lethal damage at any HP state, the flagship
  SHALL survive with HP ≥ 1.
- Taiha non-flagship can be sunk in sortie night battle: when `is_sortie ==
  true` and a non-flagship friendly ship that WAS in taiha (HP ≤ 25% max) at
  entry receives lethal damage, the ship MAY be sunk (HP = 0).
- Practice night battle has no sinking protection: when `is_sortie == false`
  and any friendly ship receives lethal damage, the ship SHALL be sunk
  regardless of HP state or flagship status.
- Enemy ships never receive sinking protection: regardless of `is_sortie`, an
  enemy ship receiving lethal damage SHALL be sunk (HP = 0).

### NightBattleInput carries sortie context

The `NightBattleInput` struct SHALL include an `is_sortie: bool` field
indicating whether this night battle occurs during a sortie (true) or practice
(false). Callers SHALL supply this field explicitly.

- The `emukc_gameplay` orchestrate layer SHALL set `is_sortie = true` when
  calling `simulate_night` for a sortie battle.
- The `emukc_gameplay` orchestrate layer SHALL set `is_sortie = false` when
  calling `simulate_night` for a practice battle.

## Why This Matters

Sinking protection is a core PvE fairness rule: a ship that enters night
battle in good shape cannot be instantly sunk, and the flagship is always
protected. Practice battles intentionally disable it so practice can sink
ships. The `is_sortie` flag is the single switch that prevents practice logic
from leaking into sorties (and vice versa).

## When to Apply

- When modifying `simulate_night` or the `NightBattleInput` struct.
- When wiring the orchestrate layer's call to `simulate_night`.

## Examples

- Sortie night battle, non-taiha ship takes lethal damage → survives at HP ≥ 1
  with proportional damage.
- Sortie night battle, flagship at any HP takes lethal damage → survives.
- Practice night battle, any ship takes lethal damage → sunk.
- Enemy ship in any battle → sunk on lethal damage (never protected).

## Related

- `docs/solutions/architecture-patterns/sortie.md` — sortie battle sequencing
  whose damage must report effective (post-protection) values.
- `docs/solutions/architecture-patterns/equipment-improvement-bonus.md` —
  night battle basic power these protections apply to.
