//! Battle API response wire structs and their builders.
//!
//! Sortie and practice emit the same two wire shapes, so both the structs and
//! the packet-to-wire translation live here once.

use serde::Serialize;

use emukc_battle::{
    BattleHougeki, BattleKouku, BattleNightHougeki, BattleOpeningAttack, BattlePacket,
    BattleRaigeki, BattleRuntimeShip, BattleShipInput, NightBattlePacket,
};
use emukc_model::kc2::{KcApiShip, KcApiSlotItem};

/// Day battle API response.
#[expect(non_snake_case)]
#[derive(Debug, Clone, Serialize)]
pub struct DayBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    pub api_ship_ke: Vec<i64>,
    pub api_ship_lv: Vec<i64>,
    pub api_e_nowhps: Vec<i64>,
    pub api_e_maxhps: Vec<i64>,
    pub api_eSlot: Vec<[i64; 5]>,
    pub api_eParam: Vec<[i64; 4]>,
    pub api_e_effect_list: Vec<Vec<i64>>,
    pub api_smoke_type: i64,
    pub api_balloon_cell: i64,
    pub api_atoll_cell: i64,
    pub api_midnight_flag: i64,
    pub api_search: [i64; 2],
    pub api_stage_flag: [i64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_kouku: Option<BattleKouku>,
    pub api_opening_taisen_flag: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_opening_taisen: Option<BattleHougeki>,
    pub api_opening_flag: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_opening_atack: Option<BattleOpeningAttack>,
    pub api_hourai_flag: [i64; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki1: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki2: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki3: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_raigeki: Option<BattleRaigeki>,
}

/// Night battle API response.
#[expect(non_snake_case)]
#[derive(Debug, Clone, Serialize)]
pub struct NightBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    pub api_ship_ke: Vec<i64>,
    pub api_ship_lv: Vec<i64>,
    pub api_e_nowhps: Vec<i64>,
    pub api_e_maxhps: Vec<i64>,
    pub api_eSlot: Vec<[i64; 5]>,
    pub api_eParam: Vec<[i64; 4]>,
    pub api_smoke_type: i64,
    pub api_balloon_cell: i64,
    pub api_atoll_cell: i64,
    pub api_touch_plane: [i64; 2],
    pub api_flare_pos: [i64; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki: Option<BattleNightHougeki>,
}

/// Map a ship's slot items to the 5-element array expected by the API.
///
/// Prefers actual slot contents over slot-item IDs when slots are populated.
pub(crate) fn enemy_slot_ids(ship: &KcApiShip, slot_items: &[KcApiSlotItem]) -> [i64; 5] {
    if ship.api_slot.iter().any(|slot| *slot > 0) {
        let mut slots = [-1; 5];
        for (idx, slot) in ship.api_slot.iter().take(5).enumerate() {
            if *slot > 0 {
                slots[idx] = *slot;
            }
        }
        return slots;
    }
    let mut slots = [-1; 5];
    for (idx, slot_item) in slot_items.iter().take(5).enumerate() {
        slots[idx] = slot_item.api_slotitem_id;
    }
    slots
}

/// The `api_fParam` / `api_eParam` quadruple: firepower, torpedo, AA, armor.
fn ship_params(ship: &KcApiShip) -> [i64; 4] {
    [ship.api_karyoku[0], ship.api_raisou[0], ship.api_taiku[0], ship.api_soukou[0]]
}

/// Build a day-battle API response.
///
/// HP arrays come from the ships as they entered the node, not from the packet.
pub fn build_day_response(
    deck_id: i64,
    friendly: &[BattleShipInput],
    enemy: &[BattleShipInput],
    packet: BattlePacket,
) -> DayBattleResponse {
    DayBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: friendly.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_f_maxhps: friendly.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_fParam: friendly.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_ship_ke: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: enemy.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_e_maxhps: enemy.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_eSlot: enemy.iter().map(|ship| enemy_slot_ids(&ship.ship, &ship.slot_items)).collect(),
        api_eParam: enemy.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_e_effect_list: enemy
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

/// Build a night-battle API response.
///
/// HP arrays come from the packet, which already reflects the night phase.
pub fn build_night_response(
    deck_id: i64,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
    packet: NightBattlePacket,
) -> NightBattleResponse {
    NightBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: packet.friendly_nowhps,
        api_f_maxhps: packet.friendly_maxhps,
        api_fParam: friendly.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_ship_ke: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: packet.enemy_nowhps,
        api_e_maxhps: packet.enemy_maxhps,
        api_eSlot: enemy.iter().map(|ship| enemy_slot_ids(&ship.ship, &ship.slot_items)).collect(),
        api_eParam: enemy.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_smoke_type: 0,
        api_balloon_cell: 0,
        api_atoll_cell: 0,
        api_touch_plane: packet.touch_plane,
        api_flare_pos: packet.flare_pos,
        api_hougeki: packet.hougeki,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ship(nowhp: i64, maxhp: i64) -> KcApiShip {
        KcApiShip {
            api_id: 1,
            api_sortno: 1,
            api_ship_id: 1,
            api_lv: 1,
            api_exp: [0, 0, 0],
            api_nowhp: nowhp,
            api_maxhp: maxhp,
            api_soku: 10,
            api_leng: 1,
            api_slot: [-1; 5],
            api_onslot: [0; 5],
            api_slot_ex: 0,
            api_onslot_max: None,
            api_kyouka: [0; 7],
            api_backs: 1,
            api_fuel: 0,
            api_bull: 0,
            api_slotnum: 4,
            api_ndock_time: 0,
            api_ndock_item: [0; 2],
            api_srate: 0,
            api_cond: 49,
            api_karyoku: [0; 2],
            api_raisou: [0; 2],
            api_taiku: [0; 2],
            api_soukou: [0; 2],
            api_kaihi: [0; 2],
            api_taisen: [0; 2],
            api_sakuteki: [0; 2],
            api_lucky: [0; 2],
            api_locked: 0,
            api_locked_equip: 0,
            api_sally_area: 0,
            api_sp_effect_items: None,
        }
    }

    fn test_slot_item(slotitem_id: i64) -> KcApiSlotItem {
        KcApiSlotItem {
            api_id: slotitem_id,
            api_slotitem_id: slotitem_id,
            api_locked: 0,
            api_level: 0,
            api_alv: None,
        }
    }

    fn test_input(
        ship: KcApiShip,
        slot_items: Vec<KcApiSlotItem>,
        effect_list: Vec<i64>,
    ) -> BattleShipInput {
        BattleShipInput {
            ship,
            slot_items,
            effect_list,
            married: false,
        }
    }

    fn test_packet() -> BattlePacket {
        BattlePacket {
            formation: [1, 1, 1],
            friendly_nowhps: vec![],
            enemy_nowhps: vec![],
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: 0,
            search: [1, 1],
            stage_flag: [0, 0, 0],
            kouku: None,
            opening_taisen_flag: 0,
            opening_taisen: None,
            opening_flag: 0,
            opening_attack: None,
            hourai_flag: [0, 0, 0, 0],
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
        }
    }

    #[test]
    fn day_response_replaces_an_empty_enemy_effect_list_with_zero() {
        let friendly = vec![test_input(test_ship(10, 10), vec![], vec![0])];
        let enemy = vec![
            test_input(test_ship(20, 20), vec![], vec![]),
            test_input(test_ship(20, 20), vec![], vec![7, 8]),
        ];

        let resp = build_day_response(1, &friendly, &enemy, test_packet());

        assert_eq!(resp.api_e_effect_list, vec![vec![0], vec![7, 8]]);
    }

    #[test]
    fn day_response_falls_back_to_slot_items_when_api_slot_is_empty() {
        let friendly = vec![test_input(test_ship(10, 10), vec![], vec![0])];
        let mut equipped = test_ship(20, 20);
        equipped.api_slot = [101, 0, -1, -1, -1];
        let enemy = vec![
            test_input(equipped, vec![test_slot_item(999)], vec![0]),
            test_input(test_ship(20, 20), vec![test_slot_item(525)], vec![0]),
        ];

        let resp = build_day_response(1, &friendly, &enemy, test_packet());

        assert_eq!(resp.api_eSlot, vec![[101, -1, -1, -1, -1], [525, -1, -1, -1, -1]]);
    }

    #[test]
    fn day_response_reports_entry_hp_not_packet_hp() {
        let friendly = vec![test_input(test_ship(31, 40), vec![], vec![0])];
        let enemy = vec![test_input(test_ship(12, 20), vec![], vec![0])];
        let mut packet = test_packet();
        packet.friendly_nowhps = vec![3];
        packet.enemy_nowhps = vec![0];

        let resp = build_day_response(1, &friendly, &enemy, packet);

        assert_eq!(resp.api_f_nowhps, vec![31]);
        assert_eq!(resp.api_f_maxhps, vec![40]);
        assert_eq!(resp.api_e_nowhps, vec![12]);
        assert_eq!(resp.api_e_maxhps, vec![20]);
    }
}
