---
title: "Night enemy-attack loop reported uncapped raw damage for friendly defenders"
date: 2026-06-24
category: logic-errors
module: emukc_battle
problem_type: logic_error
component: service_object
symptoms:
  - Friendly ship saved by night sinking protection displays the pre-protection lethal damage, not the applied value
  - Client HP bar drifts below the server's true HP after a night battle and stays desynced until /api_port
  - Desync only on the enemy-attacks-friendly night path; the friendly-attacks-enemy path is correct
root_cause: logic_error
resolution_type: code_fix
severity: high
tags: [battle, night-battle, damage, sinking-protection, display-damage, kabau, api-damage, hp-desync]
related_components: [emukc_gameplay]
---

# Night enemy-attack loop reported uncapped raw damage for friendly defenders

## Problem

The night battle enemy-attack loop in `simulate_night_hougeki`
(`crates/emukc_battle/src/simulation/night.rs`) pushed the **uncapped** `raw_dmg`
into the displayed `api_damage` array for a friendly defender, bypassing
`display_damage`. This violates the
[battle damage foundation](../architecture-patterns/battle-damage-foundation.md)
invariant that every phase array reports the *effective* post-protection damage
returned by `apply_damage`.

The friendly-attack loop in the same function did it correctly:

```rust
let (raw_dmg, dealt) = enemy[target_idx].apply_damage(rng, raw, target_idx);
let display = crate::targeting::display_damage(&enemy[target_idx], raw_dmg, dealt);
hit_damages.push(display);
```

The enemy-attack loop did not:

```rust
let (raw_dmg, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
hit_damages.push(raw_dmg); // BUG: friendly defender, but reports raw not dealt
```

`display_damage(defender, raw, dealt)` returns `dealt` for a friendly defender
(sinking-protection / overkill safe) and `raw` for an enemy defender (overkill
preserved). The enemy-attack loop's defender is always the friendly fleet, so it
must report `dealt`.

## Symptoms

When a non-taiha friendly ship takes a lethal night hit during a sortie, night
sinking protection reduces the applied damage (`dealt`) so the ship survives.
The wire reported the lethal `raw_dmg` instead. The KanColle client reconstructs
HP as *initial − cumulative per-phase damage*, so it subtracted more HP than the
server actually removed — the ship's HP bar drifted below the server's true HP
and stayed wrong for the rest of the battle (battle packets carry no final HP;
reconciliation happens only at `/api_port`). Below-lethal hits were unaffected
(`raw == dealt`), so the desync only appeared when protection or overkill fired.

## Root Cause

`apply_damage` returns `(raw_damage, dealt)` where `dealt` is exactly the HP
subtracted; `display_damage` is the single chokepoint that picks `dealt` vs `raw`
by `defender.is_friendly`. Every other phase (day shelling, kouku, the night
friendly-attack loop, special attack) routes its defender through
`display_damage`; the night enemy-attack loop was the **lone site** that pushed
`raw_dmg` directly. Pre-existing since `baa3f77` (2026-05-23).

## Fix

`crates/emukc_battle/src/simulation/night.rs` — route the enemy-attack loop's
defender through `display_damage`, mirroring the friendly-attack loop (commit
`7dad5da`, plan `docs/plans/2026-06-22-002-fix-battle-attack-system-plan.md`
U4):

```rust
let display = crate::targeting::display_damage(&friendly[target_idx], raw_dmg, dealt);
hit_damages.push(display);
```

No golden re-freeze was needed: the existing 20 night golden transcripts never
drive a friendly defender into the protection/overkill path, so the change
produced zero golden diff (`golden_transcript` passed without re-blessing).
Regression test `sortie_night_enemy_attack_displays_capped_damage` constructs the
protection scenario and asserts the displayed enemy-attack damage
(`api_at_eflag == 1`) equals the friendly ship's actual HP loss.

## How It Was Found / Why It Recurred

Surfaced by the 旗艦援護 (かばう) escort-shield review, not by gameplay symptoms:
the shield redirects enemy night hits onto non-flagship escorts, which drew
attention to the enemy-attack loop and exposed the long-standing `raw_dmg` push.

The display-damage invariant was already documented *and* regression-tested in
`kouku.rs` (`kouku_fdam_uses_display_damage_not_raw_under_protection`), yet a
sibling phase loop reintroduced the same class. The lesson: a per-phase invariant
enforced by a shared helper (`display_damage`) is only as good as every call
site remembering to use it — a new or edited damage-display loop is a
high-recurrence spot.

## Prevention

- When adding or editing any phase loop that populates a damage array, route the
  defender through `display_damage(defender, raw, dealt)` — never push `raw_dmg`
  for a friendly defender.
- When touching damage display, grep every `hit_damages.push` / `damages.push`
  site (`rg "push\(raw" crates/emukc_battle/src/simulation`) and confirm each
  routes through `display_damage`.

## Related

- [Battle damage foundation](../architecture-patterns/battle-damage-foundation.md) — the invariant this bug violated.
- [Night battle sinking protection](../architecture-patterns/night-battle-sinking-protection.md) — produces the proportional `dealt` value that must be displayed.
- `crates/emukc_battle/src/targeting.rs` — `display_damage` (the friendly/enemy chokepoint).
