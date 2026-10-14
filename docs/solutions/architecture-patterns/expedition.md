---
title: "Expedition requirements: what they mean and when they are judged"
date: 2026-10-14
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Changing how an expedition's requirements are read, checked or reported"
  - "Consuming kcanotify's expedition.json or ships_filter_data.json"
  - "Touching start_expedition, complete_expedition or the fleet launch columns"
  - "Changing expedition resource rewards or the landing craft table"
tags: [expedition, requirements, kcanotify, escort-carrier, landing-craft]
related_components: [emukc_model, emukc_bootstrap, emukc_db]
---

# Expedition requirements: what they mean and when they are judged

## Context

An expedition's requirements come from kcanotify's `expedition.json`, parsed by
`emukc_bootstrap/src/parser/kcanotify/expedition.rs` into
`Kc3rdExpeditionRequirements` in the codex's `expedition_condition.json`.
Issues #116, #117 and #118 fixed what the fields mean and when they are checked.

## Guidance

### When: supply at launch, the rest on return

A fleet that does not meet the requirements still sails, as in the official
game, and fails on return.

- `start_expedition` errors only when the fleet cannot leave at all: it is
  empty, or already away. Otherwise it stores whether every ship is fully
  supplied in `fleet.launch_fully_supplied`; it has to be read before the
  launch cost takes fuel and ammo.
- Every other requirement is judged in `complete_expedition` by
  `evaluate_expedition_requirements` (a `bool`; the unmet reason goes to the
  debug log), on the ships as they are then. The official game lets a
  reinforcement slot be re-equipped while the fleet is away, and a fleet short
  at launch that is made good that way succeeds (wikiwiki 「遠征」, verified
  there). A test that changes stats mid-trip pins this both ways.
- `complete_expedition` picks the failure in this order: recall, then short of
  supply at launch or of the requirements on return
  (`ExpeditionFailureKind::RequirementsNotMet`), then fatigue (a ship at morale
  39 or less after the return drop). An unmet trip fails like a fatigued one:
  no resources or items, failure experience, and no `ExpeditionCompleted`
  outcome, so no quest progress.
- Full supply is one of the requirements, not a start error. The launch cost is
  taken down to 0 at most: a ship short of it still sails.
- `launch_fully_supplied` was added with `ALTER TABLE ... DEFAULT TRUE`
  (`entity/profile/fleet.rs`): a fleet already away on an older database passed
  the old start check, which refused an unsupplied fleet.

### What: kcanotify's fields

| kcanotify | Requirements field | Meaning |
|---|---|---|
| `total-firepower` | `total_firepower` | sum of `firepower_now` |
| `total-fp` | `total_aa` | sum of `aa_now`: kcanotify sums `taiku` for it, it is **not** firepower |
| `total-asw`, `total-los` | `total_asw`, `total_los` | sums of `asw_now`, `los_now` |
| `flag-cond` `"7/27"` | `flagship_types: [7, 27]` | the flagship is any of them; empty = no requirement |
| `total-cond` `27-1\|2-2` | `composition` | 27 is an escort carrier |

Equipment improvement bonuses and the ASW of carrier aircraft are not
subtracted: the totals are the ships' current stats.

### Escort carriers (ship type 27)

27 is not in `api_mst_stype`; only kcanotify's expedition rules use it. The
codex's `escort_carriers.json` is kcanotify's `ships_filter_data.json`
`cve_ids` (downloaded as `kcanotify_ships_filter.json`). An escort carrier
counts as 27 and as its own type (all are 7 today) in each composition
requirement, each requirement counted on its own. As flagship it is 27 only, as
in kcanotify: `"7/27"` lets it lead, a bare `"7"` would not.

The list goes stale when a new escort carrier ships, until kcanotify adds it
and the codex is rebuilt.

### Rewards: landing craft bonus

`calculate_expedition_resource_reward` gives the four resources as
`⌊R·S⌋ + ⌊R·S·(B1 + B★)⌋ + ⌊R·S·B2⌋` (#119), item rewards unchanged:

- `R` the base amount, `S` 1 or 1.5 on great success; `R·S` is not floored first.
- `B1 = min(20%, Σ craft rates + 5% for Kinu Kai Ni)`; `B★ = B1 × average
  stars / 100`, the average taken over the craft with a rate (not Kinu).
- `B2 = min(cap, 2% × 特大発動艇 count)`, the cap read from `TOKU_CAPS` by
  特大発 count (up to 3, 4 or more) and 大発動艇 (68) count.

Rates and caps are KC3Kai's `calcLandingCraftInfo` (`KC3Kai@6b0534d2`), checked
against wikiwiki's 大発動艇 and 特大発動艇 pages; the three separate floors are
fandom's `Expedition/Introduction`. kcanotify differs twice and is not followed:
its star term is a flat `0.2% × average stars`, and its cap column counts 166 and
167 along with 68. A unit test pins each difference. wikiwiki writes two floors
instead of three, which gives 1 more when `R·S` ends in .5 (R=35 great success
with four 大発: 62 here, 63 there); change it only on a live counterexample.

The craft are read on return, from all six slots of `mission_ships`, like the
requirements: the reinforcement slot can be re-equipped while the fleet is away,
and KC3Kai counts it too. Arithmetic is in integers (basis points, `S` as
3/2), because `f64` rates such as 0.29 floor 1 short.

`LANDING_CRAFT_RATES` is a hand-kept id list: type 24 also holds craft with no
rate (230, 355, 482, ...). It goes stale when a new landing craft ships; add it
from wikiwiki's 大発動艇 equipment table and KC3Kai. Kinu Kai Ni's combined-fleet
rule does not apply: expeditions have no combined fleet.

## Why This Matters

The wrong reading made B1 ask for 200 firepower, let any flagship take 43 and
45, and made A4's escort-carrier branch impossible. Refusing the start was not
what the official game does.

## Known gaps

- The 特大発 cap for 4 or more is marked unverified on fandom; wikiwiki and
  KC3Kai agree on the table used.
