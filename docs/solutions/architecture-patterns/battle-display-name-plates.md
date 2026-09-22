---
title: "api_si_list names equipment the client will draw a name plate for, nothing else"
date: 2026-09-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Choosing what goes into api_si_list for any attack phase"
  - "Adding or removing ids from BTXT_FLAT_IDS"
  - "Investigating a battle resource 404 the client raised on slot/btxt_flat"
tags: [battle, api-si-list, btxt_flat, display-equipment, cdn-coverage, resource]
related_components: [emukc_bootstrap, main-decoder]
---

# `api_si_list` names equipment the client will draw a name plate for, nothing else

## Context

Whatever the server puts in `api_si_list` is more than a label: for several
attack phases the client turns it into a CDN request for
`kcs2/resources/slot/btxt_flat/<id>_<key>.png`, the equipment's name plate. If
that file does not exist, the request 404s during the battle animation. The
archived `102 -> btxt_flat` incident (九八式水上偵察機(夜偵) shown for a
surface attack) is exactly this.

The trap is that the request is *conditional* on which phase the client
resolves the attack to, and the name plate exists for only part of the
equipment catalogue. Naming "the most relevant equipment" is the wrong
instinct; naming "equipment this phase can draw" is the rule.

## Guidance

### Which phases request a name plate

`CutinAttack` (`module-19362`) loads `btxt_flat` for its slot, but only when
constructed with `showTelop = true`. The phases differ:

| Phase | `showTelop` | Name plate |
|---|---|---|
| `PhaseAttackNormal` (gun) | `true` | yes |
| `PhaseAttackRaigeki` (torpedo) | `true` | yes |
| `PhaseAttackRocket`, `PhaseAttackKakuza`, `PhaseAttackSpType4KaTsu` | `true` | yes |
| `PhaseAttackKansaiki` (aircraft) | **`false`** | no |
| `PhaseAttackBakurai` (depth charge) | **`false`** | no |

`CutinDouble` (連撃) loads it for two slots and `CutinResourcesPreloadTask`
(destroyer night cut-ins) for three, both unguarded. `PreloadCutinKubo`
(carrier cut-in) loads it for three slots but only when `night == 1` and only
for non-enemy ids.

Which phase a plain attack (`api_at_type = 0`) resolves to is decided by the
client's `_getNormalAttackType`, which reads the **attacker's own slots and
whether the defender is a submarine** — never `api_si_list`. So for an
anti-submarine attack or a carrier's plain attack, the content of
`api_si_list` changes nothing the player sees.

### Which equipment has a name plate

Probed against the game CDN on 2026-09-22, one path per id, with gun ids 0041
and 0220 as the 200 control:

- Guns, secondaries, torpedoes, radars: present (90%+ of each family).
- Sonars (0046, 0047, 0132), depth charges, seaplanes (0025, 0102): **absent**.
- Carrier aircraft: absent, except the night-capable machines (F6F-3N,
  Swordfish, TBM-3D, 試製 夜間瑞雲 …), which are what the night carrier cut-in
  draws.
- Recently added guns (0572, 0579, 0582, 0583, 0584): present, and were missing
  from `BTXT_FLAT_IDS` — the table lags new content.
- Abyssal 深海標準 guns (1660, 1661): absent, correctly.

The pattern is not arbitrary: a name plate exists for equipment that some
name-plate-bearing phase can draw, and for nothing else.

### The rule that follows

`api_si_list` names only equipment the phase will draw:

- Plain surface attack: guns and secondaries. Aircraft and seaplanes are out —
  a carrier resolves to `PhaseAttackKansaiki`, which never reads the list.
- Anti-submarine attack: `[-1]`. Nothing that forms one has a name plate, and
  `PhaseAttackBakurai` / `PhaseAttackKansaiki` never read the list anyway.
- 連撃 and the gunnery cut-ins: guns first, then secondaries.
- Night cut-ins: the bucket that forms that cut-in (torpedoes, lookouts,
  radars, drums), which is what `CutinResourcesPreloadTask` draws.

## Why This Matters

`BTXT_FLAT_IDS` is a record of what exists upstream, not a filter the battle
crate may consult: `emukc_battle` does not depend on `emukc_bootstrap`, and a
second copy of a display-type table drifting is what produced the original
incident. So the battle crate has to be right *by construction* — by naming
only equipment families whose phases draw a plate — and the coverage check in
`validate_day_battle_response` catches the client-visible consequence if it
ever is not.

## When to Apply

- Adding a display path: decide first which client phase will consume it and
  whether that phase draws a name plate. If it does, only name equipment
  families that have one.
- A gate finding says a derived `btxt_flat` path is uncovered: probe that one
  path before touching either side. A 200 means `BTXT_FLAT_IDS` is stale (add
  the id, then `make decode-main` to re-sync `cache_rules.json` — the table
  flows Rust → asset). A 404 means the display rule is too broad.
- Never infer absence from `BTXT_FLAT_IDS` alone: the local `z/cache` sweep only
  requested ids already in the table, so it proves what is in the table exists
  and says nothing about what is not.

## Related

- `docs/solutions/architecture-patterns/battle-attack-type.md` — participation
  is ship-type gated; equipment only selects the display type.
- `docs/solutions/architecture-patterns/battle-protocol-validator-boundary.md`
  — where the coverage check lives and why the display rule does not live there.
- `docs/battle/rules.md` — `display.names_only_equipment_with_a_name_plate`,
  `display.day_asw_names_no_equipment`, `resource.derived_paths_are_generated`.
