# KCSAPI Handler Coverage Analysis

> Last updated: 2026-09-21
> Reference: `docs/apilist.txt` (a copy of
> [sinsinpub/kcs2-assets apilist.txt](https://github.com/sinsinpub/kcs2-assets/blob/master/api_info/apilist.txt))

This document is the **roadmap**: what each missing module costs, what it
depends on, and in what order to build it. It deliberately carries no endpoint
inventory of its own.

## Which endpoints are implemented

`apilist.md` in the repo root holds the two lists, and they are mechanically
checkable rather than hand-maintained: extract `nest("/prefix", mod::router())`
from `src/bin/net/router/kcsapi/mod.rs` plus each submodule's `.route("/leaf"`,
and diff that against the fenced blocks. Verified on 2026-09-21 at client
6.3.5.0: **117 implemented**, **31 missing**, no overlap, and the 31 are exactly
the upstream reference's 136 endpoints minus the 117.

Do not restate those lists here — a second copy is a second thing to drift.

## Major Missing Modules

### `api_req_combined_battle/` — Combined Fleet Battles (P0)

14 endpoints. Highest reuse value — shares battle core with `api_req_sortie/`.

- `battle`, `midnight_battle`, `sp_midnight`
- `battle_water`, `each_battle`, `each_battle_water`
- `ec_battle`, `ec_midnight_battle`, `ec_night_to_day`
- `airbattle`, `ld_airbattle`, `ld_shooting`
- `battleresult`, `goback_port`

Key challenges:
- Fleet splitting: main fleet + escort fleet composition
- Escort fleet logic in all battle phases
- `battleresult` MVP calculation across two fleets
- `goback_port` retreat mechanics

### `api_req_air_corps/` — Land-Based Air Corps (P1)

8 endpoints. Tightly coupled with map/sortie system.

- `set_plane` — assign planes to base
- `change_name` — rename base
- `change_deployment_base` — move base between map areas
- `set_action` — set sortie/defense mode
- `supply` — resupply planes
- `expand_base` — unlock new base slot
- `expand_maintenance_level` — upgrade base level
- `cond_recovery` — recover plane condition

Dependencies:
- `air_corps` gameplay trait + DB entity
- Map integration: `api_req_map/start_air_base` (sortie with LBAS)
- `api_port/airCorpsCondRecoveryWithTimer` (condition recovery on port)
- Sortie integration: LBAS strike phase in battle

## Scattered Missing Endpoints

| Endpoint | Description | Priority | Notes |
|----------|-------------|----------|-------|
| `api_req_map/start_air_base` | LBAS sortie | P1 | Implement with air_corps module |
| `api_req_map/anchorage_repair` | Emergency anchorage repair | P2 | Independent QoL feature |
| `api_req_map/air_raid` | Heavy bomber interception | P2 | Implement with combined_battle |
| `api_req_hensei/preset_lock` | Fleet preset lock | P3 | Can stub |
| `api_req_hensei/preset_order_change` | Fleet preset reorder | P3 | Can stub |
| `api_req_practice/change_matching_kind` | Practice matching mode | P3 | Can stub |
| `api_req_ranking/getlist` | Ranking list | P3 | Return empty list |
| `api_port/airCorpsCondRecoveryWithTimer` | LBAS condition recovery | P1 | Implement with air_corps module |

`api_req_ranking/mxltvkpyuklh` used to be listed here; it has been implemented
since. `api_req_kousyou/remodel_slot_recover` and `api_req_member/registration_sp`
appear in neither the router nor `docs/apilist.txt`, so they are not tracked as
gaps — add them back only with a source that says the client calls them.

## Recommended Development Roadmap

### Phase 1: Combined Fleet Battles (P0)

Reuse existing `api_req_sortie/` battle framework.

1. Add combined fleet composition types to `emukc_model`
2. Implement fleet splitting logic in `emukc_gameplay`
3. Adapt battle simulation for escort fleet phases
4. Implement `api_req_combined_battle/` handlers (14 files)
5. Verify: full event map sortie with combined fleet

### Phase 2: Equipment Improvement (P1) — done 2026-09-21

Shipped: `Codex::remodel_recipes` over the existing `slotitem_extra_info`
improvement data, `Ctx::remodel_slot{,_list,_detail}`, and the three handlers.
Rules and the success-rate table are in
`crates/emukc_model/src/codex/remodel_slot.rs`.

### Phase 3: Land-Based Air Corps (P1)

Coupled with map/sortie, implement after combined fleet.

1. Add `air_corps` gameplay trait + DB entity
2. Implement `api_req_air_corps/` handlers (8 files)
3. Add `api_req_map/start_air_base` and `api_port/airCorpsCondRecoveryWithTimer`
4. Integrate LBAS strike phase into battle simulation
5. Verify: deploy LBAS → sortie → verify air strike phase

### Phase 4: Scattered Endpoints (P2–P3)

Low-priority stubs and QoL features.

- `api_req_map/anchorage_repair` (P2)
- `api_req_map/air_raid` (P2)
- `api_req_hensei/preset_lock`, `preset_order_change` (P3)
- `api_req_practice/change_matching_kind` (P3)
- `api_req_ranking/getlist` (P3, return empty)

## Relation to Existing Plan

This analysis extends the gap tracking in `docs/plan.md` (Gap #7: Combined fleet / LBAS / support). Phase 1–3 here correspond to Track 4 (Advanced Battle Topologies) in the existing plan.
