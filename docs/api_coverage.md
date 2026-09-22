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
6.3.5.0: **126 implemented**, **22 missing**, no overlap, and the 22 are exactly
the upstream reference's 136 endpoints minus the 126.

Do not restate those lists here — a second copy is a second thing to drift.

## Major Missing Modules

### `api_req_combined_battle/` — Combined Fleet Battles (P0)

5 endpoints left, all of them 敵連合艦隊. Every 味方連合 vs 敵通常艦隊 cell
shipped 2026-09-21: `battle`, `battle_water`, `airbattle`, `ld_airbattle`,
`ld_shooting`, `sp_midnight`, `midnight_battle`, `battleresult`, `goback_port`.

- `each_battle`, `each_battle_water`, `ec_battle`, `ec_midnight_battle`,
  `ec_night_to_day` — the enemy is combined too, which changes phase order,
  night opponent selection and the correction table

Key challenges left:
- Enemy-side fleet splitting, and the night opponent score
  (`docs/battle/combined-fleet-reference.md` §Night battle opponent selection)
- The 連合 vs 連合 correction table, which upstream marks 要検証
- **No data to drive it.** The bootstrapped `map_catalog.json` holds 37 regular
  maps (1-1..7-5) and not one enemy composition longer than six ships, so no
  cell in the local codex can produce an enemy combined fleet. Enemy combined
  fleets exist only on event maps. Until event map data is available these five
  endpoints cannot be exercised end to end, only unit-tested against hand-built
  fixtures.

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
since. `api_req_member/registration_sp` appears in neither the router nor
`docs/apilist.txt`, and `grep` over the decoded client finds nothing, so it is
still not tracked as a gap.

`api_req_kousyou/remodel_slot_recover` **is** a real gap, found 2026-09-21 while
implementing the arsenal. It is absent from `docs/apilist.txt` — that reference
predates the feature — but the decoded client declares a full API class for it
(`main.decoded.js:105351`) posting `api_menu_id`, `api_slot_id` and
`api_dev_num`, and the official account announced it as 【装備改修度】の状態回復:
spend 工廠資源 x1 plus 開発資材 x1–3 to restore an equipment's improvement
state. It is deliberately left out of `apilist.md`, whose missing list is
defined as `docs/apilist.txt` minus the router and must stay mechanically
derivable; track it here until that reference is refreshed.

## Recommended Development Roadmap

### Phase 1: Combined Fleet Battles (P0)

Reuse existing `api_req_sortie/` battle framework.

1. Add combined fleet composition types to `emukc_model` — done
2. Implement fleet splitting logic in `emukc_gameplay` — done
3. Adapt battle simulation for escort fleet phases — done for 敵通常艦隊
4. Implement `api_req_combined_battle/` handlers — 5 of 14 done
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
