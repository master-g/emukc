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
use super::super::map_route::{
    FleetRouteContext, FleetRouteShipEntry, evaluate_route_destination, select_start_cell,
};
use super::super::slot_item::{DRUM_CANISTER_MST_ID, find_slot_items_by_id_impl};
use super::setup::escort_fleet_ships_impl;

/// A ship's own `api_soku` at or above this is fast (5 is slow, 10 is fast).
const FAST_SPEED: i64 = 10;

/// One equipment's share of the formula-33 `LoS` score, before the branch-point coefficient:
/// `equipment coefficient × (LoS + improvement coefficient × √★)`.
///
/// Both coefficient tables follow the compass simulator's `logic/seek/equip.ts`, keyed by
/// `api_type[2]`. It files 艦上偵察機(II) (94) under 艦上偵察機 and 大型電探(II) (93) under
/// 大型電探, so they share those rows here.
fn los_equipment_score(type3: i64, saku: i64, level: i64) -> f64 {
    if saku == 0 {
        return 0.0;
    }
    let coefficient = match type3 {
        8 => 0.8,      // 艦上攻撃機
        9 | 94 => 1.0, // 艦上偵察機
        11 => 1.1,     // 水上爆撃機
        10 => 1.2,     // 水上偵察機
        _ => 0.6,
    };
    let improvement = match type3 {
        11 => 1.15,              // 水上爆撃機
        9 | 94 | 10 | 41 => 1.2, // 艦上偵察機, 水上偵察機, 大型飛行艇
        12 => 1.25,              // 小型電探
        13 | 93 => 1.4,          // 大型電探
        _ => 0.0,
    };
    coefficient * (saku as f64 + improvement * (level as f64).sqrt())
}

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
    let mut context = sortie_route_context(c, codex, &sortie).await?;
    context.visited_cell_ids.insert(current.cell_no);
    let cell_no = evaluate_route_destination(current, stage, &context, selected_cell_id)?;
    let mut visited_cell_ids = context.visited_cell_ids;
    visited_cell_ids.insert(cell_no);
    Ok(RouteStep {
        cell_no,
        visited_cell_ids,
    })
}

/// Pick the start cell on a map whose start depends on the fleet. `None` when the map has
/// no start rules or none of them fires, leaving the choice to the caller.
pub(super) async fn route_start_cell<'a, C>(
    c: &C,
    codex: &Codex,
    sortie: &SortieRoute<'_>,
    stage: &'a MapStageDefinition,
) -> Result<Option<&'a MapCellDefinition>, GameplayError>
where
    C: ConnectionTrait,
{
    if stage.start_rules.is_empty() {
        return Ok(None);
    }
    let context = sortie_route_context(c, codex, sortie).await?;
    Ok(select_start_cell(stage, &context).and_then(|cell_no| stage.cell(cell_no)))
}

/// Everything the predicates may read about this sortie's fleet.
pub(super) async fn sortie_route_context<C>(
    c: &C,
    codex: &Codex,
    sortie: &SortieRoute<'_>,
) -> Result<FleetRouteContext, GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, sortie.profile_id).await?;
    let mut context =
        build_fleet_route_context(c, codex, sortie.fleet_ships, profile.hq_level).await?;
    if profile.combined_type != 0 {
        let escort = escort_fleet_ships_impl(c, sortie.profile_id).await?;
        context.escort_ship_entries =
            build_fleet_route_context(c, codex, &escort, profile.hq_level).await?.ship_entries;
    }
    context.visited_cell_ids = sortie.visited_cell_ids.clone();
    Ok(context)
}

pub(super) async fn build_fleet_route_context<C>(
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
    // Map slot instance id → (type3 equip category, master id, LoS stat from master, ★)
    let slot_item_info = slot_items
        .into_iter()
        .map(|item| {
            let api_saku =
                codex.manifest.find_slotitem(item.mst_id).map(|mst| mst.api_saku).unwrap_or(0);
            (item.id, (item.type3, item.mst_id, api_saku, item.level))
        })
        .collect::<BTreeMap<_, _>>();
    let mut ship_ids = BTreeSet::new();
    let mut ship_type_counts = BTreeMap::<i64, i64>::new();
    let mut ship_entries = Vec::with_capacity(fleet_ships.len());
    let mut min_speed = i64::MAX;
    let mut drum_ships = 0;
    let mut flagship_ship_id = None;
    let mut flagship_ship_type = None;
    // Formula-33 accumulators: Σ√(ship's own LoS) and Σ weighted equipment LoS.
    let mut los_own_acc: f64 = 0.0;
    let mut los_equip_acc: f64 = 0.0;

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
                slotitem_ids: BTreeSet::new(),
                base_slow: mst.api_soku < FAST_SPEED,
            };
            // `los_now` is the ship's own LoS plus its equipment's, so the equipment
            // sum is what has to come off to get the value under the square root.
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
                let Some((type3, mst_id, api_saku, level)) = slot_item_info.get(&slot_id).copied()
                else {
                    continue;
                };
                entry.slotitem_types.insert(type3);
                entry.slotitem_ids.insert(mst_id);
                if mst_id == DRUM_CANISTER_MST_ID {
                    carries_drum = true;
                }
                ship_equip_saku += api_saku;
                los_equip_acc += los_equipment_score(type3, api_saku, level);
            }
            if carries_drum {
                drum_ships += 1;
            }
            ship_entries.push(entry);

            // ponytail: equipment LoS bonuses (装備ボーナス) are not modelled anywhere in ship
            // stats; once they are, they belong under this square root.
            los_own_acc += ((ship.los_now - ship_equip_saku).max(0) as f64).sqrt();
        } else {
            // Unknown ship — its equipment cannot be told apart from its own LoS.
            los_own_acc += (ship.los_now.max(0) as f64).sqrt();
        }
        min_speed = min_speed.min(ship.speed);
    }

    let fleet_size = fleet_ships.len() as i64;
    // ponytail: 0.4 is the commonly used HQ-level factor; 3-5 G and 6-3 H have reported
    // counter-examples (6-3 H measured 0.33–0.35). Revisit if a per-node factor is verified.
    let hq_penalty = (0.4 * hq_level as f64).ceil();
    // A 7-ship 遊撃部隊 would make this negative; the regular maps never field one.
    let fleet_bonus = ((6 - fleet_size).max(0)) as f64 * 2.0;
    let los_ship_term = los_own_acc - hq_penalty + fleet_bonus;

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
        drum_ships,
        los_ship_term,
        los_equip_term: los_equip_acc,
        escort_ship_entries: Vec::new(),
    })
}
