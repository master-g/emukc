# TODO

> Endpoint coverage is not tracked here. `apilist.md` holds the implemented and
> missing lists, derived mechanically from the router; `docs/api_coverage.md`
> holds the roadmap. Last cross-checked 2026-09-22: 130 implemented, 19 missing.

## Completed
- [x] impl incentive gameplay and api
- [x] material cap is buggy
- [x] find out what the remodel fields mean in kcwiki ship.json
- [x] remove old kc3rd ship model
- [x] dock, port ops traits and init
- [x] require_info api impl
- [x] material self replenish logic
- [x] replenish ship condition
- [x] better cache list making
- [x] kache sucks, rewrite it
- [x] rewrite async for with `StreamExt` and `FutureExt`

## Code Quality / Tech Debt
- [ ] recalculate quest progress on `start` the quest
- [ ] quest `api_voice_id` field is missing (`questlist.rs`)
- [ ] quest `api_invalid_flag` field is missing, e.g. plane convert quests (`questlist.rs`)
- [ ] quest `api_c_list` (composition quest list) not populated (`questlist.rs`)
- [ ] implement slotitem consumption for quest reward claim (`game/quest/consume.rs`)
- [x] implement combined fleet handler (`api_req_hensei/combined`)
- [ ] update quest progress on port entry (`api_port/port.rs`)
- [ ] fix naming confusion in `net/assets/mod.rs`
- [ ] remove all profile data on account deletion (`user/account.rs`)
- [ ] add more codex limitations (`codex/mod.rs`)
- [ ] add more DB entity relations (`entity/profile/mod.rs`)
- [ ] review tsunkit quest parser edge cases (`parser/tsunkit_quest/types.rs`)
- [ ] practice system: implement opponent fleet generation (`game/practice.rs`)
- [ ] ship ops: replace temporary implementation (`game/ship/mod.rs`)
- [x] migrate off deprecated `axum_extra::extract::Host` in `net/router/game.rs`

## High Priority - Core Gameplay
- [ ] **Map & Sortie System** (`api_req_map/*`)
  - [x] `api_req_map/start` - sortie start
  - [x] `api_req_map/next` - advance to next node (with non-battle node effects)
  - [x] `api_req_map/select_eventmap_rank` - event difficulty select
  - [x] Non-battle node effects: resource acquisition, maelstrom (渦潮) with radar reduction
  - [x] Battle damage persistence: ship HP updated after battle result
  - [x] Sortie resource consumption: fuel/ammo per battle node
  - [ ] `api_req_map/air_raid` - air raid on base
  - [ ] `api_req_map/anchorage_repair` - anchorage repair
  - [ ] `api_req_map/start_air_base` - air base sortie
- [ ] **Battle System** (`api_req_sortie/*`, `api_req_battle_midnight/*`, `api_req_combined_battle/*`)
  - [x] `api_req_sortie/battle` - normal day battle
  - [x] `api_req_sortie/battleresult` - battle result
  - [x] `api_req_sortie/airbattle` - aerial battle
  - [x] `api_req_sortie/ld_airbattle` - long-distance aerial battle
  - [x] `api_req_sortie/ld_shooting` - long-distance shelling
  - [x] `api_req_sortie/goback_port` - retreat
  - [x] `api_req_battle_midnight/battle` - night battle
  - [x] `api_req_battle_midnight/sp_midnight` - night-start battle
  - [x] `api_req_combined_battle/{battle,battle_water,airbattle,ld_airbattle,ld_shooting,sp_midnight,midnight_battle}` - 味方連合 vs 敵通常
  - [ ] 敵連合 variants (5 remaining: `ec_*`, `each_*`) - blocked on event map data
- [ ] **Mission / Expedition System** (`api_req_mission/*`)
  - [x] `api_req_mission/start` - start expedition
  - [x] `api_req_mission/result` - expedition result
  - [x] `api_req_mission/return_instruction` - recall expedition
  - [ ] Static expedition unlock table from verified external data
    Priority: low until a reliable structured data source is available

## Medium Priority - Enhanced Features
- [ ] **Practice System** (`api_req_practice/*`)
  - [x] `api_req_practice/battle` - practice battle
  - [x] `api_req_practice/battle_result` - practice result
  - [x] `api_req_practice/midnight_battle` - practice night battle
  - [ ] `api_req_practice/change_matching_kind` - change matching type
- [ ] **Air Corps System** (`api_req_air_corps/*`)
  - [x] `api_get_member/base_air_corps` - air corps data
  - [ ] `api_port/airCorpsCondRecoveryWithTimer` - condition recovery
  - [x] `api_req_air_corps/set_plane` - assign planes
  - [ ] `api_req_air_corps/set_action` - set action (standby/sortie/defense)
  - [ ] `api_req_air_corps/supply` - resupply planes
  - [x] `api_req_air_corps/change_deployment_base` - move a squadron between bases
  - [ ] `api_req_air_corps/change_name` - rename squadron
  - [ ] Other air corps management (4 endpoints)
- [x] **Equipment Improvement / Akashi Arsenal** (`api_req_kousyou/remodel_*`)
  - [x] `api_req_kousyou/remodel_slotlist` - improvement candidate list
  - [x] `api_req_kousyou/remodel_slotlist_detail` - improvement detail
  - [x] `api_req_kousyou/remodel_slot` - perform improvement
  - [x] `api_req_kousyou/remodel_slot_recover` - reset improvement level

## Low Priority - Optional
- [ ] `api_req_hensei/preset_lock` - lock fleet preset
- [ ] `api_req_hensei/preset_order_change` - reorder fleet presets
- [ ] `api_req_ranking/getlist` - ranking list display
- [x] `api_dmm_payment/paycheck` - payment (stub returning check_value 1)

## Ideas / Not Scheduled
- [ ] **Live-account snapshot** — mirror the owner's own KanColle account into a local profile by
  calling the official API with their own session token.
  - [x] Read-only capture — `scripts/fetch_live_api.py` pulls the verified read-only set, including
    the signed `api_port/port`, into a gitignored snapshot. Order of operations and the shape traps
    it cost to learn: `docs/solutions/best-practices/live-api-investigation.md`.
  - [ ] Read a snapshot back as an emukc profile. The capture already reproduces the whole save
    (340 ships, 1855 items, air corps, quests); nothing imports it yet.
  - [ ] Turn successive captures into a timeline rather than separate dumps — the 电子骨灰盒 part,
    so the account stays readable after the service is gone.
  - [ ] Sortieing on the live account stays a separate decision: it is the only irreversible part
    and buys exactly one thing the read path cannot — genuine battle responses to check the
    simulator against. Practice battles give the same shape with no sinking risk; do those first.
  - Answered by the 2026-09-22 round: the token lives exactly as long as the game page, so a
    capture is one page-open round rather than a background sync; it never touches the repo (env
    var in, `z/snapshot/` out, both gitignored).
