//! What a resource cell gives and a maelstrom takes.
//!
//! The figures are the map catalog's, cell by cell. See
//! `docs/solutions/architecture-patterns/resource-and-maelstrom-cells.md`.

use std::collections::BTreeMap;

use emukc_crypto::rng;
use emukc_db::entity::profile::{item::slot_item, ship};
use emukc_db::sea_orm::{ActiveValue, IntoActiveModel, entity::prelude::*};
use emukc_model::{
    codex::{
        Codex,
        map::{CellEffects, CellGain, CellItem, CellMaelstrom},
    },
    kc2::MaterialCategory,
};

use crate::{
    err::GameplayError,
    game::{
        map::active_map_catalog, material::add_material_impl, sortie_store::SortieStore,
        use_item::add_use_item_impl,
    },
};

use super::{SortieHappening, SortieItemGet};

/// How likely a maelstrom that can be a strong one is to be one. wikiwiki 資材 has
/// 「1割ほど」 and no count.
const STRONG_MAELSTROM_CHANCE: f64 = 0.10;

/// What a ship carrying radar is spared of a maelstrom, in percent, by how many ships of
/// the fleet carry one. wikiwiki 資材「うずしおによる燃料・弾薬の喪失ルール」.
const RADAR_SPARES: [i64; 7] = [0, 25, 40, 50, 55, 58, 60];

/// Settle the cell the fleet has just come to: the things a resource cell gives, or what
/// a maelstrom takes. Special materials and furniture boxes are the player's at once; fuel,
/// ammunition, steel and bauxite wait for the port, and the caller puts them by with
/// [`put_by`] once its transaction is through.
pub(super) async fn resolve_cell_effect<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    map_id: i64,
    cell_no: i64,
    fleet_ships: &[ship::Model],
) -> Result<(Option<Vec<SortieItemGet>>, Option<SortieHappening>), GameplayError>
where
    C: ConnectionTrait,
{
    let catalog = active_map_catalog(codex);
    let Some(effect) = catalog.cell_effect(map_id, cell_no) else {
        return Ok((None, None));
    };
    let equipment = fleet_equipment(c, fleet_ships).await?;

    let mut gains = Vec::new();
    for gain in &effect.gains {
        let amount = gain_amount(gain, &catalog.cell_effects, &equipment);
        match (gain.item.furniture_box(), gain.item.material_id()) {
            (Some(use_item), _) => {
                add_use_item_impl(c, profile_id, use_item, amount).await?;
            }
            // Fuel, ammunition, steel and bauxite wait for the port.
            (None, Some(1..=4)) => {}
            (None, material) => {
                let category = MaterialCategory::from_id(material.unwrap_or_default());
                add_material_impl(c, codex, profile_id, &[(category, amount)]).await?;
            }
        }
        gains.push(SortieItemGet {
            item: gain.item,
            amount,
        });
    }

    let happening = match &effect.maelstrom {
        Some(maelstrom) => Some(drain_ships(c, maelstrom, &equipment, fleet_ships).await?),
        None => None,
    };
    Ok(((!gains.is_empty()).then_some(gains), happening))
}

/// Put by what a resource cell gave of fuel, ammunition, steel and bauxite until the fleet
/// is back in port.
pub(super) fn put_by(store: &SortieStore, profile_id: i64, gains: Option<&[SortieItemGet]>) {
    for gain in gains.unwrap_or_default() {
        if let (None, Some(material @ 1..=4)) = (gain.item.furniture_box(), gain.item.material_id())
        {
            store.add_pending_gain(profile_id, material as usize - 1, gain.amount);
        }
    }
}

/// The equipment the fleet carries, by the ship carrying it.
async fn fleet_equipment<C>(
    c: &C,
    fleet_ships: &[ship::Model],
) -> Result<Vec<Vec<slot_item::Model>>, GameplayError>
where
    C: ConnectionTrait,
{
    let slots = |ship: &ship::Model| {
        [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
            .into_iter()
            .filter(|&id| id > 0)
    };
    let ids: Vec<i64> = fleet_ships.iter().flat_map(slots).collect();
    let mut items: BTreeMap<i64, slot_item::Model> = if ids.is_empty() {
        BTreeMap::new()
    } else {
        slot_item::Entity::find()
            .filter(slot_item::Column::Id.is_in(ids))
            .all(c)
            .await?
            .into_iter()
            .map(|item| (item.id, item))
            .collect()
    };
    Ok(fleet_ships
        .iter()
        .map(|ship| slots(ship).filter_map(|id| items.remove(&id)).collect())
        .collect())
}

/// What one gain of a resource cell comes to: a figure between its bounds in steps of 5,
/// more for each drum and landing craft the fleet carries, its cap at most. A gain whose
/// bounds are one figure draws no random number.
fn gain_amount(gain: &CellGain, effects: &CellEffects, equipment: &[Vec<slot_item::Model>]) -> i64 {
    let count = |ids: &[i64]| {
        equipment.iter().flatten().filter(|item| ids.contains(&item.mst_id)).count() as f64
    };
    let base = if gain.max > gain.min {
        gain.min + 5 * rng::i64_inclusive(0..=(gain.max - gain.min) / 5)
    } else {
        gain.min
    };
    let bonus =
        count(&effects.drums) * gain.per_drum + count(&effects.landing_craft) * gain.per_craft;
    let amount = base + bonus.floor() as i64;
    gain.cap.map_or(amount, |cap| amount.min(cap))
}

/// Take a maelstrom's share from every ship. The amount reported is the most any one
/// ship lost, which is what the client shows.
async fn drain_ships<C>(
    c: &C,
    maelstrom: &CellMaelstrom,
    equipment: &[Vec<slot_item::Model>],
    fleet_ships: &[ship::Model],
) -> Result<SortieHappening, GameplayError>
where
    C: ConnectionTrait,
{
    // 電探 small, large and large (II); a ship counts once however many she carries, and
    // her reinforcement slot does not count.
    let radar_ships = fleet_ships
        .iter()
        .zip(equipment)
        .filter(|(ship, items)| {
            items.iter().any(|item| item.id != ship.slot_ex && matches!(item.type3, 12 | 13 | 93))
        })
        .count();
    let spared = RADAR_SPARES[radar_ships.min(RADAR_SPARES.len() - 1)];
    // Only a cell that can be a strong maelstrom draws for it.
    let rate = match maelstrom.strong_rate {
        Some(strong) if rng::bool(STRONG_MAELSTROM_CHANCE) => strong,
        _ => maelstrom.rate,
    };
    let fuel = maelstrom.item == CellItem::Fuel;

    let mut most = 0;
    for ship in fleet_ships {
        let left = if fuel {
            ship.fuel
        } else {
            ship.ammo
        };
        let loss = (left * rate * (100 - spared) / 10_000).min(left);
        let loss = maelstrom.cap.map_or(loss, |cap| loss.min(cap));
        if loss <= 0 {
            continue;
        }
        most = most.max(loss);
        let mut am = ship.into_active_model();
        if fuel {
            am.fuel = ActiveValue::Set(left - loss);
        } else {
            am.ammo = ActiveValue::Set(left - loss);
        }
        am.update(c).await?;
    }
    Ok(SortieHappening {
        item: maelstrom.item,
        amount: most,
        radar_reduced: radar_ships > 0,
    })
}
