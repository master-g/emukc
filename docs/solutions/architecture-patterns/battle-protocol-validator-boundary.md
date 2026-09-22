---
title: "Battle response validators check protocol conformance, not behavioral correctness"
date: 2026-06-25
category: architecture-patterns
module: emukc_bootstrap
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Using validate_day_battle_response / validate_night_battle_response or analyze_day_battle_incident"
  - "Extending the battle field tables (DAY_BATTLE_* / NIGHT_BATTLE_*)"
  - "Tempted to treat the battle validators as a correctness check on damage/hit results"
tags: [battle, validator, protocol-conformance, diagnostic, field-table, day-night, boundary]
related_components: [emukc, main-decoder]
---

# Battle response validators check protocol conformance, not behavioral correctness

## Context

`crates/emukc_bootstrap/src/battle_rules.rs` exposes
`validate_day_battle_response`, `validate_night_battle_response`, and
`analyze_day_battle_incident` (all `<T: Serialize>`). They are reached from the
`battle validate` / `battle analyze-incident` CLI commands. It is easy to read
the name "validate battle" as "did the battle compute the right numbers" — it
does not. These are **protocol-shape diagnostics**, driven by field tables
decoded from the client's `main.js`.

## Guidance

### What the validators actually check (protocol conformance only)

`validate_day_battle_response` (battle_rules.rs:851) serializes the response to
JSON and asserts **structural conformance** against the client-derived field
tables:

- Required array fields exist (`DAY_BATTLE_ARRAY_FIELDS`,
  `DAY_BATTLE_ARRAY_FLAG_FIELDS`) and required scalars exist
  (`DAY_BATTLE_SCALAR_FLAG_FIELDS`).
- Parallel arrays are equal length (enemy `api_ship_ke` / `api_ship_lv` /
  `api_e_nowhps` / `api_e_maxhps` / `api_eSlot` / `api_eParam`; friendly
  `api_f_nowhps` / `api_f_maxhps` / `api_fParam`).
- Flag↔payload presence agreement: when a high-confidence payload field is in
  use, its gating flag must agree (`api_stage_flag[0]`↔`api_kouku`,
  `api_opening_flag`↔`api_opening_atack`, `api_opening_taisen_flag`↔
  `api_opening_taisen`, etc.).

Since 2026-09-22 it also asserts two things that are still client-parse
contracts rather than game logic:

- **Attack-type acceptance.** Every `api_at_type` / `api_sp_list` value must be
  one the consuming client module will dispatch, per
  `crates/emukc_bootstrap/assets/battle_attack_type_acceptance.json`. The three
  stages have three different sets — day shelling takes 7 (空母カットイン),
  opening ASW throws on it, and night shelling reads `api_sp_list` through a
  second `PhaseHougeki` copy whose numbering contradicts the day one. Out of
  set is an error-level `UnacceptedAttackType`.
- **Resource coverage.** Every resource path the response makes the client
  request is checked against `make_list`'s generation rules; an uncovered one
  is an error-level `ProtocolSuspicion`. Name plates (`btxt_flat`) derive from
  display equipment only, never from the enemy loadout in `api_eSlot`, and a
  day carrier cut-in derives none because `PreloadCutinKubo` loads it only when
  `night == 1`. `item_up` ids are normalized the way generation normalizes
  them, or every abyssal equipment would read as a gap.

Both remain protocol questions: "will the client parse this" and "will the
client 404 on this". It does **not** check that damage numbers, hit/miss, or
target selection are *correct* — only that the packet has the right shape the
client will parse.

What the validator deliberately does **not** own is which equipment may appear
in which attack's display. That is a game rule, it needs an equipment-type
table, and a copy of such a table drifting is exactly what produced the
archived `102 -> btxt_flat` incident. It lives in `emukc_battle`, guaranteed by
construction and tested there; the client-visible consequence — display
equipment with no name plate — is what the coverage check above catches.

### Night reuses day; it is a mirror, not a fork

`validate_night_battle_response` (battle_rules.rs:1098) mirrors the day
validator and **reuses its helpers** over night-specific field tables
(`NIGHT_BATTLE_ARRAY_FIELDS`, `NIGHT_BATTLE_SCALAR_FLAG_FIELDS`,
`NIGHT_BATTLE_HOUGEKI_FIELDS`). Night adds only what differs:
`check_night_hougeki_shape` (battle_rules.rs:1045) for the `api_hougeki` /
`api_n_hougeki1` / `api_n_hougeki2` shapes. `analyze_day_battle_incident`
(battle_rules.rs:1278) is reused for night incidents too (the banner-resource
builder is shared — battle_rules.rs:1961). When adding a battle phase or
field, extend the field-table constants, not the per-call logic.

### These are diagnostics, never runtime auto-checks

Sortie/practice handlers do **not** call these validators. They are explicit
CLI-invoked tools (`cargo run -- battle validate` / `analyze-incident`). Do not
wire them into request handling and do not assume a passing sortie was
validated.

## Why This Matters

A packet whose values are all legal but semantically wrong — the day
anti-submarine `api_at_type = 7` this plan fixed, which is a valid
`PhaseHougeki` branch and plays a carrier cut-in for a depth charge — passes
the acceptance check by design. Only the crate that produces it can catch that,
and only with its own unit tests.

A protocol-conformant packet that carries wrong numbers passes these
validators by design — they guard the client-parse contract, not game logic.
Confusing the two leads to either false confidence ("validator green, so the
battle math is right") or scope creep (stuffing damage-correctness assertions
into a protocol linter, where the "right" value has no client-derived source of
truth to check against). Behavioral correctness is covered separately by
gameplay tests and the golden transcript.

## When to Apply

- When adding a new battle phase, field, or response variant: extend the
  `DAY_BATTLE_*` / `NIGHT_BATTLE_*` field tables and let day/night share helpers.
- When someone asks the validator to "catch a wrong-damage bug": redirect to
  gameplay tests / `tests/gameplay_tests/battle_golden.rs`.
- After battle knowledge changes: re-sync field tables via
  `cd main-decoder && bun run decode -- --sync-battle-assets` (the field tables
  derive from decoded assets — see CLAUDE.md *Client-Derived Battle Validation*).

## Related

- `docs/solutions/architecture-patterns/bootstrap-validator-dependency-direction.md`
  — the same validator/finding/report shape, applied to map route + source
  cross-check, and why these live in emukc_bootstrap over public model types.
- `docs/solutions/architecture-patterns/battle-damage-foundation.md` — where
  actual damage *correctness* is defined.
