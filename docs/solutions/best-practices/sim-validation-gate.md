---
title: "Scenario presets auto-gate through day+night protocol validation; the gate must bite"
date: 2026-06-25
category: best-practices
module: emukc_gameplay
problem_type: testing_pattern
component: test_harness
severity: medium
applies_when:
  - "Adding a scenario preset to PRESETS"
  - "Adding or modifying the sim->validate gate in tests/sim_validation_gate.rs"
  - "Wiring any always-pass validator into a test so it can't silently become a no-op"
tags: [testing, scenario-preset, protocol-validation, gate, day-night, registry-driven, negative-test]
related_components: [emukc_bootstrap]
---

# Scenario presets auto-gate through day+night protocol validation; the gate must bite

## Context

`crates/emukc_gameplay/tests/sim_validation_gate.rs` runs every scenario preset
through the battle sim across a bounded seed set and asserts the serialized
battle response passes the client-derived protocol rules. The preset list is a
registry (`PRESETS`), so the gate is data-driven: the coverage you get is
whatever is registered, and the gate is only meaningful if it can actually
fail.

## Guidance

### Adding a preset extends gate coverage for free

Scenario presets live in a registry: `pub const PRESETS: &[Preset]`
(`crates/emukc_gameplay/src/scenario/mod.rs:119`), each entry carrying
`name`, `build: fn() -> Scenario`, and the `maparea` / `mapinfo` it sorties on.
The gate iterates `for preset in PRESETS` (`sim_validation_gate.rs:51`, `:140`).
**To extend protocol coverage, add a `Preset` to `PRESETS` — do not write a new
per-preset test.** Both the day gate and the night gate pick it up
automatically. (Presets are also resolvable by name via `Preset::lookup`,
used by the `battle` CLI's `resolve_scenario`.)

### Every preset is gated across a bounded seed set, day and night

- `const SEEDS: &[u64] = &[1, 2, 3, 5, 8, 13]` (`sim_validation_gate.rs:30`) —
  each preset is simulated under each seed so RNG-dependent shape variation is
  exercised, not just one lucky roll.
- `every_preset_day_battle_passes_protocol_validation` asserts
  `validate_day_battle_response(...)` reports no errors for every (preset, seed).
- `every_preset_night_battle_passes_protocol_validation` does the same with
  `validate_night_battle_response(...)`.

### The night gate uses the deterministic midnight path on purpose

The night gate produces its packet via `context.sortie_sp_midnight_battle(pid, 1)`
(`sim_validation_gate.rs:161`), **not** a full sortie-then-night sequence. The
full day→night sequence is flaky on the current sim; `sortie_sp_midnight_battle`
exercises the identical `simulate_night` → `build_night_response` path
deterministically. If you change night response building, this is the path the
gate covers — keep it deterministic or the gate becomes flaky.

### A gate of always-pass assertions must have a negative test

`gate_bites_on_corrupted_payload` (`sim_validation_gate.rs:101`) feeds a
deliberately corrupted payload and asserts `report.has_errors()`. This is not
optional decoration: a protocol gate whose validator silently stopped checking
(e.g. a field table emptied, an early return added) would keep reporting
"no errors" and the green gate would be a lie. The negative test proves the
gate can still fail. Any "everything passes" gate you add needs a paired
"this must fail" test.

## Why This Matters

A registry-driven gate is only as honest as (a) what's registered and (b)
whether it can fail. Without the negative test, a gate that quietly degrades to
a no-op stays green forever and gives false confidence. Without the
deterministic night path, the gate flakes and gets muted. The pattern — add to
the registry, keep the path deterministic, pair every all-pass gate with a
bite test — is what keeps this from rotting into a rubber stamp.

## When to Apply

- Adding a scenario preset: register it in `PRESETS`; do not hand-roll a test.
- Adding any "all cases pass validation" gate: add the matching corrupted-input
  test that proves it bites.
- Touching night response building: verify `sortie_sp_midnight_battle` stays
  deterministic.

## Related

- `docs/solutions/architecture-patterns/battle-protocol-validator-boundary.md`
  — what `validate_day/night_battle_response` actually check (protocol shape,
  not behavioral correctness); this gate inherits that boundary.
- `docs/solutions/conventions/test-example-layout.md` — `tests/` is test-only.
