# API Check List

## Implemented APIs

`api_req_combined_battle/` serves 味方連合 vs 敵通常艦隊 in full: the day battle in
both 編成, the aerial and long-distance cells, the night-start cell, the night
battle, the result and the return to port. The `api_req_sortie/` and
`api_req_battle_midnight/` twins cover the single-fleet simulation.

```plain
api_dmm_payment/paycheck

api_get_member/base_air_corps
api_get_member/basic
api_get_member/chart_additional_info
api_get_member/deck
api_get_member/furniture
api_get_member/kdock
api_get_member/mapinfo
api_get_member/material
api_get_member/mission
api_get_member/ndock
api_get_member/payitem
api_get_member/picture_book
api_get_member/practice
api_get_member/preset_deck
api_get_member/preset_dev_items
api_get_member/preset_slot
api_get_member/questlist
api_get_member/record
api_get_member/require_info
api_get_member/ship2
api_get_member/ship3
api_get_member/ship_deck
api_get_member/slot_item
api_get_member/sortie_conditions
api_get_member/unsetslot
api_get_member/useitem

api_port/port

api_req_air_corps/change_deployment_base
api_req_air_corps/set_plane

api_req_battle_midnight/battle
api_req_battle_midnight/sp_midnight

api_req_combined_battle/airbattle
api_req_combined_battle/battle
api_req_combined_battle/battle_water
api_req_combined_battle/battleresult
api_req_combined_battle/goback_port
api_req_combined_battle/ld_airbattle
api_req_combined_battle/ld_shooting
api_req_combined_battle/midnight_battle
api_req_combined_battle/sp_midnight

api_req_furniture/buy
api_req_furniture/change
api_req_furniture/music_list
api_req_furniture/music_play
api_req_furniture/radio_play
api_req_furniture/set_portbgm

api_req_hensei/change
api_req_hensei/combined
api_req_hensei/lock
api_req_hensei/preset_delete
api_req_hensei/preset_expand
api_req_hensei/preset_register
api_req_hensei/preset_select

api_req_hokyu/charge

api_req_init/firstship
api_req_init/nickname

api_req_kaisou/can_preset_slot_select
api_req_kaisou/hangar_expand
api_req_kaisou/lock
api_req_kaisou/marriage
api_req_kaisou/open_exslot
api_req_kaisou/powerup
api_req_kaisou/preset_slot_delete
api_req_kaisou/preset_slot_expand
api_req_kaisou/preset_slot_register
api_req_kaisou/preset_slot_select
api_req_kaisou/preset_slot_update_exslot_flag
api_req_kaisou/preset_slot_update_lock
api_req_kaisou/preset_slot_update_name
api_req_kaisou/remodeling
api_req_kaisou/slot_deprive
api_req_kaisou/slot_exchange_index
api_req_kaisou/slotset
api_req_kaisou/slotset_ex
api_req_kaisou/unsetslot_all

api_req_kousyou/createitem
api_req_kousyou/createship
api_req_kousyou/createship_speedchange
api_req_kousyou/destroyitem2
api_req_kousyou/destroyship
api_req_kousyou/getship
api_req_kousyou/open_new_dock
api_req_kousyou/preset_dev_items_delete
api_req_kousyou/preset_dev_items_expand
api_req_kousyou/preset_dev_items_register
api_req_kousyou/preset_dev_items_update_name
api_req_kousyou/remodel_slot
api_req_kousyou/remodel_slot_recover
api_req_kousyou/remodel_slotlist
api_req_kousyou/remodel_slotlist_detail

api_req_map/next
api_req_map/select_eventmap_rank
api_req_map/start

api_req_member/get_event_selected_reward
api_req_member/get_incentive
api_req_member/get_practice_enemyinfo
api_req_member/itemuse
api_req_member/itemuse_cond
api_req_member/payitemuse
api_req_member/set_flagship_position
api_req_member/set_friendly_request
api_req_member/set_option_setting
api_req_member/set_oss_condition
api_req_member/update_tutorial_progress
api_req_member/updatecomment
api_req_member/updatedeckname

api_req_mission/result
api_req_mission/return_instruction
api_req_mission/start

api_req_nyukyo/open_new_dock
api_req_nyukyo/speedchange
api_req_nyukyo/start

api_req_practice/battle
api_req_practice/battle_result
api_req_practice/midnight_battle

api_req_quest/clearitemget
api_req_quest/start
api_req_quest/stop

api_req_ranking/mxltvkpyuklh

api_req_sortie/airbattle
api_req_sortie/battle
api_req_sortie/battleresult
api_req_sortie/goback_port
api_req_sortie/ld_airbattle
api_req_sortie/ld_shooting

api_start2/get_option_setting
api_start2/getData

api_world/get_worldinfo
api_world/register
```

## Missing APIs (Not Yet Implemented)

### Core Battle System

The remaining combined-fleet variants are the ones where the **enemy** is also
combined (`ec_*`, `each_*`). Every 味方連合 vs 敵通常艦隊 cell is implemented.

```plain
api_req_combined_battle/each_battle
api_req_combined_battle/each_battle_water
api_req_combined_battle/ec_battle
api_req_combined_battle/ec_midnight_battle
api_req_combined_battle/ec_night_to_day
```

### Map & Sortie System

```plain
api_req_map/air_raid
api_req_map/anchorage_repair
api_req_map/start_air_base
```

### Practice System

```plain
api_req_practice/change_matching_kind
```

### Air Corps System

```plain
api_port/airCorpsCondRecoveryWithTimer
api_req_air_corps/set_action
api_req_air_corps/supply
api_req_air_corps/change_name
api_req_air_corps/expand_base
api_req_air_corps/expand_maintenance_level
api_req_air_corps/cond_recovery
```

### Fleet Preset

```plain
api_req_hensei/preset_lock
api_req_hensei/preset_order_change
```

### Ranking

```plain
api_req_ranking/getlist
```

## Implementation Priority

### High Priority (Core Gameplay)

1. **Combined Fleet Battles** - the 5 remaining 敵連合 `api_req_combined_battle/*` endpoints
2. **Air Corps System** - event map support, and the only remaining `api_get_member` gap

### Medium Priority (Enhanced Features)

1. **Map & Sortie remainder** - air_raid, anchorage_repair, start_air_base

### Low Priority (Optional Features)

1. **Fleet Presets** - QoL enhancements (preset_lock, preset_order_change)
2. **Practice matching** - change_matching_kind
3. **Ranking** - Ranking list display
