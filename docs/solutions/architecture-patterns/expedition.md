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
tags: [expedition, requirements, kcanotify, escort-carrier]
related_components: [emukc_model, emukc_bootstrap, emukc_db]
---

# Expedition requirements: what they mean and when they are judged

## Context

An expedition's requirements come from kcanotify's `expedition.json`, parsed by
`emukc_bootstrap/src/parser/kcanotify/expedition.rs` into
`Kc3rdExpeditionRequirements` in the codex's `expedition_condition.json`.
Issues #116, #117 and #118 fixed what the fields mean and when they are checked.

## Guidance

### When: at launch, failing on return

A fleet that does not meet the requirements still sails, as in the official
game, and fails on return.

- `start_expedition` errors only when the fleet cannot leave at all: it is
  empty, or already away. Otherwise it calls `evaluate_expedition_requirements`
  (a `bool`; the unmet reason goes to the debug log) and stores the verdict in
  `fleet.launch_requirements_met`.
- The verdict is final. The fleet cannot be reorganised while away, so nothing
  is re-read on return; a test that changes stats mid-trip pins this.
- `complete_expedition` picks the failure in this order: recall, then
  requirements unmet at launch (`ExpeditionFailureKind::RequirementsNotMet`),
  then fatigue (a ship at morale 39 or less after the return drop). An unmet
  trip fails like a fatigued one: no resources or items, failure experience,
  and no `ExpeditionCompleted` outcome, so no quest progress.
- Full supply is one of the requirements, not a start error. The launch cost is
  taken down to 0 at most: a ship short of it still sails.
- `launch_requirements_met` was added with `ALTER TABLE ... DEFAULT TRUE`
  (`entity/profile/fleet.rs`): a fleet already away on an older database passed
  the old start check, which refused unmet requirements.

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

## Why This Matters

The wrong reading made B1 ask for 200 firepower, let any flagship take 43 and
45, and made A4's escort-carrier branch impossible. Refusing the start was not
what the official game does.

## Known gaps

- 大発 bonuses (#119) are a numeric change under the Balance Defaults Policy and
  are not covered here.
