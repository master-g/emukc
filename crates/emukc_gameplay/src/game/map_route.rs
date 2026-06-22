use std::collections::{BTreeMap, BTreeSet};

use emukc_crypto::rng;
use emukc_model::codex::map::{
    MapCellDefinition, MapStageDefinition, RouteOperator, RoutePredicate, RouteRule, SpeedClass,
};

use crate::err::GameplayError;

#[derive(Debug, Clone, Default)]
pub(crate) struct FleetRouteShipEntry {
    pub(crate) ship_id: i64,
    pub(crate) ship_type: i64,
    pub(crate) speed: i64,
    pub(crate) slotitem_types: BTreeSet<i64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FleetRouteContext {
    pub(crate) fleet_size: i64,
    pub(crate) visited_cell_ids: BTreeSet<i64>,
    pub(crate) ship_ids: BTreeSet<i64>,
    pub(crate) flagship_ship_id: Option<i64>,
    pub(crate) flagship_ship_type: Option<i64>,
    pub(crate) ship_type_counts: BTreeMap<i64, i64>,
    pub(crate) ship_entries: Vec<FleetRouteShipEntry>,
    pub(crate) min_speed: i64,
    /// Raw sum of each ship's current `LoS` (base + equipment).  Used as the
    /// fallback when no formula is specified.
    pub(crate) los_total: i64,
    pub(crate) total_drums: i64,
    /// Precomputed `LoS` under Formula 1: `Σ sqrt(ship.los_now)`.
    /// Formula 1 uses per-equipment sqrt-weighted values; this is an approximation
    /// using the combined ship `LoS` when per-equipment breakdown is unavailable.
    pub(crate) los_formula1: f64,
    /// Precomputed `LoS` under Formula 3 (the standard 2-5-fleet formula):
    /// `Σ(equip_los × 0.6 + sqrt(ship_base_los)) − ceil(0.4 × hq_lv) + (6 − fleet_size) × 2`.
    pub(crate) los_formula3: f64,
}

impl FleetRouteContext {
    /// Return the `LoS` value to compare against a route predicate threshold.
    ///
    /// `formula` mirrors the `formula` field of `RoutePredicate::LoS`:
    /// - `None` or unrecognised string → `los_total` (raw sum, backward-compatible)
    /// - `"式1"` / `"1"` → formula-1 precomputed value
    /// - `"式3"` / `"3"` → formula-3 precomputed value
    ///
    /// Unknown formula strings produce a `tracing::warn!` and fall back to
    /// `los_total` so that unrecognised wiki annotations degrade gracefully.
    pub(crate) fn los_by_formula(&self, formula: Option<&str>) -> f64 {
        match formula {
            None => self.los_total as f64,
            Some("式1") | Some("1") => self.los_formula1,
            Some("式3") | Some("3") => self.los_formula3,
            Some(unknown) => {
                tracing::warn!(
                    formula = unknown,
                    los_total = self.los_total,
                    "unknown LoS formula, falling back to los_total"
                );
                self.los_total as f64
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoutePredicateEval {
    Matched,
    NotMatched,
    SourceUnknown,
    Unsupported,
}

/// Thin crate-local shim over [`MapStageDefinition::cell_has_routing_outgoing`], which owns
/// the canonical definition (shared with catalog-assembly's P-unlock routability guard).
pub(crate) fn cell_has_routing_outgoing(cell_no: i64, stage: &MapStageDefinition) -> bool {
    stage.cell_has_routing_outgoing(cell_no)
}

/// Returns the number of distinct candidate target cells that the routing
/// evaluation could select for `current`, **without** rolling the RNG.
///
/// This mirrors the first half of `evaluate_route_destination` (predicate
/// matching, priority filtering, topology intersection) but skips the weighted
/// random roll.  The result can be used to set `rashin_flg`: when the count is
/// exactly 1 the routing is deterministic and the client should not show a
/// branch selector; when > 1 the routing is non-deterministic.
pub(crate) fn evaluate_route_candidate_count(
    current: &MapCellDefinition,
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
) -> usize {
    let Some(rules) = stage.routing_rules.get(&current.cell_no).filter(|rules| !rules.is_empty())
    else {
        return current.next_cells.len();
    };

    let mut fallback_rules = Vec::<&RouteRule>::new();
    let mut matched_groups = BTreeMap::<String, (i64, Vec<&RouteRule>)>::new();
    for rule in rules {
        match route_predicate_matches(&rule.predicate, context, stage) {
            RoutePredicateEval::Matched if matches!(rule.predicate, RoutePredicate::Always) => {
                fallback_rules.push(rule);
            }
            RoutePredicateEval::Matched => {
                let key = route_predicate_key(&rule.predicate);
                let entry =
                    matched_groups.entry(key).or_insert_with(|| (rule.priority, Vec::new()));
                entry.0 = entry.0.min(rule.priority);
                entry.1.push(rule);
            }
            RoutePredicateEval::NotMatched
            | RoutePredicateEval::SourceUnknown
            | RoutePredicateEval::Unsupported => {}
        }
    }

    let executable = if matched_groups.is_empty() {
        fallback_rules
    } else {
        let min_priority =
            matched_groups.values().map(|(priority, _)| *priority).min().unwrap_or(0);
        matched_groups
            .into_values()
            .filter(|(priority, _)| *priority == min_priority)
            .flat_map(|(_, rules)| rules)
            .collect::<Vec<_>>()
    };

    if executable.is_empty() {
        // No matched rules; fall back to next_cells.
        return current.next_cells.len();
    }

    let candidate_targets: BTreeSet<i64> = executable
        .iter()
        .map(|rule| rule.to_cell_no)
        .filter(|cell_no| current.next_cells.contains(cell_no))
        .collect();

    if candidate_targets.is_empty() {
        current.next_cells.len()
    } else {
        candidate_targets.len()
    }
}

pub(crate) fn evaluate_route_destination(
    current: &MapCellDefinition,
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
    selected_cell_id: Option<i64>,
) -> Result<i64, GameplayError> {
    let Some(rules) = stage.routing_rules.get(&current.cell_no).filter(|rules| !rules.is_empty())
    else {
        return select_route_from_cells(current, stage, selected_cell_id);
    };

    let mut fallback_rules = Vec::<&RouteRule>::new();
    let mut matched_groups = BTreeMap::<String, (i64, Vec<&RouteRule>)>::new();
    let mut saw_source_unknown = false;
    let mut saw_unsupported = false;
    for rule in rules {
        match route_predicate_matches(&rule.predicate, context, stage) {
            RoutePredicateEval::Matched if matches!(rule.predicate, RoutePredicate::Always) => {
                fallback_rules.push(rule);
            }
            RoutePredicateEval::Matched => {
                let key = route_predicate_key(&rule.predicate);
                let entry =
                    matched_groups.entry(key).or_insert_with(|| (rule.priority, Vec::new()));
                entry.0 = entry.0.min(rule.priority);
                entry.1.push(rule);
            }
            RoutePredicateEval::NotMatched => {}
            RoutePredicateEval::SourceUnknown => saw_source_unknown = true,
            RoutePredicateEval::Unsupported => saw_unsupported = true,
        }
    }

    let executable = if matched_groups.is_empty() {
        fallback_rules
    } else {
        let min_priority =
            matched_groups.values().map(|(priority, _)| *priority).min().unwrap_or(0);
        matched_groups
            .into_values()
            .filter(|(priority, _)| *priority == min_priority)
            .flat_map(|(_, rules)| rules)
            .collect::<Vec<_>>()
    };

    if executable.is_empty() {
        let any_indeterminate = saw_source_unknown || saw_unsupported;
        let all_source_unknown = saw_source_unknown
            && !saw_unsupported
            && rules
                .iter()
                .all(|rule| matches!(rule.predicate, RoutePredicate::SourceUnknown { .. }));
        if all_source_unknown {
            let targets = rules.iter().map(|rule| rule.to_cell_no).collect::<BTreeSet<_>>();
            tracing::warn!(
                cell_no = current.cell_no,
                targets = ?targets,
                "all routing rules are source-unknown, falling back to random selection"
            );
            if let Some(selected_cell_id) = selected_cell_id
                && targets.contains(&selected_cell_id)
                && current.next_cells.contains(&selected_cell_id)
            {
                return Ok(selected_cell_id);
            }
            return select_route_from_cells(current, stage, None);
        }

        if let Some(selected_cell_id) = selected_cell_id
            && any_indeterminate
            && current.next_cells.contains(&selected_cell_id)
        {
            return Ok(selected_cell_id);
        }

        let unconditional_targets = rules
            .iter()
            .filter(|rule| matches!(rule.predicate, RoutePredicate::Always))
            .map(|rule| rule.to_cell_no)
            .collect::<BTreeSet<_>>();
        if any_indeterminate && unconditional_targets.len() == 1 {
            return unconditional_targets.iter().next().copied().ok_or_else(|| {
                GameplayError::WrongType(format!(
                    "cell {} has no executable route",
                    current.cell_no
                ))
            });
        }
        if any_indeterminate {
            return select_route_from_cells(current, stage, selected_cell_id);
        }
        return Err(GameplayError::WrongType(format!(
            "cell {} has no executable routing rule",
            current.cell_no,
        )));
    }

    let candidate_targets: BTreeSet<i64> = executable
        .iter()
        .map(|rule| rule.to_cell_no)
        .filter(|cell_no| current.next_cells.contains(cell_no))
        .collect();
    if candidate_targets.is_empty() {
        tracing::warn!(
            cell_no = current.cell_no,
            rule_targets = ?executable.iter().map(|r| r.to_cell_no).collect::<Vec<_>>(),
            next_cells = ?current.next_cells,
            "route rules filtered by topology, falling back to next_cells"
        );
        return select_route_from_cells(current, stage, selected_cell_id);
    }
    if let Some(selected_cell_id) = selected_cell_id {
        if !candidate_targets.contains(&selected_cell_id) {
            return Err(GameplayError::WrongType(format!(
                "cell {selected_cell_id} is not a valid route from {}",
                current.cell_no,
            )));
        }
        return Ok(selected_cell_id);
    }
    if candidate_targets.len() == 1 {
        return candidate_targets.iter().next().copied().ok_or_else(|| {
            GameplayError::WrongType(format!("cell {} has no executable route", current.cell_no))
        });
    }

    let weights = executable.iter().fold(BTreeMap::<i64, u64>::new(), |mut acc, rule| {
        let weight = if let RoutePredicate::FleetSizeWeightedRandom {
            weights,
        } = &rule.predicate
        {
            let pct = weights
                .iter()
                .find(|w| w.fleet_size == context.fleet_size)
                .or_else(|| {
                    weights
                        .iter()
                        .min_by_key(|w| (w.fleet_size - context.fleet_size).unsigned_abs())
                })
                .map(|w| w.probability_pct)
                .unwrap_or(50.0);
            ((pct * 100.0).round() as i64).max(1)
        } else {
            rule.weight.unwrap_or_else(|| {
                rule.probability_pct
                    .map(|probability| ((probability * 100.0).round() as i64).max(1))
                    .unwrap_or(1)
            })
        };
        *acc.entry(rule.to_cell_no).or_default() += weight.max(1) as u64;
        acc
    });
    let total_weight = weights.values().sum::<u64>();
    if total_weight == 0 {
        return candidate_targets.iter().next().copied().ok_or_else(|| {
            GameplayError::WrongType(format!("cell {} has no executable route", current.cell_no))
        });
    }

    let roll = rng::u64(0..total_weight);
    select_route_target_for_roll(&weights, roll).ok_or_else(|| {
        GameplayError::WrongType(format!("cell {} has no executable route", current.cell_no))
    })
}

fn select_route_from_cells(
    current: &MapCellDefinition,
    stage: &MapStageDefinition,
    selected_cell_id: Option<i64>,
) -> Result<i64, GameplayError> {
    if let Some(selected_cell_id) = selected_cell_id {
        if !current.next_cells.contains(&selected_cell_id) {
            return Err(GameplayError::WrongType(format!(
                "cell {selected_cell_id} is not a valid route from {}",
                current.cell_no,
            )));
        }
        Ok(selected_cell_id)
    } else {
        match current.next_cells.as_slice() {
            [] => Err(GameplayError::WrongType(format!(
                "cell {} has no executable route",
                current.cell_no,
            ))),
            [only] => Ok(*only),
            _ if current.cell_no == 0 => {
                let inferred_start = stage.parse_warnings.iter().any(|warning| {
                    warning == "missing_start_routes"
                        || warning.starts_with("inferred_multi_root_start")
                });
                if inferred_start {
                    return Err(GameplayError::WrongType(
                        "cell 0 requires explicit start routing rules for multiple targets"
                            .to_string(),
                    ));
                }
                let index = rng::usize(0..current.next_cells.len());
                Ok(current.next_cells[index])
            }
            _ => {
                let index = rng::usize(0..current.next_cells.len());
                Ok(current.next_cells[index])
            }
        }
    }
}

pub(crate) fn route_predicate_matches(
    predicate: &RoutePredicate,
    context: &FleetRouteContext,
    stage: &MapStageDefinition,
) -> RoutePredicateEval {
    match predicate {
        RoutePredicate::Always
        | RoutePredicate::FleetSizeWeightedRandom {
            ..
        } => RoutePredicateEval::Matched,
        RoutePredicate::VisitedNode {
            cell_nos,
            visited,
        } => RoutePredicateEval::from_bool(
            cell_nos.iter().any(|cell_no| context.visited_cell_ids.contains(cell_no)) == *visited,
        ),
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => {
            let cell_nos: Vec<i64> = node_labels
                .iter()
                .filter_map(|label| {
                    stage.cells.iter().find_map(|cell| {
                        cell.node_label
                            .as_ref()
                            .and_then(|nl| (nl == label).then_some(cell.cell_no))
                    })
                })
                .collect();
            // If any label could not be resolved to a cell in the current stage
            // graph, we have incomplete information — treat this as SourceUnknown
            // rather than NotMatched so the caller can apply the appropriate
            // fallback logic instead of silently routing incorrectly.
            if cell_nos.len() != node_labels.len() {
                return RoutePredicateEval::SourceUnknown;
            }
            RoutePredicateEval::from_bool(
                cell_nos.iter().any(|cell_no| context.visited_cell_ids.contains(cell_no))
                    == *visited,
            )
        }
        RoutePredicate::Unknown {
            ..
        } => RoutePredicateEval::Unsupported,
        RoutePredicate::FleetSize {
            op,
            value,
        } => RoutePredicateEval::from_bool(compare_route_value(context.fleet_size, *op, *value)),
        RoutePredicate::EquipmentCount {
            slotitem_types,
            op,
            value,
        } => {
            let count = context
                .ship_entries
                .iter()
                .filter(|entry| {
                    slotitem_types
                        .iter()
                        .any(|slotitem_type| entry.slotitem_types.contains(slotitem_type))
                })
                .count() as i64;
            RoutePredicateEval::from_bool(compare_route_value(count, *op, *value))
        }
        RoutePredicate::ShipTypeCount {
            ship_types,
            op,
            value,
        } => {
            let count = ship_types
                .iter()
                .map(|ship_type| {
                    context.ship_type_counts.get(ship_type).copied().unwrap_or_default()
                })
                .sum::<i64>();
            RoutePredicateEval::from_bool(compare_route_value(count, *op, *value))
        }
        RoutePredicate::FlagshipShipType {
            ship_types,
        } => RoutePredicateEval::from_bool(
            context.flagship_ship_type.is_some_and(|ship_type| ship_types.contains(&ship_type)),
        ),
        RoutePredicate::FlagshipShipId {
            ship_ids,
        } => RoutePredicateEval::from_bool(
            context.flagship_ship_id.is_some_and(|ship_id| ship_ids.contains(&ship_id)),
        ),
        RoutePredicate::ContainsShipType {
            ship_types,
        } => RoutePredicateEval::from_bool(ship_types.iter().any(|ship_type| {
            context.ship_type_counts.get(ship_type).copied().unwrap_or_default() > 0
        })),
        RoutePredicate::ContainsShipId {
            ship_ids,
        } => RoutePredicateEval::from_bool(
            ship_ids.iter().any(|ship_id| context.ship_ids.contains(ship_id)),
        ),
        RoutePredicate::ContainsShipSet {
            ship_types,
            ship_ids,
        } => RoutePredicateEval::from_bool(context.ship_entries.iter().any(|entry| {
            ship_ids.contains(&entry.ship_id) || ship_types.contains(&entry.ship_type)
        })),
        RoutePredicate::OnlyShipTypes {
            ship_types,
        } => RoutePredicateEval::from_bool(
            context
                .ship_type_counts
                .iter()
                .all(|(ship_type, count)| *count <= 0 || ship_types.contains(ship_type)),
        ),
        RoutePredicate::OnlyShipSet {
            ship_types,
            ship_ids,
        } => RoutePredicateEval::from_bool(context.ship_entries.iter().all(|entry| {
            ship_ids.contains(&entry.ship_id) || ship_types.contains(&entry.ship_type)
        })),
        RoutePredicate::ShipSetCount {
            ship_types,
            ship_ids,
            op,
            value,
        } => {
            let count = context
                .ship_entries
                .iter()
                .filter(|entry| {
                    ship_ids.contains(&entry.ship_id) || ship_types.contains(&entry.ship_type)
                })
                .count() as i64;
            RoutePredicateEval::from_bool(compare_route_value(count, *op, *value))
        }
        RoutePredicate::ShipSetSpeedCount {
            ship_types,
            ship_ids,
            speed_op,
            speed_class,
            op,
            value,
        } => {
            let count = context
                .ship_entries
                .iter()
                .filter(|entry| {
                    (ship_ids.contains(&entry.ship_id) || ship_types.contains(&entry.ship_type))
                        && compare_route_value(
                            entry.speed,
                            *speed_op,
                            speed_class_floor(*speed_class),
                        )
                })
                .count() as i64;
            RoutePredicateEval::from_bool(compare_route_value(count, *op, *value))
        }
        RoutePredicate::Speed {
            class,
        } => RoutePredicateEval::from_bool(context.min_speed >= speed_class_floor(*class)),
        RoutePredicate::LoS {
            formula,
            op,
            value,
        } => {
            let los = context.los_by_formula(formula.as_deref());
            RoutePredicateEval::from_bool(compare_route_value_f64(los, *op, *value as f64))
        }
        RoutePredicate::DrumCanisterCount {
            op,
            value,
        } => RoutePredicateEval::from_bool(compare_route_value(context.total_drums, *op, *value)),
        RoutePredicate::And(predicates) => {
            for predicate in predicates {
                match route_predicate_matches(predicate, context, stage) {
                    RoutePredicateEval::Matched => {}
                    result => return result,
                }
            }
            RoutePredicateEval::Matched
        }
        RoutePredicate::Or(predicates) => {
            let mut saw_source_unknown = false;
            let mut saw_unsupported = false;
            for predicate in predicates {
                match route_predicate_matches(predicate, context, stage) {
                    RoutePredicateEval::Matched => return RoutePredicateEval::Matched,
                    RoutePredicateEval::NotMatched => {}
                    RoutePredicateEval::SourceUnknown => saw_source_unknown = true,
                    RoutePredicateEval::Unsupported => saw_unsupported = true,
                }
            }
            if saw_source_unknown {
                RoutePredicateEval::SourceUnknown
            } else if saw_unsupported {
                RoutePredicateEval::Unsupported
            } else {
                RoutePredicateEval::NotMatched
            }
        }
        RoutePredicate::Not(predicate) => {
            match route_predicate_matches(predicate, context, stage) {
                RoutePredicateEval::Matched => RoutePredicateEval::NotMatched,
                RoutePredicateEval::NotMatched => RoutePredicateEval::Matched,
                RoutePredicateEval::SourceUnknown => RoutePredicateEval::SourceUnknown,
                RoutePredicateEval::Unsupported => RoutePredicateEval::Unsupported,
            }
        }
        RoutePredicate::SourceUnknown {
            ..
        } => RoutePredicateEval::SourceUnknown,
    }
}

fn compare_route_value(actual: i64, op: RouteOperator, expected: i64) -> bool {
    match op {
        RouteOperator::Eq => actual == expected,
        RouteOperator::Gte => actual >= expected,
        RouteOperator::Lte => actual <= expected,
    }
}

/// Floating-point variant of [`compare_route_value`] for `LoS` comparisons where
/// the computed value is a `f64` (e.g. when applying a formula).  The threshold
/// (`expected`) is truncated to `f64` before comparison.
fn compare_route_value_f64(actual: f64, op: RouteOperator, expected: f64) -> bool {
    match op {
        RouteOperator::Eq => (actual - expected).abs() < f64::EPSILON,
        RouteOperator::Gte => actual >= expected,
        RouteOperator::Lte => actual <= expected,
    }
}

fn speed_class_floor(class: SpeedClass) -> i64 {
    match class {
        SpeedClass::Slow => 5,
        SpeedClass::Fast => 10,
        SpeedClass::FastPlus => 15,
        SpeedClass::Fastest => 20,
    }
}

pub(crate) fn select_route_target_for_roll(
    weights: &BTreeMap<i64, u64>,
    mut roll: u64,
) -> Option<i64> {
    for (cell_no, weight) in weights {
        if roll < *weight {
            return Some(*cell_no);
        }
        roll -= *weight;
    }
    weights.keys().last().copied()
}

fn route_predicate_key(predicate: &RoutePredicate) -> String {
    match predicate {
        RoutePredicate::Always => "always".into(),
        RoutePredicate::VisitedNode {
            cell_nos,
            visited,
        } => {
            format!(
                "vn:{}:{}",
                cell_nos.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(","),
                visited
            )
        }
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => {
            format!("vnl:{}:{}", node_labels.join(","), visited)
        }
        RoutePredicate::FleetSize {
            op,
            value,
        } => format!("fs:{op:?}:{value}"),
        RoutePredicate::EquipmentCount {
            slotitem_types,
            op,
            value,
        } => {
            format!(
                "ec:{}:{op:?}:{value}",
                slotitem_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        RoutePredicate::ShipTypeCount {
            ship_types,
            op,
            value,
        } => {
            format!(
                "stc:{}:{op:?}:{value}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        RoutePredicate::FlagshipShipType {
            ship_types,
        } => {
            format!(
                "fst:{}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        RoutePredicate::FlagshipShipId {
            ship_ids,
        } => {
            format!(
                "fsi:{}",
                ship_ids.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(",")
            )
        }
        RoutePredicate::ContainsShipType {
            ship_types,
        } => {
            format!(
                "cst:{}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        RoutePredicate::ContainsShipId {
            ship_ids,
        } => {
            format!(
                "csi:{}",
                ship_ids.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(",")
            )
        }
        RoutePredicate::ContainsShipSet {
            ship_types,
            ship_ids,
        } => {
            format!(
                "css:{}:{}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                ship_ids.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(",")
            )
        }
        RoutePredicate::OnlyShipTypes {
            ship_types,
        } => {
            format!(
                "ost:{}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        RoutePredicate::OnlyShipSet {
            ship_types,
            ship_ids,
        } => {
            format!(
                "oss:{}:{}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                ship_ids.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(",")
            )
        }
        RoutePredicate::ShipSetCount {
            ship_types,
            ship_ids,
            op,
            value,
        } => {
            format!(
                "ssc:{}:{}:{op:?}:{value}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                ship_ids.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(",")
            )
        }
        RoutePredicate::ShipSetSpeedCount {
            ship_types,
            ship_ids,
            speed_op,
            speed_class,
            op,
            value,
        } => {
            format!(
                "sssc:{}:{}:{speed_op:?}:{speed_class:?}:{op:?}:{value}",
                ship_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                ship_ids.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(",")
            )
        }
        RoutePredicate::Speed {
            class,
        } => format!("spd:{class:?}"),
        RoutePredicate::LoS {
            formula,
            op,
            value,
        } => format!("los:{formula:?}:{op:?}:{value}"),
        RoutePredicate::DrumCanisterCount {
            op,
            value,
        } => format!("dcc:{op:?}:{value}"),
        RoutePredicate::And(preds) => {
            format!("and:{}", preds.iter().map(route_predicate_key).collect::<Vec<_>>().join("|"))
        }
        RoutePredicate::Or(preds) => {
            format!("or:{}", preds.iter().map(route_predicate_key).collect::<Vec<_>>().join("|"))
        }
        RoutePredicate::Not(pred) => format!("not:{}", route_predicate_key(pred)),
        RoutePredicate::FleetSizeWeightedRandom {
            weights,
        } => {
            format!(
                "fswr:{}",
                weights
                    .iter()
                    .map(|w| format!("{}:{}", w.fleet_size, w.probability_pct))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        RoutePredicate::Unknown {
            raw_text,
        } => format!("unk:{raw_text}"),
        RoutePredicate::SourceUnknown {
            raw_text,
        } => format!("sunk:{raw_text}"),
    }
}

impl RoutePredicateEval {
    fn from_bool(value: bool) -> Self {
        if value {
            Self::Matched
        } else {
            Self::NotMatched
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_has_routing_outgoing_next_cells_only() {
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3])],
            ..Default::default()
        };
        assert!(cell_has_routing_outgoing(1, &stage));
    }

    #[test]
    fn cell_has_routing_outgoing_routing_rules_only() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2,
                priority: 0,
                predicate: RoutePredicate::Always,
                ..Default::default()
            }],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![])],
            routing_rules,
            ..Default::default()
        };
        assert!(cell_has_routing_outgoing(1, &stage));
    }

    #[test]
    fn cell_has_routing_outgoing_both() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2,
                priority: 0,
                predicate: RoutePredicate::Always,
                ..Default::default()
            }],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![3])],
            routing_rules,
            ..Default::default()
        };
        assert!(cell_has_routing_outgoing(1, &stage));
    }

    #[test]
    fn cell_has_routing_outgoing_neither() {
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![])],
            ..Default::default()
        };
        assert!(!cell_has_routing_outgoing(1, &stage));
    }

    #[test]
    fn cell_has_routing_outgoing_cell_not_found() {
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2])],
            ..Default::default()
        };
        assert!(!cell_has_routing_outgoing(99, &stage));
    }

    #[test]
    fn visited_node_label_matches_when_visited() {
        let stage = MapStageDefinition {
            cells: vec![MapCellDefinition {
                cell_no: 2,
                node_label: Some("A".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let context = FleetRouteContext {
            visited_cell_ids: BTreeSet::from([2]),
            ..Default::default()
        };
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNodeLabel {
                    node_labels: vec!["A".to_string()],
                    visited: true,
                },
                &context,
                &stage,
            ),
            RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn visited_node_label_source_unknown_when_label_missing() {
        // Label "A" does not exist in the stage graph → SourceUnknown, not NotMatched.
        let stage = MapStageDefinition {
            cells: vec![MapCellDefinition {
                cell_no: 2,
                node_label: Some("B".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let context = FleetRouteContext {
            visited_cell_ids: BTreeSet::from([2]),
            ..Default::default()
        };
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNodeLabel {
                    node_labels: vec!["A".to_string()],
                    visited: true,
                },
                &context,
                &stage,
            ),
            RoutePredicateEval::SourceUnknown
        ));
    }

    #[test]
    fn visited_node_label_matches_visited_false() {
        let stage = MapStageDefinition {
            cells: vec![MapCellDefinition {
                cell_no: 2,
                node_label: Some("A".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let context = FleetRouteContext {
            visited_cell_ids: BTreeSet::new(),
            ..Default::default()
        };
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNodeLabel {
                    node_labels: vec!["A".to_string()],
                    visited: false,
                },
                &context,
                &stage,
            ),
            RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn select_route_target_roll_equals_total_weight_returns_last_key() {
        let mut weights = BTreeMap::new();
        weights.insert(2, 30);
        weights.insert(5, 50);
        weights.insert(7, 20);

        let total: u64 = weights.values().sum();
        let result = select_route_target_for_roll(&weights, total);
        assert_eq!(result, Some(7), "roll == total weight should return last key");
    }

    #[test]
    fn select_route_target_roll_zero_returns_first_key() {
        let mut weights = BTreeMap::new();
        weights.insert(2, 30);
        weights.insert(5, 50);

        let result = select_route_target_for_roll(&weights, 0);
        assert_eq!(result, Some(2));
    }

    #[test]
    fn select_route_target_roll_within_first_weight() {
        let mut weights = BTreeMap::new();
        weights.insert(2, 30);
        weights.insert(5, 50);

        let result = select_route_target_for_roll(&weights, 25);
        assert_eq!(result, Some(2));
    }

    #[test]
    fn select_route_target_roll_at_boundary() {
        let mut weights = BTreeMap::new();
        weights.insert(2, 30);
        weights.insert(5, 50);
        weights.insert(7, 20);

        let result = select_route_target_for_roll(&weights, 30);
        assert_eq!(result, Some(5));
    }

    // --- LoS formula helpers ---

    /// Build a minimal `FleetRouteContext` for `LoS` tests.
    /// `los_total` is the raw sum.
    /// `los_formula1` and `los_formula3` are supplied explicitly so tests can
    /// set different values to verify formula dispatch.
    fn make_los_context(los_total: i64, los_formula1: f64, los_formula3: f64) -> FleetRouteContext {
        FleetRouteContext {
            fleet_size: 6,
            los_total,
            los_formula1,
            los_formula3,
            ..Default::default()
        }
    }

    fn make_los_stage() -> MapStageDefinition {
        MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            ..Default::default()
        }
    }

    fn make_cell(cell_no: i64, next_cells: Vec<i64>) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            next_cells,
            ..Default::default()
        }
    }

    fn make_unknown_rule(from_cell_no: i64, to_cell_no: i64, priority: i64) -> RouteRule {
        RouteRule {
            from_cell_no,
            to_cell_no,
            priority,
            weight: Some(1),
            predicate: RoutePredicate::Unknown {
                raw_text: String::new(),
            },
            ..Default::default()
        }
    }

    fn make_source_unknown_rule(from_cell_no: i64, to_cell_no: i64, priority: i64) -> RouteRule {
        RouteRule {
            from_cell_no,
            to_cell_no,
            priority,
            weight: Some(1),
            predicate: RoutePredicate::SourceUnknown {
                raw_text: String::new(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn unknown_rules_fallback_to_random_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_unknown_rule(1, 3, 0), make_unknown_rule(1, 5, 1)]);

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![3, 4]), make_cell(3, vec![]), make_cell(4, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![3, 4]);
        let context = FleetRouteContext::default();

        let mut found_3 = false;
        let mut found_4 = false;
        for _ in 0..20 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            assert!(result == 3 || result == 4, "result should be 3 or 4, got {result}");
            if result == 3 {
                found_3 = true;
            }
            if result == 4 {
                found_4 = true;
            }
        }
        assert!(found_3, "should have routed to cell 3 at least once");
        assert!(found_4, "should have routed to cell 4 at least once");
    }

    #[test]
    fn source_unknown_rules_fallback_to_random_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![make_source_unknown_rule(1, 10, 0), make_source_unknown_rule(1, 20, 1)],
        );

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![7, 8]), make_cell(7, vec![]), make_cell(8, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![7, 8]);
        let context = FleetRouteContext::default();

        let mut found_7 = false;
        let mut found_8 = false;
        for _ in 0..20 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            assert!(result == 7 || result == 8, "result should be 7 or 8, got {result}");
            if result == 7 {
                found_7 = true;
            }
            if result == 8 {
                found_8 = true;
            }
        }
        assert!(found_7, "should have routed to cell 7 at least once");
        assert!(found_8, "should have routed to cell 8 at least once");
    }

    #[test]
    fn unknown_rules_accept_selected_cell_id_in_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_unknown_rule(1, 3, 0)]);

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![3, 5]), make_cell(3, vec![]), make_cell(5, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![3, 5]);
        let context = FleetRouteContext::default();

        let result = evaluate_route_destination(&current, &stage, &context, Some(5)).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn unknown_rules_no_rules_match_and_no_always_uses_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_unknown_rule(1, 3, 10), make_unknown_rule(1, 5, 10)]);

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 4]), make_cell(2, vec![]), make_cell(4, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![2, 4]);
        let context = FleetRouteContext::default();

        let mut found_2 = false;
        let mut found_4 = false;
        for _ in 0..20 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            if result == 2 {
                found_2 = true;
            }
            if result == 4 {
                found_4 = true;
            }
        }
        assert!(found_2, "should have routed to cell 2 at least once");
        assert!(found_4, "should have routed to cell 4 at least once");
    }

    #[test]
    fn indeterminate_rules_fallback_to_next_cells_when_multiple_unconditional() {
        let mut routing_rules = BTreeMap::new();
        // One executable Always rule to cell 3, one Unknown rule to cell 5
        routing_rules.insert(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 0,
                    weight: Some(1),
                    predicate: RoutePredicate::Always,
                    ..Default::default()
                },
                make_unknown_rule(1, 5, 1),
            ],
        );

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![3, 4]), make_cell(3, vec![]), make_cell(4, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![3, 4]);
        let context = FleetRouteContext::default();

        // Should route to cell 3 (the single unconditional target)
        let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn source_unknown_rejects_selected_cell_not_in_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![make_source_unknown_rule(1, 10, 0), make_source_unknown_rule(1, 20, 1)],
        );

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![7, 8]), make_cell(7, vec![]), make_cell(8, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![7, 8]);
        let context = FleetRouteContext::default();

        // selected_cell_id 10 is in rule targets but NOT in next_cells.
        // The stricter check now falls back to select_route_from_cells (random from next_cells).
        let result = evaluate_route_destination(&current, &stage, &context, Some(10));
        let cell_no = result.unwrap();
        assert!(cell_no == 7 || cell_no == 8, "should fall back to next_cells, got {cell_no}");
    }

    #[test]
    fn source_unknown_accepts_selected_cell_in_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules
            .insert(1, vec![make_source_unknown_rule(1, 10, 0), make_source_unknown_rule(1, 7, 1)]);

        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![7, 8]), make_cell(7, vec![]), make_cell(8, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![7, 8]);
        let context = FleetRouteContext::default();

        // selected_cell_id 7 is in both rule targets and next_cells.
        let result = evaluate_route_destination(&current, &stage, &context, Some(7));
        assert_eq!(result.unwrap(), 7);
    }

    // --- LoS formula dispatch tests ---

    #[test]
    fn los_formula_none_uses_los_total() {
        // formula: None should fall back to los_total regardless of the precomputed values.
        let ctx = make_los_context(80, 50.0, 40.0);
        let stage = make_los_stage();
        let eval = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: None,
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: 80,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval, RoutePredicateEval::Matched),
            "formula=None should use los_total=80, threshold=80"
        );
        // Also verify threshold just above fails
        let eval_fail = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: None,
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: 81,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval_fail, RoutePredicateEval::NotMatched),
            "formula=None los_total=80 should not meet threshold=81"
        );
    }

    #[test]
    fn los_formula1_uses_precomputed_formula1() {
        // formula "式1" routes to los_formula1 (50.0), not los_total (80).
        let ctx = make_los_context(80, 50.0, 40.0);
        let stage = make_los_stage();
        let eval = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: Some("式1".to_string()),
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: 50,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval, RoutePredicateEval::Matched),
            "formula=式1 should use los_formula1=50, threshold=50"
        );
        // Threshold above formula1 value but below los_total should NOT match.
        let eval_fail = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: Some("式1".to_string()),
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: 51,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval_fail, RoutePredicateEval::NotMatched),
            "formula=式1 los_formula1=50 should not meet threshold=51"
        );
    }

    #[test]
    fn los_formula3_uses_precomputed_formula3() {
        // formula "式3" routes to los_formula3 (40.0).  Same fleet that passes
        // formula 1 (50.0 >= 45) may fail formula 3 (40.0 < 45).
        let ctx = make_los_context(80, 50.0, 40.0);
        let stage = make_los_stage();

        let threshold = 45;

        // Formula 1 passes (50 >= 45)
        let eval_f1 = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: Some("式1".to_string()),
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: threshold,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval_f1, RoutePredicateEval::Matched),
            "formula=式1 50.0 >= 45 should match"
        );

        // Formula 3 fails (40 < 45) — different result for same fleet and threshold
        let eval_f3 = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: Some("式3".to_string()),
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: threshold,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval_f3, RoutePredicateEval::NotMatched),
            "formula=式3 40.0 < 45 should not match"
        );
    }

    #[test]
    fn los_unknown_formula_falls_back_to_los_total() {
        // An unknown formula string (e.g. "式9") should fall back to los_total.
        let ctx = make_los_context(100, 60.0, 50.0);
        let stage = make_los_stage();
        let eval = route_predicate_matches(
            &RoutePredicate::LoS {
                formula: Some("式9".to_string()),
                op: emukc_model::codex::map::RouteOperator::Gte,
                value: 100,
            },
            &ctx,
            &stage,
        );
        assert!(
            matches!(eval, RoutePredicateEval::Matched),
            "unknown formula should fall back to los_total=100, threshold=100"
        );
    }

    #[test]
    fn visited_node_label_source_unknown_when_label_absent_from_graph() {
        // Label "Z" is not present in the stage at all → SourceUnknown.
        let stage = MapStageDefinition {
            cells: vec![MapCellDefinition {
                cell_no: 1,
                node_label: Some("A".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let context = FleetRouteContext {
            visited_cell_ids: BTreeSet::from([1]),
            ..Default::default()
        };
        let eval = route_predicate_matches(
            &RoutePredicate::VisitedNodeLabel {
                node_labels: vec!["Z".to_string()],
                visited: true,
            },
            &context,
            &stage,
        );
        assert!(
            matches!(eval, RoutePredicateEval::SourceUnknown),
            "unresolvable label should yield SourceUnknown, got {eval:?}"
        );
    }

    #[test]
    fn visited_node_label_not_matched_when_label_resolves_but_not_visited() {
        // Label resolves but the cell has not been visited and visited=true → NotMatched.
        let stage = MapStageDefinition {
            cells: vec![MapCellDefinition {
                cell_no: 3,
                node_label: Some("C".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let context = FleetRouteContext {
            visited_cell_ids: BTreeSet::new(), // cell 3 not visited
            ..Default::default()
        };
        let eval = route_predicate_matches(
            &RoutePredicate::VisitedNodeLabel {
                node_labels: vec!["C".to_string()],
                visited: true,
            },
            &context,
            &stage,
        );
        assert!(
            matches!(eval, RoutePredicateEval::NotMatched),
            "resolved label, not visited, visited=true → NotMatched"
        );
    }

    // --- EquipmentCount evaluator tests ---
    //
    // Corpus evidence: all wikiwiki phrases producing EquipmentCount use either
    //   「電探を装備した艦が N 隻以上/以下」  (ships carrying the named item type)
    // or
    //   「搭載艦の隻数が N 隻以上/以下」     (ships carrying the named item type)
    //
    // Both phrase patterns count *ships*, not individual items.  The evaluator
    // iterates FleetRouteShipEntry and counts entries whose slotitem_types set
    // contains at least one matching type — which is precisely ship-count
    // semantics.  No item-count variant (「電探を N 個以上装備」) was found in
    // the wikiwiki catalog or parser unit tests, so EquipmentCount remains a
    // single ship-count predicate.

    /// Fleet: [ship(radar), ship(no radar), ship(radar + radar)].
    /// `slotitem_types` is a `BTreeSet` so the two-radar ship contributes type 12
    /// only once.  The predicate counts *ships*, so the result is 2, not 3 or 4.
    /// This is the canonical ship-count vs item-count distinction test.
    #[test]
    fn equipment_count_counts_ships_not_items() {
        let context = FleetRouteContext {
            fleet_size: 3,
            ship_entries: vec![
                FleetRouteShipEntry {
                    slotitem_types: BTreeSet::from([12]), // one radar
                    ..Default::default()
                },
                FleetRouteShipEntry {
                    slotitem_types: BTreeSet::new(), // no radar
                    ..Default::default()
                },
                FleetRouteShipEntry {
                    // Two radars of the same type; the set deduplicates them.
                    // If the evaluator counted items it would see 3 here, but
                    // ship-count semantics give 2 across the whole fleet.
                    slotitem_types: BTreeSet::from([12]),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        // Exactly 2 ships carry radar (type 12 falls in [12, 13, 93]).
        assert!(
            matches!(
                route_predicate_matches(
                    &RoutePredicate::EquipmentCount {
                        slotitem_types: vec![12, 13, 93],
                        op: RouteOperator::Eq,
                        value: 2,
                    },
                    &context,
                    &MapStageDefinition::default(),
                ),
                RoutePredicateEval::Matched
            ),
            "ship-count should be 2 regardless of how many radars each ship carries"
        );
        // Sanity: Gte(2) also matches, Gte(3) does not.
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Gte,
                    value: 2,
                },
                &context,
                &MapStageDefinition::default(),
            ),
            RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Gte,
                    value: 3,
                },
                &context,
                &MapStageDefinition::default(),
            ),
            RoutePredicateEval::NotMatched
        ));
    }

    /// Empty fleet → count is 0; Eq(0) matches, Gte(1) does not.
    #[test]
    fn equipment_count_empty_fleet_is_zero() {
        let context = FleetRouteContext {
            fleet_size: 0,
            ship_entries: vec![],
            ..Default::default()
        };
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Eq,
                    value: 0,
                },
                &context,
                &MapStageDefinition::default(),
            ),
            RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Gte,
                    value: 1,
                },
                &context,
                &MapStageDefinition::default(),
            ),
            RoutePredicateEval::NotMatched
        ));
    }

    /// A ship with an empty `slotitem_types` set (zero equipment slots filled)
    /// must not be counted, even if the predicate requests type 12.
    #[test]
    fn equipment_count_ship_with_no_slots_does_not_count() {
        let context = FleetRouteContext {
            fleet_size: 2,
            ship_entries: vec![
                FleetRouteShipEntry {
                    slotitem_types: BTreeSet::new(), // nothing equipped
                    ..Default::default()
                },
                FleetRouteShipEntry {
                    slotitem_types: BTreeSet::new(), // nothing equipped
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Eq,
                    value: 0,
                },
                &context,
                &MapStageDefinition::default(),
            ),
            RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Gte,
                    value: 1,
                },
                &context,
                &MapStageDefinition::default(),
            ),
            RoutePredicateEval::NotMatched
        ));
    }

    /// Rule targets cell 2 but `next_cells`=[4,5] (cell 2 not in `next_cells`).
    /// After topology filter, `candidate_targets` is empty → should fall back to
    /// random selection from `next_cells`, not return an error.
    #[test]
    fn rules_filtered_by_topology_fallback_to_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2,
                priority: 0,
                weight: Some(1),
                predicate: RoutePredicate::Always,
                ..Default::default()
            }],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![4, 5]), make_cell(4, vec![]), make_cell(5, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![4, 5]);
        let context = FleetRouteContext::default();

        let mut found_4 = false;
        let mut found_5 = false;
        for _ in 0..20 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            assert!(
                result == 4 || result == 5,
                "fallback should pick from next_cells, got {result}"
            );
            if result == 4 {
                found_4 = true;
            }
            if result == 5 {
                found_5 = true;
            }
        }
        assert!(found_4, "should have routed to cell 4 at least once");
        assert!(found_5, "should have routed to cell 5 at least once");
    }

    /// Rules filtered by topology, `next_cells` also empty → error via `select_route_from_cells`.
    #[test]
    fn rules_filtered_by_topology_and_empty_next_cells_returns_error() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2,
                priority: 0,
                weight: Some(1),
                predicate: RoutePredicate::Always,
                ..Default::default()
            }],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![]);
        let context = FleetRouteContext::default();

        let result = evaluate_route_destination(&current, &stage, &context, None);
        assert!(result.is_err(), "empty next_cells should return error");
    }

    // ========================================================================
    // Route evaluation integration tests (U1)
    // ========================================================================

    /// Helper: build a `RouteRule` with `LoS` predicate.
    fn make_los_rule(
        from: i64,
        to: i64,
        priority: i64,
        formula: Option<&str>,
        op: RouteOperator,
        value: i64,
    ) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            priority,
            weight: Some(1),
            predicate: RoutePredicate::LoS {
                formula: formula.map(String::from),
                op,
                value,
            },
            ..Default::default()
        }
    }

    /// Helper: build a `RouteRule` with `FleetSize` predicate.
    fn make_fleet_size_rule(
        from: i64,
        to: i64,
        priority: i64,
        op: RouteOperator,
        value: i64,
    ) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            priority,
            weight: Some(1),
            predicate: RoutePredicate::FleetSize {
                op,
                value,
            },
            ..Default::default()
        }
    }

    /// Helper: build a `RouteRule` with Always predicate.
    fn make_always_rule(from: i64, to: i64, priority: i64, weight: i64) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            priority,
            weight: Some(weight),
            predicate: RoutePredicate::Always,
            ..Default::default()
        }
    }

    // --- Happy path tests ---

    /// High-priority rule matches; lower-priority rule is ignored.
    /// Cell 1 routes to {2, 3}. High priority (0) targets cell 2 (`FleetSize` >= 6),
    /// low priority (5) targets cell 3 (`FleetSize` >= 1).
    /// With `fleet_size=6` the high-priority rule wins → cell 2.
    #[test]
    fn multi_condition_rules_select_higher_priority() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![
                make_fleet_size_rule(1, 2, 0, RouteOperator::Gte, 6),
                make_fleet_size_rule(1, 3, 5, RouteOperator::Gte, 1),
            ],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3]);
        let context = FleetRouteContext {
            fleet_size: 6,
            ..Default::default()
        };

        let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
        assert_eq!(result, 2, "high priority rule should route to cell 2");
    }

    /// `LoS` branching: different `LoS` values route to different target cells.
    /// Cell 1 routes to {2, 3}:
    ///   - `LoS` >= 60 → cell 2
    ///   - `LoS` >= 30 → cell 3
    ///
    /// With `los_total=50`, only the second rule matches → cell 3.
    #[test]
    fn los_branching_routes_by_los_value() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![
                make_los_rule(1, 2, 1, None, RouteOperator::Gte, 60),
                make_los_rule(1, 3, 2, None, RouteOperator::Gte, 30),
            ],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3]);

        // LoS 50: fails threshold 60 (cell 2), passes threshold 30 (cell 3)
        let context = make_los_context(50, 50.0, 50.0);
        let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
        assert_eq!(result, 3, "los=50 should route to cell 3 (threshold 30)");

        // LoS 70: passes threshold 60 (cell 2)
        let context_high = make_los_context(70, 70.0, 70.0);
        let result_high =
            evaluate_route_destination(&current, &stage, &context_high, None).unwrap();
        assert_eq!(result_high, 2, "los=70 should route to cell 2 (threshold 60)");
    }

    /// Rules `from_cell` matching: only rules matching the current cell are evaluated.
    /// Cell 1 has rules for cell 1 (→2) and cell 3 (→6). Being at cell 1,
    /// only the rule for cell 1 applies.
    #[test]
    fn rule_from_cell_matching_selects_correct_rule() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_fleet_size_rule(1, 2, 0, RouteOperator::Gte, 1)]);
        routing_rules.insert(3, vec![make_fleet_size_rule(3, 6, 0, RouteOperator::Gte, 1)]);
        let stage = MapStageDefinition {
            cells: vec![
                make_cell(1, vec![2]),
                make_cell(2, vec![]),
                make_cell(3, vec![6]),
                make_cell(6, vec![]),
            ],
            routing_rules,
            ..Default::default()
        };

        // At cell 1 → rule for cell 1 fires → cell 2
        let current_1 = make_cell(1, vec![2]);
        let context = FleetRouteContext {
            fleet_size: 4,
            ..Default::default()
        };
        let result = evaluate_route_destination(&current_1, &stage, &context, None).unwrap();
        assert_eq!(result, 2, "at cell 1 should route to cell 2");

        // At cell 3 → rule for cell 3 fires → cell 6
        let current_3 = make_cell(3, vec![6]);
        let result = evaluate_route_destination(&current_3, &stage, &context, None).unwrap();
        assert_eq!(result, 6, "at cell 3 should route to cell 6");
    }

    // --- Edge case tests ---

    /// All rules filtered by topology → fallback to `select_route_from_cells`.
    /// Cell 1 has `next_cells` {4, 5} but rule targets cell 2 (not in `next_cells`).
    /// After topology filter, no `candidate_targets` remain → fall back to `next_cells`.
    /// Verifies the result is one of {4, 5} and both are reachable over 20 trials.
    #[test]
    fn all_rules_filtered_by_topology_falls_back_to_next_cells() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2, // not in next_cells
                priority: 0,
                weight: Some(1),
                predicate: RoutePredicate::FleetSize {
                    op: RouteOperator::Gte,
                    value: 1,
                },
                ..Default::default()
            }],
        );
        let stage = MapStageDefinition {
            cells: vec![
                make_cell(1, vec![4, 5]),
                make_cell(2, vec![]),
                make_cell(4, vec![]),
                make_cell(5, vec![]),
            ],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![4, 5]);
        let context = FleetRouteContext {
            fleet_size: 6,
            ..Default::default()
        };

        let mut found_4 = false;
        let mut found_5 = false;
        for _ in 0..20 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            assert!(
                result == 4 || result == 5,
                "fallback should pick from next_cells {{4,5}}, got {result}"
            );
            if result == 4 {
                found_4 = true;
            }
            if result == 5 {
                found_5 = true;
            }
        }
        assert!(found_4, "should have routed to cell 4 at least once");
        assert!(found_5, "should have routed to cell 5 at least once");
    }

    /// Empty `next_cells` with no rule match → returns error.
    /// Cell 1 has empty `next_cells` and a `LoS` rule that doesn't match.
    #[test]
    fn empty_next_cells_no_rule_match_returns_error() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_los_rule(1, 2, 0, None, RouteOperator::Gte, 100)]);
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![]);
        // LoS is 10, threshold is 100 → rule doesn't match, next_cells is empty
        let context = make_los_context(10, 10.0, 10.0);

        let result = evaluate_route_destination(&current, &stage, &context, None);
        assert!(result.is_err(), "empty next_cells with no rule match should error");
    }

    /// Multiple rules with same priority and weight → weighted random selection.
    /// Cell 1 routes to {2, 3}. Two Always rules with equal priority (0) and
    /// different weights: cell 2 gets weight 80, cell 3 gets weight 20.
    /// Over 100 trials, both should appear and cell 2 should dominate.
    #[test]
    fn same_priority_rules_use_weighted_random() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_always_rule(1, 2, 0, 80), make_always_rule(1, 3, 0, 20)]);
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3]);
        let context = FleetRouteContext::default();

        let mut count_2 = 0usize;
        let mut count_3 = 0usize;
        for _ in 0..100 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            assert!(result == 2 || result == 3, "result should be 2 or 3, got {result}");
            if result == 2 {
                count_2 += 1;
            } else {
                count_3 += 1;
            }
        }
        assert!(count_2 > 0, "cell 2 should appear at least once");
        assert!(count_3 > 0, "cell 3 should appear at least once");
        // With 80:20 weights over 100 trials, cell 2 should dominate.
        assert!(
            count_2 > count_3,
            "cell 2 (weight 80) should appear more than cell 3 (weight 20), got {count_2} vs {count_3}"
        );
    }

    /// Route rule references `to_cell_no` not in `next_cells` but exists in cells.
    /// Cell 1 has `next_cells` {4, 5}. Rule targets cell 2 which exists in the stage
    /// but is not in `next_cells`. Topology filter excludes cell 2, falling back
    /// to `next_cells`. Verify both {4, 5} are reachable.
    #[test]
    fn rule_to_cell_exists_in_cells_but_not_next_cells_is_filtered() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(1, vec![make_fleet_size_rule(1, 2, 0, RouteOperator::Gte, 1)]);
        let stage = MapStageDefinition {
            cells: vec![
                make_cell(1, vec![4, 5]),
                make_cell(2, vec![]), // exists in cells but not in cell 1's next_cells
                make_cell(4, vec![]),
                make_cell(5, vec![]),
            ],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![4, 5]);
        let context = FleetRouteContext {
            fleet_size: 6,
            ..Default::default()
        };

        let mut found_4 = false;
        let mut found_5 = false;
        for _ in 0..20 {
            let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
            assert!(
                result == 4 || result == 5,
                "topology filter should exclude cell 2, got {result}"
            );
            if result == 4 {
                found_4 = true;
            }
            if result == 5 {
                found_5 = true;
            }
        }
        assert!(found_4, "should have routed to cell 4 at least once");
        assert!(found_5, "should have routed to cell 5 at least once");
    }

    // --- Integration test ---

    /// Simulated map stage: cell 1→{4,5}, cell 3→{6}.
    /// Routing rules for cell 1:
    ///   - `LoS` >= 40 → cell 5 (priority 0)
    ///   - Always → cell 4 (priority 1, acts as fallback)
    ///
    /// Routing rules for cell 3:
    ///   - `FleetSize` >= 4 → cell 6 (priority 0)
    ///
    /// Tests conditional routing + topology fallback combined behavior.
    /// When `LoS` is high, the `LoS` rule (priority 0) wins. When `LoS` is low,
    /// the Always rule (priority 1) acts as fallback → cell 4.
    #[test]
    fn simulated_map_stage_conditional_routing_and_topology_fallback() {
        let mut routing_rules = BTreeMap::new();
        // Cell 1: high LoS routes to cell 5, Always fallback routes to cell 4
        routing_rules.insert(
            1,
            vec![
                make_los_rule(1, 5, 0, None, RouteOperator::Gte, 40),
                make_always_rule(1, 4, 1, 1),
            ],
        );
        // Cell 3: fleet size >= 4 routes to cell 6
        routing_rules.insert(3, vec![make_fleet_size_rule(3, 6, 0, RouteOperator::Gte, 4)]);
        let stage = MapStageDefinition {
            cells: vec![
                make_cell(1, vec![4, 5]),
                make_cell(3, vec![6]),
                make_cell(4, vec![]),
                make_cell(5, vec![]),
                make_cell(6, vec![]),
            ],
            routing_rules,
            ..Default::default()
        };

        // --- At cell 1, high LoS (50 >= 40) → cell 5 ---
        let current_1 = make_cell(1, vec![4, 5]);
        let ctx_high_los = make_los_context(50, 50.0, 50.0);
        let result = evaluate_route_destination(&current_1, &stage, &ctx_high_los, None).unwrap();
        assert_eq!(result, 5, "high LoS at cell 1 should route to cell 5");

        // --- At cell 1, low LoS (20 < 40) → Always fallback rule at priority 1 → cell 4 ---
        let ctx_low_los = make_los_context(20, 20.0, 20.0);
        let result = evaluate_route_destination(&current_1, &stage, &ctx_low_los, None).unwrap();
        assert_eq!(result, 4, "low LoS at cell 1 should use Always fallback to cell 4");

        // --- At cell 3, fleet size 6 (>= 4) → cell 6 ---
        let current_3 = make_cell(3, vec![6]);
        let ctx_fleet = FleetRouteContext {
            fleet_size: 6,
            ..Default::default()
        };
        let result = evaluate_route_destination(&current_3, &stage, &ctx_fleet, None).unwrap();
        assert_eq!(result, 6, "cell 3 with fleet 6 should route to cell 6");

        // --- At cell 3, fleet size 2 (< 4) → no rule match → single next_cell 6 ---
        let ctx_small_fleet = FleetRouteContext {
            fleet_size: 2,
            ..Default::default()
        };
        // Cell 3 has a FleetSize rule that doesn't match (2 < 4) and no Always fallback.
        // But cell 3's next_cells is [6], so the topology-only path still works
        // because no rules exist that produce matched_groups — the Always rule
        // for cell 3 is absent, so it falls through to select_route_from_cells.
        // However, evaluate_route_destination only calls select_route_from_cells
        // when there are NO rules for the cell. Since there IS a rule (it just
        // didn't match), it will error. This tests the error path.
        let err_result = evaluate_route_destination(&current_3, &stage, &ctx_small_fleet, None);
        assert!(
            err_result.is_err(),
            "cell 3 with fleet 2 should error: rule exists but doesn't match"
        );
    }

    // --- evaluate_route_candidate_count tests ---

    #[test]
    fn candidate_count_single_always_rule_returns_1() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2,
                priority: 0,
                weight: Some(1),
                predicate: RoutePredicate::Always,
                ..Default::default()
            }],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3]);
        let context = FleetRouteContext::default();

        let count = evaluate_route_candidate_count(&current, &stage, &context);
        assert_eq!(count, 1);
    }

    #[test]
    fn candidate_count_multiple_always_rules_returns_2() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: Some(1),
                    predicate: RoutePredicate::Always,
                    ..Default::default()
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 0,
                    weight: Some(1),
                    predicate: RoutePredicate::Always,
                    ..Default::default()
                },
            ],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3]);
        let context = FleetRouteContext::default();

        let count = evaluate_route_candidate_count(&current, &stage, &context);
        assert_eq!(count, 2);
    }

    #[test]
    fn candidate_count_rules_filtered_by_topology() {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: Some(1),
                    predicate: RoutePredicate::Always,
                    ..Default::default()
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 5,
                    priority: 0,
                    weight: Some(1),
                    predicate: RoutePredicate::Always,
                    ..Default::default()
                },
            ],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2]), make_cell(2, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2]);
        let context = FleetRouteContext::default();

        let count = evaluate_route_candidate_count(&current, &stage, &context);
        assert_eq!(count, 1);
    }

    #[test]
    fn candidate_count_no_rules_falls_back_to_next_cells_len() {
        let stage = MapStageDefinition {
            cells: vec![
                make_cell(1, vec![2, 3, 4]),
                make_cell(2, vec![]),
                make_cell(3, vec![]),
                make_cell(4, vec![]),
            ],
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3, 4]);
        let context = FleetRouteContext::default();

        let count = evaluate_route_candidate_count(&current, &stage, &context);
        assert_eq!(count, 3);
    }
}
