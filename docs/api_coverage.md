# KCSAPI Handler Coverage Analysis

> Last updated: 2026-05-24
> Reference: [sinsinpub/kcs2-assets apilist.txt](https://github.com/sinsinpub/kcs2-assets/blob/master/api_info/apilist.txt)

## Summary

~85% endpoint coverage by count. Three major modules entirely missing, plus scattered individual endpoints.

## Implemented Modules

| Module | Endpoints | Status |
|--------|-----------|--------|
| `api_start2/` | 3 | Complete |
| `api_get_member/` | 26 | Mostly complete |
| `api_port/` | 2 | Missing `airCorpsCondRecoveryWithTimer` |
| `api_req_battle_midnight/` | 3 | Complete |
| `api_req_furniture/` | 7 | Complete |
| `api_req_hensei/` | 8 | Missing `preset_lock`, `preset_order_change` |
| `api_req_hokyu/` | 2 | Complete |
| `api_req_init/` | 3 | Complete |
| `api_req_kaisou/` | 18 | Complete |
| `api_req_kousyou/` | 11 | Missing `remodel_slot*` series |
| `api_req_map/` | 5 | Missing `start_air_base`, `anchorage_repair`, `air_raid` |
| `api_req_member/` | 14 | Missing `registration_sp` |
| `api_req_mission/` | 4 | Complete |
| `api_req_nyukyo/` | 4 | Complete |
| `api_req_practice/` | 4 | Missing `change_matching_kind` |
| `api_req_quest/` | 4 | Complete |
| `api_req_ranking/` | 1 | Missing `getlist`, `mxltvkpyuklh` |
| `api_req_sortie/` | 7 | Complete |
| `api_dmm_payment/` | 1 | Complete |
| `api_world/` | 3 | Complete |

## Major Missing Modules

### `api_req_combined_battle/` — Combined Fleet Battles (P0)

~15 endpoints. Highest reuse value — shares battle core with `api_req_sortie/`.

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

### `api_req_kousyou/remodel_slot*` — Equipment Improvement (P1)

4 endpoints. Independent module, self-contained logic.

- `remodel_slotlist` — list improvable equipment
- `remodel_slotlist_detail` — improvement details for selected item
- `remodel_slot` — execute improvement
- `remodel_slot_recover` — cancel/rollback improvement

Dependencies:
- `remodel_slot` gameplay trait
- Possible codex extension for improvement recipe data

### `api_req_air_corps/` — Land-Based Air Corps (P1)

~8 endpoints. Tightly coupled with map/sortie system.

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
| `api_req_member/registration_sp` | Pre-registration | P3 | Can stub |
| `api_req_ranking/getlist` | Ranking list | P3 | Return empty list |
| `api_req_ranking/mxltvkpyuklh` | Ranking (obfuscated) | P3 | Return empty list |
| `api_port/airCorpsCondRecoveryWithTimer` | LBAS condition recovery | P1 | Implement with air_corps module |

## Recommended Development Roadmap

### Phase 1: Combined Fleet Battles (P0)

Reuse existing `api_req_sortie/` battle framework.

1. Add combined fleet composition types to `emukc_model`
2. Implement fleet splitting logic in `emukc_gameplay`
3. Adapt battle simulation for escort fleet phases
4. Implement `api_req_combined_battle/` handlers (~15 files)
5. Verify: full event map sortie with combined fleet

### Phase 2: Equipment Improvement (P1)

Independent module, can parallelize with Phase 3.

1. Add `remodel_slot` gameplay trait
2. Add improvement recipe data to codex (if needed)
3. Implement 4 handlers under `api_req_kousyou/`
4. Verify: improve equipment → verify stat changes persist

### Phase 3: Land-Based Air Corps (P1)

Coupled with map/sortie, implement after combined fleet.

1. Add `air_corps` gameplay trait + DB entity
2. Implement `api_req_air_corps/` handlers (~8 files)
3. Add `api_req_map/start_air_base` and `api_port/airCorpsCondRecoveryWithTimer`
4. Integrate LBAS strike phase into battle simulation
5. Verify: deploy LBAS → sortie → verify air strike phase

### Phase 4: Scattered Endpoints (P2–P3)

Low-priority stubs and QoL features.

- `api_req_map/anchorage_repair` (P2)
- `api_req_map/air_raid` (P2)
- `api_req_hensei/preset_lock`, `preset_order_change` (P3)
- `api_req_practice/change_matching_kind` (P3)
- `api_req_ranking/getlist`, `mxltvkpyuklh` (P3, return empty)
- `api_req_member/registration_sp` (P3, stub)

## Relation to Existing Plan

This analysis extends the gap tracking in `docs/plan.md` (Gap #7: Combined fleet / LBAS / support). Phase 1–3 here correspond to Track 4 (Advanced Battle Topologies) in the existing plan.
