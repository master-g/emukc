//! Sortie routing: which cell the fleet moves to next.
//!
//! [`route_next_cell`] is the only way in. Behind it sit the fleet facts read
//! from the database, the route-history rule, predicate evaluation and the
//! branch roll in [`map_route`](super::super::map_route).

use std::collections::{BTreeMap, BTreeSet};

use emukc_db::entity::profile::ship;
use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::codex::{
    Codex,
    map::{MapCellDefinition, MapStageDefinition},
};

use crate::err::GameplayError;

use super::super::basic::find_profile;
use super::super::map_route::{FleetRouteContext, FleetRouteShipEntry, evaluate_route_destination};
use super::super::slot_item::{DRUM_CANISTER_MST_ID, find_slot_items_by_id_impl};

/// The sortie a route is being picked for.
pub(super) struct SortieRoute<'a> {
    pub(super) profile_id: i64,
    /// The sortie fleet, flagship first.
    pub(super) fleet_ships: &'a [ship::Model],
    /// Cells passed so far; empty when the fleet has not left the start.
    pub(super) visited_cell_ids: &'a BTreeSet<i64>,
}

/// One move of the fleet.
pub(super) struct RouteStep {
    /// The cell the fleet moves to.
    pub(super) cell_no: i64,
    /// The route history after the move, for the sortie to keep.
    pub(super) visited_cell_ids: BTreeSet<i64>,
}

/// Pick the cell the fleet moves to from `current`.
///
/// The cell being left always counts as visited, so a 「〜を経由」 rule sees the
/// start cell on the first step just as it sees every later cell.
pub(super) async fn route_next_cell<C>(
    c: &C,
    codex: &Codex,
    sortie: SortieRoute<'_>,
    stage: &MapStageDefinition,
    current: &MapCellDefinition,
    selected_cell_id: Option<i64>,
) -> Result<RouteStep, GameplayError>
where
    C: ConnectionTrait,
{
    let hq_level = find_profile(c, sortie.profile_id).await?.hq_level;
    let mut context = build_fleet_route_context(c, codex, sortie.fleet_ships, hq_level).await?;
    context.visited_cell_ids = sortie.visited_cell_ids.clone();
    context.visited_cell_ids.insert(current.cell_no);
    let cell_no = evaluate_route_destination(current, stage, &context, selected_cell_id)?;
    let mut visited_cell_ids = context.visited_cell_ids;
    visited_cell_ids.insert(cell_no);
    Ok(RouteStep {
        cell_no,
        visited_cell_ids,
    })
}

async fn build_fleet_route_context<C>(
    c: &C,
    codex: &Codex,
    fleet_ships: &[ship::Model],
    hq_level: i64,
) -> Result<FleetRouteContext, GameplayError>
where
    C: ConnectionTrait,
{
    let slot_ids = fleet_ships
        .iter()
        .flat_map(|ship| {
            [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
        })
        .filter(|slot_id| *slot_id > 0)
        .collect::<Vec<_>>();
    let slot_items = if slot_ids.is_empty() {
        Vec::new()
    } else {
        find_slot_items_by_id_impl(c, &slot_ids).await?
    };
    // Map slot instance id → (type3 equip category, master id, LoS stat from master)
    let slot_item_info = slot_items
        .into_iter()
        .map(|item| {
            let api_saku =
                codex.manifest.find_slotitem(item.mst_id).map(|mst| mst.api_saku).unwrap_or(0);
            (item.id, (item.type3, item.mst_id, api_saku))
        })
        .collect::<BTreeMap<_, _>>();
    let mut ship_ids = BTreeSet::new();
    let mut ship_type_counts = BTreeMap::<i64, i64>::new();
    let mut ship_entries = Vec::with_capacity(fleet_ships.len());
    let mut min_speed = i64::MAX;
    let mut los_total = 0;
    let mut drum_ships = 0;
    let mut flagship_ship_id = None;
    let mut flagship_ship_type = None;
    // Accumulators for LoS formulas.
    let mut los_f1_acc: f64 = 0.0;
    let mut los_f3_acc: f64 = 0.0;

    for (idx, ship) in fleet_ships.iter().enumerate() {
        ship_ids.insert(ship.mst_id);
        if let Some(mst) = codex.manifest.find_ship(ship.mst_id) {
            *ship_type_counts.entry(mst.api_stype).or_default() += 1;
            if idx == 0 {
                flagship_ship_id = Some(ship.mst_id);
                flagship_ship_type = Some(mst.api_stype);
            }
            let mut entry = FleetRouteShipEntry {
                ship_id: ship.mst_id,
                ship_type: mst.api_stype,
                speed: ship.speed,
                slotitem_types: BTreeSet::new(),
            };
            // Sum equipment LoS for this ship (used in formula 3).
            let mut ship_equip_saku: i64 = 0;
            // Routing counts the *ship*, not the canisters on it: every wikiwiki
            // condition reads 「ドラム缶搭載艦の隻数」, and 5-4 spells the rule out —
            // a ship carrying both a canister and a landing craft counts once for
            // each, never twice for two canisters. Expeditions count the items
            // themselves, which is why `expedition.rs` keeps its own tally.
            let mut carries_drum = false;
            for slot_id in
                [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
            {
                let Some((type3, mst_id, api_saku)) = slot_item_info.get(&slot_id).copied() else {
                    continue;
                };
                entry.slotitem_types.insert(type3);
                if mst_id == DRUM_CANISTER_MST_ID {
                    carries_drum = true;
                }
                ship_equip_saku += api_saku;
            }
            if carries_drum {
                drum_ships += 1;
            }
            ship_entries.push(entry);

            // Formula 1: Σ sqrt(ship.los_now).  Per-equipment sqrt-weighting is
            // an approximation here; we use the combined value since individual
            // equipment LoS is not split out in the DB entity.
            los_f1_acc += (ship.los_now as f64).sqrt();

            // Formula 3: Σ(equip_los × 0.6 + sqrt(ship_base_los)).
            // ship_base_los = ship.los_now − ship_equip_saku.
            let ship_base_los = (ship.los_now - ship_equip_saku).max(0) as f64;
            los_f3_acc += ship_equip_saku as f64 * 0.6 + ship_base_los.sqrt();
        } else {
            // Unknown ship — fall back to raw los_now for both formulas.
            los_f1_acc += (ship.los_now as f64).sqrt();
            los_f3_acc += (ship.los_now as f64).sqrt();
        }
        min_speed = min_speed.min(ship.speed);
        los_total += ship.los_now;
    }

    let fleet_size = fleet_ships.len() as i64;
    // Formula 3 HQ penalty: ceil(0.4 × hq_level).
    let hq_penalty = (0.4 * hq_level as f64).ceil();
    // Fleet-size bonus: (6 - fleet_size) × 2.
    let fleet_bonus = ((6 - fleet_size).max(0)) as f64 * 2.0;
    let los_formula3 = (los_f3_acc - hq_penalty + fleet_bonus).max(0.0);

    Ok(FleetRouteContext {
        fleet_size,
        visited_cell_ids: BTreeSet::new(),
        ship_ids,
        flagship_ship_id,
        flagship_ship_type,
        ship_type_counts,
        ship_entries,
        min_speed: if min_speed == i64::MAX {
            0
        } else {
            min_speed
        },
        los_total,
        drum_ships,
        los_formula1: los_f1_acc,
        los_formula3,
    })
}
