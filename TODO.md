# TODO

> Endpoint coverage is not tracked here. `apilist.md` holds the implemented and
> missing lists, derived mechanically from the router; `docs/api_coverage.md`
> holds the roadmap. Last cross-checked 2026-10-09: 143 implemented, 5 missing.

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
- [x] recalculate quest progress on `start` the quest
- [x] quest `api_voice_id`: `1000 + quest no` for the twelve quests with a line in `kc9999` (`game/view/quest_list.rs`)
- [x] quest `api_invalid_flag`: 1 when a conversion's flagship equipment is locked (`game/quest/holding.rs`)
- [x] quest `api_c_list`: not needed — the server judges a conversion quest's holdings itself and sends `api_state` 3 (`questlist.rs`)
- [x] implement slotitem consumption for quest reward claim (`game/quest/holding.rs`)
- [x] implement combined fleet handler (`api_req_hensei/combined`)
- [ ] update quest progress on port entry (`game/view/port.rs`) — probably unnecessary: quests are refreshed when the quest list is read
- [ ] fix naming confusion in `net/assets/mod.rs`
- [ ] remove all profile data on account deletion (`user/account.rs`) — only the `profile` row goes; child tables keep orphans
- [ ] practice system: implement opponent fleet generation (`game/practice.rs`) — five template rivals with one level-180 ship each for now
- [ ] single-fleet day shelling gives each round to one side (`execute_shelling1` / `execute_shelling2` in `emukc_battle/src/simulation/mod.rs`): the side that fires second never shells unless a battleship opens a second round. The combined-fleet paths already run both sides in every round
- [ ] `calculate_single_slot_airstrike_damage` (`emukc_battle/src/damage.rs`) is dead code kept for an airstrike phase that never used it
- [x] migrate off deprecated `axum_extra::extract::Host` in `net/router/game.rs`

## High Priority - Core Gameplay
- [ ] **Map & Sortie System** (`api_req_map/*`)
  - [x] `api_req_map/start` - sortie start
  - [x] `api_req_map/next` - advance to next node (with non-battle node effects)
  - [x] `api_req_map/select_eventmap_rank` - event difficulty select
  - [x] Non-battle node effects: resource acquisition, maelstrom (渦潮) with radar reduction
  - [x] Battle damage persistence: ship HP updated after battle result
  - [x] Sortie resource consumption: fuel/ammo per battle node
  - [x] air raid on the air base during a sortie (`api_destruction_battle` in `api_req_map/next`; 6-5 only)
  - [ ] `api_req_map/air_raid` - 超重爆迎撃, event maps only
  - [ ] `api_req_map/anchorage_repair` - anchorage repair
  - [x] `api_req_map/start_air_base` - air base sortie
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
  - [ ] 敵連合 variants (3 remaining: `each_battle`, `each_battle_water`, `ec_night_to_day`) - blocked on event map data
- [ ] **Mission / Expedition System** (`api_req_mission/*`)
  - [x] `api_req_mission/start` - start expedition
  - [x] `api_req_mission/result` - expedition result
  - [x] `api_req_mission/return_instruction` - recall expedition
  - [ ] Static expedition unlock table from verified external data
    Priority: low until a reliable structured data source is available

## Medium Priority - Enhanced Features
- [x] **Practice System** (`api_req_practice/*`)
  - [x] `api_req_practice/battle` - practice battle
  - [x] `api_req_practice/battle_result` - practice result
  - [x] `api_req_practice/midnight_battle` - practice night battle
  - [x] `api_req_practice/change_matching_kind` - change matching type
- [x] **Air Corps System, port side** (`api_req_air_corps/*`)
  - [x] `api_get_member/base_air_corps` - air corps data
  - [x] `api_port/airCorpsCondRecoveryWithTimer` - condition recovery with time
  - [x] `api_req_air_corps/set_plane` - assign planes
  - [x] `api_req_air_corps/set_action` - set action (standby/sortie/defense)
  - [x] `api_req_air_corps/supply` - resupply planes
  - [x] `api_req_air_corps/change_deployment_base` - move a squadron between bases
  - [x] `api_req_air_corps/change_name` - rename squadron
  - [x] `api_req_air_corps/{expand_base,expand_maintenance_level,cond_recovery}` - add an air corps, raise 整備Lv, rest
- [x] **Equipment Improvement / Akashi Arsenal** (`api_req_kousyou/remodel_*`)
  - [x] `api_req_kousyou/remodel_slotlist` - improvement candidate list
  - [x] `api_req_kousyou/remodel_slotlist_detail` - improvement detail
  - [x] `api_req_kousyou/remodel_slot` - perform improvement
  - [x] `api_req_kousyou/remodel_slot_recover` - reset improvement level

## Low Priority - Optional
- [x] `api_req_hensei/preset_lock` - lock fleet preset
- [x] `api_req_hensei/preset_order_change` - reorder fleet presets
- [x] `api_req_ranking/getlist` - dropped: the client no longer calls it (`mxltvkpyuklh` is served)
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
