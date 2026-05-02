//! Practice battle API response construction.

use emukc_battle::{BattleRuntimeShip, BattleShipInput, NightBattlePacket};

use super::{PracticeBattleResultResponse, PracticeBattleSession, PracticeNightBattleResponse};

/// Map a ship's slot items to the 5-element API array.
///
/// Prefers actual slot contents over slot-item IDs when slots are populated.
pub(crate) fn enemy_slot_ids(ship: &BattleRuntimeShip) -> [i64; 5] {
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

fn enemy_slot_ids_from_input(ship: &BattleShipInput) -> [i64; 5] {
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

/// Build the practice battle result API response from a snapshot.
pub fn build_result_response(
    snapshot: super::PracticeBattleResultSnapshot,
) -> PracticeBattleResultResponse {
    PracticeBattleResultResponse {
        api_ship_id: snapshot.enemy_ship_ids,
        api_win_rank: snapshot.win_rank,
        api_get_exp: snapshot.get_exp,
        api_member_lv: snapshot.member_lv,
        api_member_exp: snapshot.member_exp,
        api_get_base_exp: snapshot.get_base_exp,
        api_mvp: snapshot.mvp,
        api_get_ship_exp: snapshot.get_ship_exp,
        api_get_exp_lvup: snapshot.get_exp_lvup,
        api_enemy_info: super::PracticeBattleEnemyInfo {
            api_user_name: String::new(),
            api_level: snapshot.enemy_level,
            api_rank: snapshot.enemy_rank,
            api_deck_name: snapshot.enemy_deck_name,
        },
    }
}

pub(crate) fn build_night_response(
    session: &PracticeBattleSession,
    packet: &NightBattlePacket,
) -> PracticeNightBattleResponse {
    PracticeNightBattleResponse {
        api_deck_id: session.deck_id,
        api_formation: packet.formation,
        api_f_nowhps: packet.friendly_nowhps.clone(),
        api_f_maxhps: packet.friendly_maxhps.clone(),
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
        api_e_nowhps: packet.enemy_nowhps.clone(),
        api_e_maxhps: packet.enemy_maxhps.clone(),
        api_eSlot: session
            .enemy
            .iter()
            .map(|ship| {
                enemy_slot_ids_from_input(&BattleShipInput {
                    ship: ship.ship.clone(),
                    slot_items: ship.slot_items.clone(),
                    effect_list: ship.effect_list.clone(),
                    married: false,
                })
            })
            .collect(),
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
        api_hougeki: packet.hougeki.clone(),
    }
}

/// Calculate practice base experience from rival level.
pub fn calculate_base_exp(rival: &emukc_model::profile::practice::Rival) -> i64 {
    (rival.level.max(1) * 9).clamp(100, 1200)
}
