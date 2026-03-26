# TODO

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
- [ ] implement combined fleet handler (`api_req_hensei/combined`)
- [ ] update quest progress on port entry (`api_port/port.rs`)
- [ ] fix naming confusion in `net/assets/mod.rs`
- [ ] remove all profile data on account deletion (`user/account.rs`)
- [ ] add more codex limitations (`codex/mod.rs`)
- [ ] add more DB entity relations (`entity/profile/mod.rs`)
- [ ] review tsunkit quest parser edge cases (`parser/tsunkit_quest/types.rs`)
- [ ] practice system: implement opponent fleet generation (`game/practice.rs`)
- [ ] ship ops: replace temporary implementation (`game/ship/mod.rs`)
- [ ] migrate off deprecated `axum_extra::extract::Host` in `net/router/game.rs`

## High Priority - Core Gameplay
- [ ] **Map & Sortie System** (`api_req_map/*`)
  - [ ] `api_req_map/start` - sortie start
  - [ ] `api_req_map/next` - advance to next node
  - [ ] `api_req_map/select_eventmap_rank` - event difficulty select
  - [ ] `api_req_map/air_raid` - air raid on base
  - [ ] `api_req_map/anchorage_repair` - anchorage repair
  - [ ] `api_req_map/start_air_base` - air base sortie
- [ ] **Battle System** (`api_req_sortie/*`, `api_req_battle_midnight/*`, `api_req_combined_battle/*`)
  - [ ] `api_req_sortie/battle` - normal day battle
  - [ ] `api_req_sortie/battleresult` - battle result
  - [ ] `api_req_sortie/airbattle` - aerial battle
  - [ ] `api_req_sortie/ld_airbattle` - long-distance aerial battle
  - [ ] `api_req_sortie/ld_shooting` - long-distance shelling
  - [ ] `api_req_sortie/goback_port` - retreat
  - [ ] `api_req_battle_midnight/battle` - night battle
  - [ ] `api_req_battle_midnight/sp_midnight` - night-to-day battle
  - [ ] Combined battle variants (14 endpoints)
- [ ] **Mission / Expedition System** (`api_req_mission/*`)
  - [ ] `api_req_mission/start` - start expedition
  - [ ] `api_req_mission/result` - expedition result
  - [ ] `api_req_mission/return_instruction` - recall expedition
  - [ ] Static expedition unlock table from verified external data
    Priority: low until a reliable structured data source is available

## Medium Priority - Enhanced Features
- [ ] **Practice System** (`api_req_practice/*`)
  - [ ] `api_req_practice/battle` - practice battle
  - [ ] `api_req_practice/battle_result` - practice result
  - [ ] `api_req_practice/midnight_battle` - practice night battle
  - [ ] `api_req_practice/change_matching_kind` - change matching type
- [ ] **Air Corps System** (`api_req_air_corps/*`)
  - [ ] `api_get_member/base_air_corps` - air corps data
  - [ ] `api_port/airCorpsCondRecoveryWithTimer` - condition recovery
  - [ ] `api_req_air_corps/set_plane` - assign planes
  - [ ] `api_req_air_corps/set_action` - set action (standby/sortie/defense)
  - [ ] `api_req_air_corps/supply` - resupply planes
  - [ ] `api_req_air_corps/change_name` - rename squadron
  - [ ] Other air corps management (4 endpoints)
- [ ] **Equipment Improvement / Akashi Arsenal** (`api_req_kousyou/remodel_*`)
  - [ ] `api_req_kousyou/remodel_slotlist` - improvement candidate list
  - [ ] `api_req_kousyou/remodel_slotlist_detail` - improvement detail
  - [ ] `api_req_kousyou/remodel_slot` - perform improvement

## Low Priority - Optional
- [ ] `api_req_hensei/preset_lock` - lock fleet preset
- [ ] `api_req_hensei/preset_order_change` - reorder fleet presets
- [ ] `api_req_ranking/getlist` - ranking list display
- [ ] `api_dmm_payment/paycheck` - payment (not needed for emulator)
