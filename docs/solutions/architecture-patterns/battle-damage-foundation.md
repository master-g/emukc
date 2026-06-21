---
title: "Battle damage foundation: phase damage reports effective post-protection damage"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing or modifying battle phase damage reporting across any battle type"
  - "Verifying client HP animation consistency with server HP state"
tags: [battle, damage, sinking-protection, hougeki, torpedo, kouku, oasw, api-damage]
related_components: [emukc_gameplay]
---

# Battle damage foundation: phase damage reports effective post-protection damage

## Context

Battle phase output arrays drive the client's sequential HP animation. The
client computes display HP as *initial HP minus cumulative per-phase damage*.
If any phase reports the raw pre-protection damage instead of the actually-
applied damage, the client's HP bar drifts out of sync with the server's true
HP state. This spec codifies the invariant that every damage array reports the
**effective** value returned by `apply_damage()`.

## Guidance

The following invariants hold for all battle phase output:

- **Effective damage, not raw.** Every phase output array — hougeki
  `api_damage`, torpedo `api_fydam`/`api_eydam`/`api_fdam`/`api_edam`, kouku
  `api_fdam`/`api_edam`, OASW `api_damage`, and night battle `api_damage` —
  SHALL contain the effective damage returned by `apply_damage()`: the actual
  HP subtracted *after* sinking protection, not the raw calculated damage.
- **Sinking-protection proportional damage is reported.** When sinking
  protection triggers for a protected ship (flagship, or a non-taiha-at-entry
  friendly ship during sortie), the reported damage SHALL equal the
  proportional damage actually applied, which is strictly less than the raw
  damage. Example: a flagship at 50 HP taking 80 raw shelling damage is
  protected to 25 applied damage, and `api_damage` reports 25 (not 80).
- **Torpedo phase reports reduced effective damage.** A non-taiha friendly
  ship at sortie entry that survives lethal opening-torpedo damage via
  sinking protection SHALL have `api_fydam`/`api_eydam` report the reduced
  effective value.
- **Kouku airstrike reports effective damage.** When an airstrike would deal
  lethal damage to a protected friendly ship, `api_fdam` for that position
  SHALL report the post-protection effective damage.
- **Below-lethal damage is unchanged.** When raw damage is less than the
  target's current HP, effective damage equals raw damage and reported damage
  equals raw damage.
- **Enemy sortie damage: overkill preserved.** An enemy ship taking damage in
  a sortie battle has effective damage equal to raw damage (NOT clamped to
  current HP); HP MAY go negative to display overkill.
  *Note:* this enemy-overkill requirement is MODIFIED by the
  `fix-battle-attack-system` change — currently capped to current HP pending
  that change.
- **Enemy practice damage: clamped.** An enemy ship taking damage in a
  practice battle has effective damage equal to raw damage clamped to current
  HP, and reported damage equals the clamped value.

## Why This Matters

The client reconstructs HP purely from the cumulative per-phase damage arrays.
If a protected ship's phase reports the pre-protection lethal value, the
client animates the ship to a HP that the server never reached, causing a
visual desync that persists for the rest of the battle. Reporting effective
damage keeps client and server HP identical at every phase boundary.

## When to Apply

- When implementing a new battle phase or output array.
- When modifying `apply_damage()` or sinking-protection logic.
- When reviewing any change to `api_damage`/`api_fdam`/`api_edam` population.

## Examples

Flagship sinking-protection scenario:

```
// flagship HP 50, raw shelling damage 80
let applied = apply_damage(&mut ship, 80, protection_on); // => 25 (proportional)
hougeki.api_damage[attack_index] = applied;                // reports 25, NOT 80
```

## Related

- `docs/solutions/architecture-patterns/night-battle-sinking-protection.md` — the sinking-protection policy that produces the proportional values reported here.
- `crates/emukc_battle/` — `apply_damage()` and phase output population.
