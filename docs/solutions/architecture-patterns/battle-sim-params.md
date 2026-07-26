---
title: "Battle execution params: NightBattleInput struct for execute_night"
date: 2026-06-22
category: architecture-patterns
module: emukc_battle
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Calling or modifying the execute_night entry point"
  - "Refactoring battle simulation parameter passing"
tags: [battle, night-battle, execute-night, input-struct, api]
related_components: [emukc_gameplay]
---

# Battle execution params: NightBattleInput struct for execute_night

## Context

Night battle simulation previously took six individual parameters. Bundling
them into a single struct reduces call-site noise, makes future parameter
additions non-breaking, and keeps the gameplay crate's construction of the
input explicit.

## Guidance

The following invariants hold for the public `execute_night` entry point:

- **Struct parameter, not individuals.** `execute_night` SHALL accept battle
  parameters via a `NightBattleInput` struct rather than individual
  parameters. Its signature takes `&Codex`, `NightBattleInput`, and
  `&mut impl BattleRng` as its only parameters.
- **All six fields present.** `NightBattleInput` SHALL contain the fields:
  `friendly`, `enemy`, `friendly_formation_id`, `enemy_formation_id`,
  `engagement`, `air_state` — covering the full previous individual-parameter
  set.
- **Facade owns execution policy.** Every night-battle call site in
  `emukc_gameplay` SHALL construct `NightBattleInput` and pass it to
  `execute_night`; callers SHALL NOT invoke raw simulation or apply the debug
  overlay themselves.

## Why This Matters

Individual parameters do not scale: adding a seventh night-battle context
field would require editing every caller and changing the signature.
`NightBattleInput` localizes the contract and makes the caller's intent
(self-contained battle context) legible at the call site.

## When to Apply

- When extending night battle context (add a field to the struct, not a new
  parameter).
- When writing a new `execute_night` caller.
- When reviewing changes to the night-battle entry signature.

## Examples

```rust
let input = NightBattleInput {
    friendly,
    enemy,
    friendly_formation_id,
    enemy_formation_id,
    engagement,
    air_state,
};
let simulation = execute_night(&codex, input, &mut rng);
```

## Related

- `crates/emukc_battle/` — `NightBattleInput` definition and `execute_night`.
- `crates/emukc_gameplay/src/game/battle/` — sortie and practice call sites.
