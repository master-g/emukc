//! Transport points (TP, 輸送物資量): what a fleet lands at a landing cell and a
//! transport gauge takes from a boss win.

use std::collections::BTreeMap;

use emukc_db::{entity::profile::ship, sea_orm::ConnectionTrait};
use emukc_model::codex::Codex;

use crate::err::GameplayError;

use super::slot_item::find_slot_items_by_id_impl;

/// 鬼怒改二 carries a 大発動艇 of her own.
const KINU_KAI_NI: i64 = 487;

/// Points a ship brings by her type alone. Types not listed bring none.
const fn ship_type_points(stype: i64) -> i64 {
    match stype {
        // 駆逐艦
        2 => 5,
        // 軽巡洋艦
        3 => 2,
        // 航空巡洋艦
        6 => 4,
        // 航空戦艦, 潜水母艦
        10 | 20 => 7,
        // 潜水空母
        14 => 1,
        // 水上機母艦
        16 => 9,
        // 揚陸艦
        17 => 12,
        // 練習巡洋艦
        21 => 6,
        // 補給艦
        22 => 15,
        _ => 0,
    }
}

/// Points a piece of equipment brings, by its `api_type[2]`.
const fn equipment_points(equip_type: i64) -> i64 {
    match equip_type {
        // 上陸用舟艇
        24 => 8,
        // 簡易輸送部材 (ドラム缶)
        30 => 5,
        // 特型内火艇
        46 => 2,
        // 戦闘糧食
        43 => 1,
        _ => 0,
    }
}

/// Points of one ship: her type's plus her equipment's.
fn ship_points(ship_mst_id: i64, stype: i64, equip_types: impl Iterator<Item = i64>) -> i64 {
    ship_type_points(stype)
        + if ship_mst_id == KINU_KAI_NI {
            8
        } else {
            0
        }
        + equip_types.map(equipment_points).sum::<i64>()
}

/// What an S rank lands from `points`; an A rank lands seven tenths, rounded down, and
/// anything less nothing.
pub(super) fn landed_points(points: i64, win_rank: &str) -> i64 {
    match win_rank {
        "S" => points,
        "A" => points * 7 / 10,
        _ => 0,
    }
}

/// The transport points of the fleet as it stands. A ship at 大破 or worse carries
/// nothing, and neither does her equipment.
pub(super) async fn fleet_transport_points_impl<C>(
    c: &C,
    codex: &Codex,
    ships: &[ship::Model],
) -> Result<i64, GameplayError>
where
    C: ConnectionTrait,
{
    let slots = |ship: &ship::Model| {
        [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
            .into_iter()
            .filter(|slot_id| *slot_id > 0)
    };
    let slot_ids = ships.iter().flat_map(slots).collect::<Vec<_>>();
    let equip_types = if slot_ids.is_empty() {
        BTreeMap::new()
    } else {
        find_slot_items_by_id_impl(c, &slot_ids)
            .await?
            .into_iter()
            .filter_map(|item| {
                let mst = codex.manifest.find_slotitem(item.mst_id)?;
                Some((item.id, mst.api_type[2]))
            })
            .collect::<BTreeMap<_, _>>()
    };

    Ok(ships
        .iter()
        .filter(|ship| ship.hp_now * 4 > ship.hp_max)
        .map(|ship| {
            let stype = codex.manifest.find_ship(ship.mst_id).map_or(0, |mst| mst.api_stype);
            ship_points(
                ship.mst_id,
                stype,
                slots(ship).filter_map(|slot_id| equip_types.get(&slot_id).copied()),
            )
        })
        .sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ship_brings_her_type_and_her_equipment() {
        // 駆逐艦 with two ドラム缶 and a 大発動艇.
        assert_eq!(ship_points(1, 2, [30, 30, 24].into_iter()), 5 + 5 + 5 + 8);
        // A 重巡洋艦 brings nothing herself; a main gun is not cargo.
        assert_eq!(ship_points(59, 5, [2].into_iter()), 0);
        // 鬼怒改二 counts as a 軽巡洋艦 with a 大発動艇 built in.
        assert_eq!(ship_points(KINU_KAI_NI, 3, std::iter::empty()), 10);
    }

    #[test]
    fn an_a_rank_lands_seven_tenths_rounded_down() {
        assert_eq!(landed_points(46, "S"), 46);
        assert_eq!(landed_points(46, "A"), 32);
        assert_eq!(landed_points(46, "B"), 0);
    }
}
