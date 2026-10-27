---
title: "Importing a live account: id remapping and the stat report"
date: 2026-10-27
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Changing `import-account`, `LiveSnapshot` or `import_account_impl`"
  - "Adding a domain to the import (quests, maps, air bases, presets)"
  - "Reading an import report's stat mismatches"
tags: [import, live-api, snapshot, id-remap, cal-ship-status, oracle]
related_components: [emukc_model, emukc_gameplay, scripts]
---

# Importing a live account

`emukcd import-account --snapshot <dir> -u <name> -p <pass>` turns a capture written by
`scripts/fetch_live_api.py` into a new local account. The field map is
`docs/research/2026-10-27-account-import-field-map.md`, the plan
`docs/plans/2026-10-27-001-feat-account-import-plan.md`.

## Layers

- `emukc_model::kc2::live::LiveSnapshot::from_api_data` takes each field out of the
  `api_data` of `api_port/port` and `api_get_member/require_info` by name. A whole-response
  parse would fail: the local port response type declares `api_plane_info` and
  `api_furniture_affect_items` wrong.
- `src/bin/cli/import_account.rs` reads the two files (the `svdata=` prefix is optional)
  and refuses anything but `api_result: 1`. Rust never sees a token and makes no request.
- `Ctx::import_account` / `import_account_impl` write the capture over a profile that
  `new_profile` has just seeded, in one transaction. The CLI checks the codex before
  creating the account and deletes the account if anything after that fails, so a name
  can be retried.

## What the live server sends that the local types did not expect

Found by parsing the 2026-10-27 capture:

- `api_basic.api_member_id` is a string. `KcApiUserBasic` takes a number or a string.
- `api_basic` has no `api_max_quests`, `api_extra_supply` or `api_war_result` (they are
  local additions); the first two come from `api_parallel_quest_count` and require-info.
- A ship has no `api_sally_area` outside an event.
- The furniture coin is not in `api_useitem`; it is `api_basic.api_fcoin`, stored as use
  item 44.
- A new profile has no option-settings row. The import writes one with the live skin and
  the volumes the client falls back to without a row (`initialize_volume`: 30/40/60, both
  voice switches on).

## Order of the writes

`ship.id` and `slot_item.id` are auto-increment keys shared by every profile, and
`From<KcApiShip> for ship::ActiveModel` sets `id: Set(api_id)`, so the import sets
`id: NotSet` and keeps a live id → local id map:

1. equipment, sorted by live id, `equip_on` 0;
2. ships, sorted by live id, slots mapped through the equipment map (`-1` and `0` kept);
3. each carried item's `equip_on`;
4. fleets, the repair docks' `ship_id`, and `api_sp_effect_items`, through the ship map.

Sorting by live id keeps the client's 入手順. The construction docks' `ship_id` is a master
id and is copied as is.

Quests are not observed: the seeded starting quests would advance on hundreds of inserts.
A fleet on an expedition keeps `launch_*` at 0, and the return path then judges it by its
ships, which cannot change while it is away.

## The stat report

Each ship goes through `Codex::cal_ship_status` on a copy that still carries the live
equipment ids. The result is what gets stored, and it is compared with what the live
server sent for firepower to luck (both columns), max HP, speed, range, `api_exp[1]`,
`api_ndock_time`, `api_ndock_item`, `api_srate` and `api_locked_equip`. A mismatch comes
from a local formula, from the codex's master data, or from a known approximation (the
report lists those). Rule out the data first: the capture holds the same moment's
`api_start2`, and evasion, ASW and line of sight do not come from `api_start2` at all but
from third-party data (#293 and #294 are errors of that kind).

On the 2026-10-27 capture (340 ships) every combat stat, HP and speed matched, and the
codex's ship and equipment masters for everything the account owns equal the live
`api_start2` in every stat, range, speed and slot field, so the three that did not match
are formula questions:

- `api_ndock_item`: the live server sends `[0, 0]` for an undamaged ship, the local formula
  gives `[1, 1]`;
- `api_srate`: 171 ships are 4 live and 5 locally;
- `api_leng`: 19 ships.

## Seeing it in the client

`IMPORT_SNAPSHOT=<dir> python tests/headless/run.py fresh_1_1 "<steps>"` imports the capture
into the headless workspace before `new-session`, which then signs in to that account. The
screenshots hold the account's nickname, so they stay under `.data/temp`.
