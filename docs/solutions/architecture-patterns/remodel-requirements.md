---
title: "Ship remodel requirements: start2 plus the client's own tables"
date: 2026-10-16
category: architecture-patterns
module: emukc_model
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Changing what a ship remodel (改造) charges or which ship it becomes"
  - "Updating the client and re-syncing decoder assets"
tags: [remodel, start2, shipupgrade, main-decoder, remodel-tables, oracle]
related_components: [emukc_gameplay, emukc_bootstrap, main-decoder]
---

# Ship remodel requirements: start2 plus the client's own tables

## Context

The server used to take every remodel requirement from the kcwiki ship data
(`Kc3rdShipRemodelRequirement`). It disagreed with the client on 88 routes (#180): kcwiki's
`true` for "needs one" parsed as 0, reverse conversions charged the target's forward remodel,
and a ship missing from kcwiki could not be remodelled. The client never reads kcwiki: it
reads start2 and a few hard-coded tables, and the server now does the same.

## Rule

`Codex::remodel_requirement(before)` (`crates/emukc_model/src/codex/remodel.rs`) is the
client's `ShipUpgradeModelHolder.getNextID` / `getRequires`:

- No `api_aftershipid` on the ship → no remodel, even if a shipupgrade row exists.
- Target: the first `api_mst_shipupgrade` row with `api_current_ship_id == before` names it,
  else `api_aftershipid`.
- Level, ammo, steel: `api_afterlv`, `api_afterbull`, `api_afterfuel` of the ship **before**.
- Use items: from the row (58, 65, 78, 77, 94, 100, boilers); none without a row.
- 開発資材, 高速建造材 (by before) and 新型砲熕兵装資材 75, 改修資材, 工廠資材 104 (by after,
  one entry narrowed to a single before) are hard-coded in the client. `main-decoder`
  (`src/remodel-tables.ts`) reads them into `crates/emukc_bootstrap/assets/remodel_tables.json`
  under `--sync-assets`; bootstrap copies it into the codex. Never hand-edit it.

`remodel_impl` deducts every field. Level is not checked server-side; the client checks it.

`Kc3rdShip.remodel` stays: new ships take their starting level from it, and practice rivals
their minimum level. It is no longer a source of what a remodel costs.

## Checks

- `make remodel-tables-oracle` runs the client's own functions for every ship id and compares
  them with the asset (also non-blocking in `make update`).
- `every_remodel_needs_what_the_client_says` walks every ship in the raw kcwiki start2 and
  compares the codex with an independent copy of the client rule. No exceptions list.
- `tests/gameplay_tests/remodel_requirements.rs` covers the under- and over-charged classes.

## Known gap

狭霧改 → 改二 resolves correctly but still fails: 902 is missing from the kcwiki ship data,
and `new_ship` / `cal_ship_status` need it (#182).
