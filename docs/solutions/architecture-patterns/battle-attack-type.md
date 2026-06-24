---
title: "Battle attack type: participation is ship-type / base-stat gated, equipment only selects display type"
date: 2026-06-24
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Modifying day shelling / opening torpedo / closing torpedo participation gates"
  - "Selecting api_at_type / api_si_list display type for a day attack"
  - "Reasoning about which ship types fire in which phase"
tags: [battle, attack-type, shelling, opening-torpedo, closing-torpedo, ship-type, api-si-list, api-at-type]
related_components: [emukc_gameplay]
---

# Battle attack type: participation is ship-type / base-stat gated, equipment only selects display type

## Context

An earlier implementation used an *equipment checklist* as the participation
gate for the day shelling phase, and a hardcoded *ship-type whitelist* for the
closing torpedo phase. Both are wrong: a DD carrying only a torpedo would
render a torpedo attack animation during shelling, and battleships with a base
torpedo stat (Bismarck drei, 金剛型第三改装, Гангут) were excluded from closing
torpedo while torpedo-less types (DE, LHA) were nominally included.

The correct model, verified against wikiwiki.jp/kancolle/戦闘について: **a
ship's participation in a phase is decided by its ship type and base stats;
equipment only selects the display type (`api_at_type` / `api_si_list`) and
adds to the damage stats.** This spec codifies the participation gates as they
ship in `crates/emukc_battle/src/targeting.rs`.

## Guidance

The following gates hold (all in `can_*` helpers in `targeting.rs`):

- **Day shelling is ship-type gated.** SS / SSV never shell. CV / CVL / CVB
  shell only when they have at least one attack plane left on slot (total
  attack-plane count > 0, i.e. not fully shot down). Every other surface ship
  type always shells, regardless of equipment. (`can_shell_day_ship`)
- **Display type falls back to normal.** When a ship has no equipment relevant
  to the attack, `api_at_type = 0` (normal single attack) and
  `api_si_list = [-1]` (a sentinel, not a real slot id); damage uses base
  firepower. Equipment selects the display type only when present — it is not a
  participation gate. (`day_attack_display_ids`)
- **Closing torpedo is base-torpedo gated, and damage-state gated.** A ship
  fires closing torpedo when `api_raisou[0] > 0` (base torpedo) **and** it is
  not chūha-or-worse — strictly `hp() * 2 > api_maxhp` (HP above 50%). There is
  **no ship-type whitelist**: BBs with base torpedo are included; DE / LHA and
  others with base torpedo 0 are excluded. (`can_closing_torpedo_ship`)
- **Opening torpedo is equipment / type gated, and ignores damage state.**
  Precondition `api_raisou[0] > 0`. Then: CLT always fires; SS / SSV fire when
  `api_lv >= 10` **or** equipped with 特殊潜航艇 (甲標的,
  `SpecialSubmarineVessel`); every other ship type fires only when equipped
  with 甲標的. Unlike closing torpedo, **damage state does not block opening
  torpedo** — a chūha ship still fires (開幕雷撃は損傷度を問わず発動する).
  (`can_opening_torpedo_ship`)

## Why This Matters

Using equipment as the participation gate makes the client render the wrong
attack animation and miscount who fires in a phase. Gating by ship type and
base torpedo stat matches the original server: equipment is demoted to
display-type selection and a stat modifier. The base-torpedo gate is also the
only rule that handles every edge case uniformly (torpedo-armed BBs, AVs with
vs without torpedo) without an ever-growing whitelist.

## When to Apply

- When changing any of `can_shell_day_ship`, `can_closing_torpedo_ship`,
  `can_opening_torpedo_ship`, or `day_attack_display_ids`.
- When adding a new attack phase or display-type selector.
- When reviewing why a given ship does or does not fire in a phase.

## Examples

```text
// DD carrying only a torpedo, day shelling phase:
//   participates (ship type gate passes), but
//   api_at_type = 0, api_si_list = [-1]  -> normal shelling animation, base firepower

// Bismarck drei (FBB) with base torpedo > 0, closing torpedo phase, HP > 50%:
//   can_closing_torpedo_ship -> true   (no ship-type whitelist)

// SS at Lv 8 with no 甲標的, opening torpedo phase:
//   can_opening_torpedo_ship -> false  (Lv < 10 and no minisub)
// Same SS at Lv 10:
//   can_opening_torpedo_ship -> true   (level exception, no equipment needed)
```

## Related

- `docs/solutions/architecture-patterns/battle-damage-foundation.md` — how the
  resulting damage is applied and reported (enemy sortie overkill lives there,
  not here).
- `docs/solutions/architecture-patterns/night-battle-sinking-protection.md` —
  sortie/practice context and 轟沈ストッパー.
- `crates/emukc_battle/src/targeting.rs` — `can_shell_day_ship`,
  `can_closing_torpedo_ship`, `can_opening_torpedo_ship`,
  `day_attack_display_ids`, and their `#[cfg(test)]` regression tests.
- `docs/plans/archive/2026-06-22-002-fix-battle-attack-system-plan.md` — the
  change that introduced these gates (R1–R4); this doc is its checked-in home.

## Deferred

SS shelling against installations with 特二式内火艇 is a known exception that is
**not** implemented (plan 002 non-goal). SS / SSV currently never shell.
