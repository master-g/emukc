//! The reconnaissance cells (航空偵察) of 6-3.
//!
//! See `docs/solutions/architecture-patterns/reconnaissance-and-escort-cells.md`.

use emukc_crypto::rng;
use emukc_db::entity::profile::ship;
use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::codex::{Codex, map::CellItem};

use crate::err::GameplayError;

use super::{SortieAirSearch, SortieItemGet, cell_effect::fleet_equipment};

/// `api_event_id` of a reconnaissance cell.
pub(super) const EVENT_ID: i64 = 7;

/// 水上偵察機, 水上爆撃機 and 大型飛行艇: the equipment a reconnaissance counts.
const SEAPLANE_TYPES: [i64; 2] = [10, 11];
const FLYING_BOAT_TYPE: i64 = 41;

/// What one reconnaissance finds, drawn evenly from twenty: wikiwiki 中部海域
/// 「航空偵察で獲得できる資源一覧」 for the amounts, the same at G as at H.
// ponytail: only the seven of twenty for 開発資材 follow from the source (0.35 a success
// and 1 a great success on average); the split of the other thirteen is ours.
const FOUND_BY_SUCCESS: [(CellItem, i64); 20] = {
    use CellItem::{Ammo, Bauxite, DevMat};
    [
        (DevMat, 1),
        (DevMat, 1),
        (DevMat, 1),
        (DevMat, 1),
        (DevMat, 1),
        (DevMat, 1),
        (DevMat, 1),
        (Ammo, 50),
        (Ammo, 50),
        (Ammo, 50),
        (Ammo, 60),
        (Ammo, 60),
        (Ammo, 80),
        (Ammo, 80),
        (Bauxite, 30),
        (Bauxite, 30),
        (Bauxite, 40),
        (Bauxite, 40),
        (Bauxite, 50),
        (Bauxite, 50),
    ]
};
const FOUND_BY_GREAT_SUCCESS: [(CellItem, i64); 20] = {
    use CellItem::{Ammo, Bauxite, DevMat};
    [
        (DevMat, 2),
        (DevMat, 2),
        (DevMat, 3),
        (DevMat, 3),
        (DevMat, 3),
        (DevMat, 3),
        (DevMat, 4),
        (Ammo, 100),
        (Ammo, 100),
        (Ammo, 100),
        (Ammo, 150),
        (Ammo, 150),
        (Ammo, 200),
        (Ammo, 200),
        (Bauxite, 80),
        (Bauxite, 80),
        (Bauxite, 100),
        (Bauxite, 100),
        (Bauxite, 150),
        (Bauxite, 150),
    ]
};

/// The score a reconnaissance at this node has to reach (偵察マス定数).
// ponytail: 6-3 is the one regular map with such cells; move this into
// `map_cell_effects.json` when a second one has them.
fn node_constant(map_id: i64, label: &str) -> Option<i64> {
    match (map_id, label) {
        (63, "G") => Some(12),
        (63, "H") => Some(16),
        _ => None,
    }
}

/// Fly the reconnaissance of the cell the fleet has come to. What it finds is shown at
/// once and given when the boss is beaten.
///
/// Each slot of seaplanes or flying boats scores her 索敵 × ⁴√(aircraft left). A score of
/// the node's constant is a success, and of that constant × (16 to 22) ÷ 10 a great one.
pub(super) async fn reconnoitre<C>(
    c: &C,
    codex: &Codex,
    map_id: i64,
    label: &str,
    fleet_ships: &[ship::Model],
) -> Result<(SortieAirSearch, Option<SortieItemGet>), GameplayError>
where
    C: ConnectionTrait,
{
    let equipment = fleet_equipment(c, fleet_ships).await?;
    let mut score = 0.0;
    let mut plane_type = 0;
    for (ship, items) in fleet_ships.iter().zip(&equipment) {
        let slots = [
            (ship.slot_1, ship.onslot_1),
            (ship.slot_2, ship.onslot_2),
            (ship.slot_3, ship.onslot_3),
            (ship.slot_4, ship.onslot_4),
            (ship.slot_5, ship.onslot_5),
        ];
        for item in items {
            let flying_boat = item.type3 == FLYING_BOAT_TYPE;
            let Some(&(_, left)) = slots.iter().find(|(slot, left)| *slot == item.id && *left > 0)
            else {
                continue;
            };
            if !flying_boat && !SEAPLANE_TYPES.contains(&item.type3) {
                continue;
            }
            let los = codex.manifest.find_slotitem(item.mst_id).map_or(0, |mst| mst.api_saku);
            score += los as f64 * (left as f64).powf(0.25);
            // The client draws a flying boat (1) or a reconnaissance seaplane (2); a
            // flying boat anywhere in the fleet is the one it is shown.
            if flying_boat {
                plane_type = 1;
            } else if plane_type == 0 {
                plane_type = 2;
            }
        }
    }
    let Some(constant) = node_constant(map_id, label).filter(|_| plane_type > 0) else {
        return Ok((
            SortieAirSearch {
                plane_type,
                result: 0,
            },
            None,
        ));
    };
    let constant = constant as f64;
    let found = if score < constant {
        None
    } else if score >= constant * rng::i64_inclusive(16..=22) as f64 / 10.0 {
        Some((2, &FOUND_BY_GREAT_SUCCESS))
    } else {
        Some((1, &FOUND_BY_SUCCESS))
    };
    let (result, reward) = match found {
        Some((result, table)) => {
            let (item, amount) = table[rng::usize(0..table.len())];
            (
                result,
                Some(SortieItemGet {
                    item,
                    amount,
                }),
            )
        }
        None => (0, None),
    };
    Ok((
        SortieAirSearch {
            plane_type,
            result,
        },
        reward,
    ))
}
