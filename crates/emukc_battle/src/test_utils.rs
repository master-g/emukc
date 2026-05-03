use emukc_model::{
    codex::Codex,
    kc2::{
        api::{KcApiShip, KcApiSlotItem},
        level,
        types::{KcShipType, KcSlotItemType3},
    },
};

use crate::types::{BattleRuntimeShip, BattleShipInput};

pub(crate) fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
    let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
    let exp_now = level::ship_level_required_exp(level);
    let (_, next_exp) = level::exp_to_ship_level(exp_now);
    ship.api_lv = level;
    ship.api_exp = [exp_now, next_exp, 0];
    codex.cal_ship_status(&mut ship, &slot_items, false).unwrap();
    BattleShipInput {
        ship,
        slot_items,
        effect_list: vec![0],
        married: false,
    }
}

pub(crate) fn first_ship_mst_by_type(codex: &Codex, ship_type: KcShipType) -> i64 {
    codex
        .manifest
        .api_mst_ship
        .iter()
        .find(|mst| KcShipType::n(mst.api_stype) == Some(ship_type))
        .map(|mst| mst.api_id)
        .unwrap()
}

pub(crate) fn first_slotitem_mst_by_type(codex: &Codex, slot_type: KcSlotItemType3) -> i64 {
    codex
        .manifest
        .api_mst_slotitem
        .iter()
        .find(|mst| KcSlotItemType3::n(mst.api_type[2]) == Some(slot_type))
        .map(|mst| mst.api_id)
        .unwrap()
}

pub(crate) fn slotitem_with_mst_id(mst_id: i64) -> KcApiSlotItem {
    KcApiSlotItem {
        api_id: 0,
        api_slotitem_id: mst_id,
        api_locked: 0,
        api_level: 0,
        api_alv: None,
    }
}

pub(crate) fn slotitem_mst_id_by_name(codex: &Codex, name: &str) -> i64 {
    codex
        .manifest
        .api_mst_slotitem
        .iter()
        .find(|mst| mst.api_name == name)
        .map(|mst| mst.api_id)
        .unwrap()
}

pub(crate) fn ship_mst_id_by_name(codex: &Codex, name: &str) -> i64 {
    codex
        .manifest
        .api_mst_ship
        .iter()
        .find(|mst| mst.api_name == name)
        .map(|mst| mst.api_id)
        .unwrap()
}

pub(crate) fn make_test_ship(
    nowhp: i64,
    entry_hp: i64,
    current_hp: i64,
    maxhp: i64,
) -> BattleRuntimeShip {
    make_test_ship_ctx(nowhp, entry_hp, current_hp, maxhp, true, true)
}

pub(crate) fn make_test_ship_ctx(
    nowhp: i64,
    entry_hp: i64,
    current_hp: i64,
    maxhp: i64,
    is_friendly: bool,
    is_sortie: bool,
) -> BattleRuntimeShip {
    let mut ship = BattleRuntimeShip::new(
        BattleShipInput {
            ship: test_api_ship(nowhp, maxhp),
            slot_items: vec![],
            effect_list: vec![],
            married: false,
        },
        is_friendly,
        is_sortie,
    );
    ship.entry_hp = entry_hp;
    ship.current_hp = current_hp;
    ship
}

pub(crate) fn test_api_ship(nowhp: i64, maxhp: i64) -> KcApiShip {
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
