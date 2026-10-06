//! Asks the router where a described fleet would go, without a database or a roll.
//!
//! This is a diagnostic: it lets the routing rules be compared against the source they were
//! converted from. The fleet is given as the rules see it — counts, speed, the two `LoS`
//! terms — so the comparison is about the rules, not about how ship stats are derived.

use std::collections::{BTreeMap, BTreeSet};

use emukc_model::codex::{
    Codex,
    map::{MapCellDefinition, MapStageDefinition},
};
use serde::Deserialize;

use crate::err::GameplayError;

use super::map::find_map_definition;
use super::map_route::{
    FleetRouteContext, FleetRouteShipEntry, route_distribution, start_distribution,
};
use super::slot_item::DRUM_CANISTER_MST_ID;

/// One ship of a probed fleet, flagship first.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteProbeShip {
    /// Master id.
    pub id: i64,
    /// `api_stype`.
    pub stype: i64,
    /// Current speed: 5 slow, 10 fast, 15 fast+, 20 fastest.
    pub speed: i64,
    /// The ship's own speed, before equipment, is slow.
    #[serde(default)]
    pub slow: bool,
    /// Master ids of the equipment carried.
    #[serde(default)]
    pub equips: Vec<i64>,
}

/// A fleet as the routing rules see it.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteProbeFleet {
    /// The ships, flagship first.
    pub ships: Vec<RouteProbeShip>,
    /// The `LoS` score's coefficient-independent part.
    #[serde(default)]
    pub los_ship: f64,
    /// The part of the `LoS` score the branch-point coefficient multiplies.
    #[serde(default)]
    pub los_equip: f64,
}

/// One question for [`probe_route`].
#[derive(Debug, Clone, Deserialize)]
pub struct RouteProbe {
    /// Map name such as `2-5`.
    pub map: String,
    /// Variant key; empty for a map without variants.
    #[serde(default)]
    pub variant: String,
    /// The node the fleet is on, `1` / `2` for a start point, or `None` to ask which start
    /// the fleet sorties from.
    pub node: Option<String>,
    /// Labels of the nodes passed before this one.
    #[serde(default)]
    pub visited: Vec<String>,
    /// The fleet.
    pub fleet: RouteProbeFleet,
}

fn context(fleet: &RouteProbeFleet) -> FleetRouteContext {
    let mut ship_type_counts = BTreeMap::<i64, i64>::new();
    for ship in &fleet.ships {
        *ship_type_counts.entry(ship.stype).or_default() += 1;
    }
    FleetRouteContext {
        fleet_size: fleet.ships.len() as i64,
        visited_cell_ids: BTreeSet::new(),
        ship_ids: fleet.ships.iter().map(|ship| ship.id).collect(),
        flagship_ship_id: fleet.ships.first().map(|ship| ship.id),
        flagship_ship_type: fleet.ships.first().map(|ship| ship.stype),
        ship_type_counts,
        ship_entries: fleet
            .ships
            .iter()
            .map(|ship| FleetRouteShipEntry {
                ship_id: ship.id,
                ship_type: ship.stype,
                speed: ship.speed,
                slotitem_types: BTreeSet::new(),
                slotitem_ids: ship.equips.iter().copied().collect(),
                base_slow: ship.slow,
            })
            .collect(),
        min_speed: fleet.ships.iter().map(|ship| ship.speed).min().unwrap_or(0),
        drum_ships: fleet
            .ships
            .iter()
            .filter(|ship| ship.equips.contains(&DRUM_CANISTER_MST_ID))
            .count() as i64,
        los_ship_term: fleet.los_ship,
        los_equip_term: fleet.los_equip,
        escort_ship_entries: Vec::new(),
    }
}

/// The variant's start cells in start-point order: `1`, then `2`.
fn start_cells(stage: &MapStageDefinition) -> Vec<&MapCellDefinition> {
    let mut starts = stage.start_source_cells();
    starts.sort_by_key(|cell| cell.cell_no);
    starts
}

fn start_label(index: usize) -> String {
    (index + 1).to_string()
}

fn cells_labelled<'a>(stage: &'a MapStageDefinition, label: &str) -> Vec<&'a MapCellDefinition> {
    if let Some(start) = start_cells(stage)
        .into_iter()
        .enumerate()
        .find_map(|(index, cell)| (start_label(index) == label).then_some(cell))
    {
        return vec![start];
    }
    stage.cells.iter().filter(|cell| cell.node_label.as_deref() == Some(label)).collect()
}

/// Where the fleet goes next, as node label to probability.
///
/// A node the player chooses from has no rules, so every next node comes out equally
/// likely. Start points are labelled `1` and `2`.
pub fn probe_route(
    codex: &Codex,
    probe: &RouteProbe,
) -> Result<BTreeMap<String, f64>, GameplayError> {
    let bad = |message: String| GameplayError::EntryNotFound(message);
    let (maparea_id, mapinfo_no) = probe
        .map
        .split_once('-')
        .and_then(|(area, no)| Some((area.parse().ok()?, no.parse().ok()?)))
        .ok_or_else(|| bad(format!("bad map name {}", probe.map)))?;
    let definition = find_map_definition(codex, maparea_id, mapinfo_no)?;
    let stage = definition
        .variants
        .get(&probe.variant)
        .ok_or_else(|| bad(format!("map {} has no variant `{}`", probe.map, probe.variant)))?;
    let mut context = context(&probe.fleet);
    let starts = start_cells(stage);

    let Some(node) = &probe.node else {
        return Ok(start_distribution(stage, &context)
            .into_iter()
            .filter_map(|(cell_no, share)| {
                let index = starts.iter().position(|cell| cell.cell_no == cell_no)?;
                Some((start_label(index), share))
            })
            .collect());
    };

    let current = cells_labelled(stage, node)
        .into_iter()
        .find(|cell| !cell.next_cells.is_empty())
        .ok_or_else(|| bad(format!("map {} has no node {node} to leave from", probe.map)))?;
    for label in &probe.visited {
        let cells = cells_labelled(stage, label);
        if cells.is_empty() {
            return Err(bad(format!("map {} has no visited node {label}", probe.map)));
        }
        context.visited_cell_ids.extend(cells.into_iter().map(|cell| cell.cell_no));
    }
    // The cell being left counts as visited, as it does on a real sortie.
    context.visited_cell_ids.insert(current.cell_no);

    let mut distribution = BTreeMap::<String, f64>::new();
    for (cell_no, share) in route_distribution(current, stage, &context) {
        let label = stage
            .cell(cell_no)
            .and_then(|cell| cell.node_label.clone())
            .ok_or_else(|| bad(format!("map {} cell {cell_no} has no label", probe.map)))?;
        *distribution.entry(label).or_default() += share;
    }
    Ok(distribution)
}
