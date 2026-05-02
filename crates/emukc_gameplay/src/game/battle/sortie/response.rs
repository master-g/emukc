//! Sortie battle API response construction.

use emukc_battle::{BattlePacket, BattleShipInput, NightBattlePacket};

use super::super::practice::PracticeBattleResponse;
use super::SortieBattleSession;
use crate::game::sortie::SortieNightBattleResponse;

/// Map a ship's slot items to the 5-element array expected by the API.
pub(crate) fn enemy_slot_ids(ship: &BattleShipInput) -> [i64; 5] {
    if ship.ship.api_slot.iter().any(|slot| *slot > 0) {
        let mut slots = [-1; 5];
        for (idx, slot) in ship.ship.api_slot.iter().take(5).enumerate() {
            if *slot > 0 {
                slots[idx] = *slot;
            }
        }
        return slots;
    }
    let mut slots = [-1; 5];
    for (idx, slot_item) in ship.slot_items.iter().take(5).enumerate() {
        slots[idx] = slot_item.api_slotitem_id;
    }
    slots
}

/// Build a sortie day-battle API response.
pub fn build_day_response(
    deck_id: i64,
    friend_ships: Vec<BattleShipInput>,
    enemy_ships: Vec<BattleShipInput>,
    packet: BattlePacket,
) -> PracticeBattleResponse {
    PracticeBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: friend_ships.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_f_maxhps: friend_ships.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_fParam: friend_ships
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_ship_ke: enemy_ships.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy_ships.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: enemy_ships.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_e_maxhps: enemy_ships.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_eSlot: enemy_ships.iter().map(enemy_slot_ids).collect(),
        api_eParam: enemy_ships
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_e_effect_list: enemy_ships
            .iter()
            .map(|ship| {
                if ship.effect_list.is_empty() {
                    vec![0]
                } else {
                    ship.effect_list.clone()
                }
            })
            .collect(),
        api_smoke_type: packet.smoke_type,
        api_balloon_cell: packet.balloon_cell,
        api_atoll_cell: packet.atoll_cell,
        api_midnight_flag: packet.midnight_flag,
        api_search: packet.search,
        api_stage_flag: packet.stage_flag,
        api_kouku: packet.kouku,
        api_opening_taisen_flag: packet.opening_taisen_flag,
        api_opening_taisen: packet.opening_taisen,
        api_opening_flag: packet.opening_flag,
        api_opening_atack: packet.opening_attack,
        api_hourai_flag: packet.hourai_flag,
        api_hougeki1: packet.hougeki1,
        api_hougeki2: packet.hougeki2,
        api_hougeki3: packet.hougeki3,
        api_raigeki: packet.raigeki,
    }
}

/// Build a sortie night-battle API response.
pub fn build_night_response(
    deck_id: i64,
    session: &SortieBattleSession,
    packet: NightBattlePacket,
) -> SortieNightBattleResponse {
    SortieNightBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: packet.friendly_nowhps,
        api_f_maxhps: packet.friendly_maxhps,
        api_fParam: session
            .friendly
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_ship_ke: session.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: session.enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: packet.enemy_nowhps,
        api_e_maxhps: packet.enemy_maxhps,
        api_eSlot: session.enemy.iter().map(super::super::practice::enemy_slot_ids).collect(),
        api_eParam: session
            .enemy
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_smoke_type: 0,
        api_balloon_cell: 0,
        api_atoll_cell: 0,
        api_touch_plane: packet.touch_plane,
        api_flare_pos: packet.flare_pos,
        api_hougeki: packet.hougeki,
    }
}
