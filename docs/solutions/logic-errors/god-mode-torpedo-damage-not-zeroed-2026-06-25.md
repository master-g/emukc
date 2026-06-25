---
title: "god_mode left per-attacker enemy torpedo damage in the packet, taiha'ing friendlies mid-animation"
date: 2026-06-25
category: logic-errors
module: emukc_battle
problem_type: logic_error
component: service_object
symptoms:
  - A friendly ship is shown taking damage and going taiha (heavily damaged) during a battle animation even with god_mode on
  - Only happens in battles with an opening or closing torpedo phase; pure shelling / aerial nodes are fine
  - Final HP is correct after /api_port — the desync is animation-only
root_cause: logic_error
resolution_type: code_fix
severity: high
tags: [battle, god-mode, debug-overlay, torpedo, raigeki, opening-torpedo, api-eydam, taiha, hp-desync]
related_components: [emukc_gameplay]
---

# god_mode left per-attacker enemy torpedo damage in the packet

## Problem

`god_mode` ("friendly ships take zero damage") is implemented as a
post-simulation overlay: `rebuild_day_packet_arrays`
(`crates/emukc_battle/src/debug_overlay.rs`) zeroes every friendly-directed
damage entry so the client's *initial HP − cumulative per-phase damage*
reconstruction lands back on full HP (see
[battle-damage-foundation](../architecture-patterns/battle-damage-foundation.md)).

For the two torpedo phases it zeroed **only the `api_fdam` summary**:

```rust
if let Some(raigeki) = &mut packet.raigeki {
    raigeki.api_fdam.fill(0); // BUG: api_eydam left intact
}
```

The client does not animate friendly HP from the `api_fdam` summary — it
animates from the **per-attacker** enemy-torpedo entries (`api_eydam` for
closing torpedo, `api_eydam_list_items` for opening torpedo). Those survived
god_mode, so the client still subtracted torpedo damage from each friendly
during the torpedo phase and could drive it into taiha, even though the
summary and the final overridden HP said full health.

`api_fdam[defender]` and `api_eydam[attacker]` carry the *same* value from two
viewpoints (verified by `torpedo.rs` tests), so zeroing one while leaving the
other is internally inconsistent.

## Root Cause

The shelling phase got this right: `zero_friendly_hougeki_damage` zeroes the
**per-attack** `api_damage[i]` (where `api_at_eflag[i]==1`), which is exactly
what the client animates from. The torpedo phases were the lone inconsistency —
they zeroed an aggregate the client ignores instead of the per-attacker array
it actually consumes. The torpedo phase god_mode path had **zero test coverage**
(every `debug_overlay` test used `raigeki: None` / `opening_attack: None`), so
the gap was invisible.

The institutional knowledge reinforced the blind spot:
[debug-overlay-bridge](../architecture-patterns/debug-overlay-bridge.md)
Learning #5 listed the fields to zero as just `api_fdam` and
`api_damage[i]` — omitting the per-attacker torpedo arrays. That list has been
corrected as part of this fix.

## Fix

`crates/emukc_battle/src/debug_overlay.rs` — also neutralize the per-attacker
enemy torpedo damage, while **preserving** the `api_erai` / `api_ecl` attack
structure (mirroring shelling: the client still plays the hit, for zero damage):

```rust
if let Some(opening) = &mut packet.opening_attack {
    opening.api_fdam.fill(0);
    for cells in opening.api_eydam_list_items.iter_mut().flatten() {
        cells.iter_mut().for_each(|cell| *cell = DamageCell::Plain(0));
    }
}
if let Some(raigeki) = &mut packet.raigeki {
    raigeki.api_fdam.fill(0);
    raigeki.api_eydam.iter_mut().for_each(|cell| *cell = DamageCell::Plain(0));
}
```

Regression test `god_mode_zeros_enemy_torpedo_damage` pins both directions:
the damage arrays (`api_fdam`, `api_eydam`, `api_eydam_list_items`) are zeroed,
and the flags (`api_erai`, `api_ecl`, and their `_list_items`) are preserved.

Night battles are unaffected: night torpedo is merged into `hougeki`
(`api_damage` + `api_at_eflag`), already covered by
`zero_friendly_night_hougeki_damage`.

## Open Uncertainty

It was not possible to *offline*-confirm that the official client animates
torpedo HP from `api_eydam` rather than the `api_fdam` summary — `main.js`
isn't in the repo and the captured battle samples
(`~/Downloads/kcsapi/battle*.txt`) contain no torpedo phase. The conclusion
rests on three independent signals: (1) the user's reproduction (god_mode on,
still taiha'd); (2) `api_eydam` being the *only* friendly-HP field god_mode
left unzeroed across all phases; (3) battle-damage-foundation requiring
`api_eydam` to report the post-protection effective value, which only matters
if the client reconstructs HP from it. Even if the client used the summary,
zeroing `api_eydam` is harmless (it would merely remove a phantom damage
number). Opening torpedo (cruiser/submarine alpha strike) is the classic
"instant taiha" source and is one of the two affected phases.

## Prevention

- god_mode / one_hit_kill packet rebuilds must zero the **per-attacker** damage
  array the client animates from, not an aggregate summary. Per phase:
  shelling/OASW `api_damage` (where `api_at_eflag==1`), kouku `api_fdam`,
  opening torpedo `api_eydam_list_items`, closing torpedo `api_eydam`. Always
  keep `api_fdam` zeroed too for consistency.
- When adding any new damage phase to `BattlePacket`, add a god_mode test that
  drives a friendly into that phase and asserts zero animated damage — the
  `raigeki: None` default silently skips this.

## Related

- [Battle damage foundation](../architecture-patterns/battle-damage-foundation.md) — the client HP reconstruction invariant this relies on.
- [Debug overlay bridge](../architecture-patterns/debug-overlay-bridge.md) — god_mode overlay design; Learning #5 field list corrected here.
- `crates/emukc_battle/src/debug_overlay.rs` — `rebuild_day_packet_arrays`, `zero_friendly_hougeki_damage`.
- `crates/emukc_battle/src/types/packet.rs` — `BattleRaigeki` / `BattleOpeningAttack::record_torpedo_hit` (fdam vs eydam population).
