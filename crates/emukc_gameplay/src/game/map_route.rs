use std::collections::{BTreeMap, BTreeSet};

use emukc_crypto::rng;
use emukc_model::codex::map::{
    MapCellDefinition, MapStageDefinition, RouteCounter, RouteOperator, RoutePredicate, RouteRule,
    SpeedClass,
};

use crate::err::GameplayError;

#[derive(Debug, Clone, Default)]
pub(crate) struct FleetRouteShipEntry {
    pub(crate) ship_id: i64,
    pub(crate) ship_type: i64,
    pub(crate) speed: i64,
    pub(crate) slotitem_types: BTreeSet<i64>,
    /// Master ids of the equipment carried.
    pub(crate) slotitem_ids: BTreeSet<i64>,
    /// The ship's own speed, before equipment, is slow.
    pub(crate) base_slow: bool,
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
    /// Ships carrying at least one drum canister — not the canister count.
    pub(crate) drum_ships: i64,
    /// The part of the formula-33 `LoS` score that does not depend on the branch point:
    /// `Σ√(ship's own LoS) − ⌈0.4 × HQ level⌉ + 2 × (6 − fleet size)`.
    pub(crate) los_ship_term: f64,
    /// The part the branch-point coefficient multiplies:
    /// `Σ equipment coefficient × (equipment LoS + improvement bonus)`.
    pub(crate) los_equip_term: f64,
    /// 第2艦隊's ships when a combined fleet sorties; empty otherwise. Kept apart
    /// from every field above, which describe 第1艦隊 alone, so no existing
    /// predicate changes its answer. None reads it yet: the regular maps have no
    /// combined-fleet branches (`sally_flag` is `[x, 0, 0]` on all of them).
    pub(crate) escort_ship_entries: Vec<FleetRouteShipEntry>,
}

impl FleetRouteContext {
    /// The formula-33 `LoS` score at a branch point with the given coefficient, floored.
    ///
    /// Thresholds are whole numbers read as 「N 以上」 / 「N 未満」, stored as `Gte N` /
    /// `Lte N-1`; flooring the score first keeps a fractional score from falling between them.
    pub(crate) fn los_score(&self, coefficient: i64) -> i64 {
        (self.los_ship_term + coefficient as f64 * self.los_equip_term).floor() as i64
    }

    fn count(&self, counter: &RouteCounter) -> i64 {
        let ships = |matches: &dyn Fn(&FleetRouteShipEntry) -> bool| {
            self.ship_entries.iter().filter(|entry| matches(entry)).count() as i64
        };
        match counter {
            RouteCounter::ShipTypes(ship_types) => {
                ships(&|entry| ship_types.contains(&entry.ship_type))
            }
            RouteCounter::Ships(ship_ids) => ships(&|entry| ship_ids.contains(&entry.ship_id)),
            RouteCounter::FleetSize => self.fleet_size,
            RouteCounter::EquipCarriers {
                slotitem_ids,
            } => ships(&|entry| slotitem_ids.iter().any(|id| entry.slotitem_ids.contains(id))),
            RouteCounter::SlowShips {
                ship_types,
            } => ships(&|entry| entry.base_slow && ship_types.contains(&entry.ship_type)),
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

pub(crate) fn evaluate_route_destination(
    current: &MapCellDefinition,
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
    selected_cell_id: Option<i64>,
) -> Result<i64, GameplayError> {
    let Some(rules) = stage.routing_rules.get(&current.cell_no).filter(|rules| !rules.is_empty())
    else {
        return select_route_from_cells(current, selected_cell_id);
    };

    let FiringRules {
        executable,
        saw_source_unknown,
        saw_unsupported,
    } = firing_rules(rules, context, stage);

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
            return select_route_from_cells(current, None);
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
            return select_route_from_cells(current, selected_cell_id);
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
        return select_route_from_cells(current, selected_cell_id);
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

    let weights = rule_weights(&executable, context);
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

struct FiringRules<'a> {
    /// The rules that decide this roll.
    executable: Vec<&'a RouteRule>,
    saw_source_unknown: bool,
    saw_unsupported: bool,
}

/// The rules that fire for this fleet: of the conditional rules that match, the group with
/// the lowest priority; the unconditional rules when none does.
fn firing_rules<'a>(
    rules: &'a [RouteRule],
    context: &FleetRouteContext,
    stage: &MapStageDefinition,
) -> FiringRules<'a> {
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
    FiringRules {
        executable,
        saw_source_unknown,
        saw_unsupported,
    }
}

/// Roll weight per target cell of the firing rules.
fn rule_weights(executable: &[&RouteRule], context: &FleetRouteContext) -> BTreeMap<i64, u64> {
    executable.iter().fold(BTreeMap::<i64, u64>::new(), |mut acc, rule| {
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
    })
}

/// Pick the start cell the fleet sorties from when the map has several and its start
/// rules decide. `None` when the variant has no start rules or none of them fires.
pub(crate) fn select_start_cell(
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
) -> Option<i64> {
    let weights = start_cell_weights(stage, context);
    let total_weight = weights.values().sum::<u64>();
    if total_weight == 0 {
        return None;
    }
    select_route_target_for_roll(&weights, rng::u64(0..total_weight))
}

fn start_cell_weights(
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
) -> BTreeMap<i64, u64> {
    rule_weights(&firing_rules(&stage.start_rules, context, stage).executable, context)
}

/// Where the fleet can go from `current` and how likely each cell is, without rolling.
///
/// Follows [`evaluate_route_destination`] for a client that sends no choice: when no rule
/// decides, every next cell is equally likely.
pub(crate) fn route_distribution(
    current: &MapCellDefinition,
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
) -> BTreeMap<i64, f64> {
    let rules = stage.routing_rules.get(&current.cell_no).map(Vec::as_slice).unwrap_or_default();
    let executable = firing_rules(rules, context, stage).executable;
    let mut weights = rule_weights(&executable, context);
    if !weights.keys().any(|cell_no| current.next_cells.contains(cell_no)) {
        weights = current.next_cells.iter().map(|cell_no| (*cell_no, 1)).collect();
    }
    normalize_weights(weights)
}

/// Which start the fleet sorties from and how likely each is, without rolling.
pub(crate) fn start_distribution(
    stage: &MapStageDefinition,
    context: &FleetRouteContext,
) -> BTreeMap<i64, f64> {
    let mut weights = start_cell_weights(stage, context);
    if weights.is_empty() {
        weights = stage.start_source_cells().iter().map(|cell| (cell.cell_no, 1)).collect();
    }
    normalize_weights(weights)
}

fn normalize_weights(weights: BTreeMap<i64, u64>) -> BTreeMap<i64, f64> {
    let total = weights.values().sum::<u64>() as f64;
    weights.into_iter().map(|(cell_no, weight)| (cell_no, weight as f64 / total)).collect()
}

fn select_route_from_cells(
    current: &MapCellDefinition,
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
            coefficient: Some(coefficient),
            op,
            value,
            ..
        } => RoutePredicateEval::from_bool(compare_route_value(
            context.los_score(*coefficient),
            *op,
            *value,
        )),
        RoutePredicate::CountSum {
            terms,
            op,
            value,
        } => {
            let sum = terms.iter().map(|term| term.coef * context.count(&term.counter)).sum();
            RoutePredicateEval::from_bool(compare_route_value(sum, *op, *value))
        }
        RoutePredicate::DrumCanisterCount {
            op,
            value,
        } => RoutePredicateEval::from_bool(compare_route_value(context.drum_ships, *op, *value)),
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
        // An `LoS` rule without the branch-point coefficient has no score to compare:
        // its threshold means nothing against any other number.
        RoutePredicate::SourceUnknown {
            ..
        }
        | RoutePredicate::LoS {
            coefficient: None,
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
            coefficient,
            op,
            value,
        } => format!("los:{formula:?}:{coefficient:?}:{op:?}:{value}"),
        RoutePredicate::CountSum {
            terms,
            op,
            value,
        } => format!("cs:{terms:?}:{op:?}:{value}"),
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
    use emukc_model::codex::map::MapVariantDefinition;

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

    /// A context whose formula-33 score is `score` whatever the coefficient.
    fn make_los_context(score: f64) -> FleetRouteContext {
        FleetRouteContext {
            fleet_size: 6,
            los_ship_term: score,
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

    // --- LoS score tests ---

    fn los_predicate(coefficient: Option<i64>, op: RouteOperator, value: i64) -> RoutePredicate {
        RoutePredicate::LoS {
            formula: None,
            coefficient,
            op,
            value,
        }
    }

    #[test]
    fn los_score_scales_the_equipment_term_by_the_coefficient_and_floors() {
        let ctx = FleetRouteContext {
            los_ship_term: 10.4,
            los_equip_term: 5.3,
            ..Default::default()
        };

        assert_eq!(ctx.los_score(1), 15, "10.4 + 5.3 = 15.7");
        assert_eq!(ctx.los_score(4), 31, "10.4 + 21.2 = 31.6");
        assert_eq!(
            ctx.los_score(4) - ctx.los_score(1),
            16,
            "the difference is 3 x the equipment term"
        );
    }

    #[test]
    fn los_threshold_is_compared_with_the_score_not_a_raw_sum() {
        // A fleet scoring 40 used to pass 「49 以上」 because the raw LoS sum was compared.
        let ctx = make_los_context(40.0);
        let stage = make_los_stage();

        assert!(matches!(
            route_predicate_matches(&los_predicate(Some(1), RouteOperator::Gte, 49), &ctx, &stage),
            RoutePredicateEval::NotMatched
        ));
        assert!(matches!(
            route_predicate_matches(&los_predicate(Some(1), RouteOperator::Gte, 40), &ctx, &stage),
            RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn fractional_los_score_does_not_fall_between_whole_thresholds() {
        // 「28 未満」 is stored as `Lte 27`, 「28 以上」 as `Gte 28`.
        let ctx = make_los_context(27.5);
        let stage = make_los_stage();

        assert!(matches!(
            route_predicate_matches(&los_predicate(Some(3), RouteOperator::Lte, 27), &ctx, &stage),
            RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(&los_predicate(Some(3), RouteOperator::Gte, 28), &ctx, &stage),
            RoutePredicateEval::NotMatched
        ));
    }

    #[test]
    fn los_without_a_coefficient_is_source_unknown() {
        let ctx = make_los_context(999.0);
        let stage = make_los_stage();

        assert!(matches!(
            route_predicate_matches(&los_predicate(None, RouteOperator::Gte, 1), &ctx, &stage),
            RoutePredicateEval::SourceUnknown
        ));
    }

    #[test]
    fn count_sum_weighs_each_counter() {
        use emukc_model::codex::map::{RouteCountTerm, RouteCounter};

        let entry = |ship_id, ship_type, base_slow, slotitem_ids: &[i64]| FleetRouteShipEntry {
            ship_id,
            ship_type,
            base_slow,
            slotitem_ids: slotitem_ids.iter().copied().collect(),
            ..Default::default()
        };
        let ctx = FleetRouteContext {
            fleet_size: 4,
            // A fast and a slow battleship, an aviation battleship, a destroyer with a drum.
            ship_entries: vec![
                entry(78, 8, false, &[]),
                entry(26, 9, true, &[]),
                entry(82, 10, true, &[]),
                entry(1, 2, false, &[75]),
            ],
            ..Default::default()
        };
        let stage = make_los_stage();
        let sum = |terms: Vec<(i64, RouteCounter)>, op, value| {
            let predicate = RoutePredicate::CountSum {
                terms: terms
                    .into_iter()
                    .map(|(coef, counter)| RouteCountTerm {
                        coef,
                        counter,
                    })
                    .collect(),
                op,
                value,
            };
            matches!(route_predicate_matches(&predicate, &ctx, &stage), RoutePredicateEval::Matched)
        };
        let battleships = || RouteCounter::ShipTypes(vec![8, 9, 10]);
        let slow_bb = || RouteCounter::SlowShips {
            ship_types: vec![8, 9],
        };

        // 戦艦級 − 低速戦艦 = 3 − 1: the slow aviation battleship is not a 低速戦艦.
        assert!(sum(vec![(1, battleships()), (-1, slow_bb())], RouteOperator::Eq, 2));
        // 戦艦級 + 駆逐 = 艦数.
        assert!(sum(
            vec![(1, RouteCounter::ShipTypes(vec![2, 8, 9, 10])), (-1, RouteCounter::FleetSize)],
            RouteOperator::Eq,
            0
        ));
        assert!(sum(vec![(1, RouteCounter::Ships(vec![26, 78, 999]))], RouteOperator::Gte, 2));
        assert!(sum(
            vec![(
                1,
                RouteCounter::EquipCarriers {
                    slotitem_ids: vec![75]
                }
            )],
            RouteOperator::Lte,
            1
        ));
        assert!(!sum(vec![(1, battleships())], RouteOperator::Gte, 4));
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
        coefficient: i64,
        op: RouteOperator,
        value: i64,
    ) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            priority,
            weight: Some(1),
            predicate: RoutePredicate::LoS {
                formula: None,
                coefficient: Some(coefficient),
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
                make_los_rule(1, 2, 1, 1, RouteOperator::Gte, 60),
                make_los_rule(1, 3, 2, 1, RouteOperator::Gte, 30),
            ],
        );
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![2, 3]), make_cell(2, vec![]), make_cell(3, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![2, 3]);

        // LoS 50: fails threshold 60 (cell 2), passes threshold 30 (cell 3)
        let context = make_los_context(50.0);
        let result = evaluate_route_destination(&current, &stage, &context, None).unwrap();
        assert_eq!(result, 3, "los=50 should route to cell 3 (threshold 30)");

        // LoS 70: passes threshold 60 (cell 2)
        let context_high = make_los_context(70.0);
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
        routing_rules.insert(1, vec![make_los_rule(1, 2, 0, 1, RouteOperator::Gte, 100)]);
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![])],
            routing_rules,
            ..Default::default()
        };
        let current = make_cell(1, vec![]);
        // LoS is 10, threshold is 100 → rule doesn't match, next_cells is empty
        let context = make_los_context(10.0);

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
            vec![make_los_rule(1, 5, 0, 1, RouteOperator::Gte, 40), make_always_rule(1, 4, 1, 1)],
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
        let ctx_high_los = make_los_context(50.0);
        let result = evaluate_route_destination(&current_1, &stage, &ctx_high_los, None).unwrap();
        assert_eq!(result, 5, "high LoS at cell 1 should route to cell 5");

        // --- At cell 1, low LoS (20 < 40) → Always fallback rule at priority 1 → cell 4 ---
        let ctx_low_los = make_los_context(20.0);
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

    // ========================================================================
    // U5: map route structural + behavioral validation over the live codex
    // ========================================================================

    /// A small fleet-config matrix to drive route decisions across (KTD5 R5). Each entry
    /// varies the inputs the predicates read (fleet size, speed, `LoS`, drums) so different
    /// rule branches fire; routing legitimately uses weighted random, so we assert
    /// *edge-legality* (every decision lands on a declared `next_cell`), never a specific
    /// destination.
    fn fleet_config_matrix() -> Vec<FleetRouteContext> {
        vec![
            FleetRouteContext {
                fleet_size: 6,
                min_speed: 20,
                drum_ships: 4,
                ..Default::default()
            },
            FleetRouteContext {
                fleet_size: 4,
                min_speed: 10,
                drum_ships: 0,
                ..Default::default()
            },
            FleetRouteContext {
                fleet_size: 1,
                min_speed: 5,
                ..Default::default()
            },
        ]
    }

    /// Drive the real `evaluate_route_destination` over a bounded set of live codex map
    /// stages against a fleet-config matrix, asserting two things (U5, KTD5):
    ///
    /// 1. **Structural** (via the independent `emukc_bootstrap` validator): each stage has
    ///    zero structural errors — every routing-rule target is in the departing cell's
    ///    `next_cells`, every `next_cells`/rule cell is real. This is the structural-corruption
    ///    gate, mirroring how the battle validators catch protocol drift.
    /// 2. **Behavioral edge-legality**: every cell the real router returns is a declared
    ///    `next_cell` of the departing cell. Driven many times per (cell × fleet config) so
    ///    weighted-random branches are exercised, but membership must hold for every outcome —
    ///    the random sweep is deterministic in pass/fail. We assert no *illegal* edge, not that
    ///    every cell is routable headlessly (some indeterminate rule sets legitimately Err).
    ///
    /// This is the in-crate home for the behavioral check because `evaluate_route_destination`
    /// is `pub(crate)`; an external `tests/` integration test cannot reach it.
    #[test]
    fn live_codex_routing_is_structurally_and_behaviorally_consistent() {
        use emukc_bootstrap::prelude::validate_map_route_stage;
        use emukc_model::codex::Codex;

        let codex = Codex::load_without_cache_source("../../.data/codex")
            .expect("codex must be bootstrapped (.data/codex); run `cargo run -- bootstrap` first");

        // Bounded set of early-area maps that are always present and exercise real routing.
        let map_ids = [11, 12, 13, 14, 21, 22, 23];
        let configs = fleet_config_matrix();

        let mut stages_checked = 0usize;
        let mut routed_decisions = 0usize;

        for map_id in map_ids {
            let Some(definition) = codex.maps.map_definition(map_id) else {
                continue;
            };
            let Some(stage) = definition.stage("") else {
                continue;
            };

            // 1. Structural: zero errors from the independent validator. Unsupported-predicate
            // warnings are allowed (the wiki source is known-incomplete), but a topology error
            // (edge off the graph, missing cell) must not exist in real codex data.
            let report = validate_map_route_stage(stage);
            assert!(
                !report.has_errors(),
                "map {map_id} stage has structural route errors: {:?}",
                report
                    .findings
                    .iter()
                    .filter(|f| f.severity
                        == emukc_bootstrap::prelude::MapRouteValidationSeverity::Error)
                    .collect::<Vec<_>>()
            );
            stages_checked += 1;

            // 2. Behavioral edge-legality across the fleet-config matrix.
            for context in &configs {
                for cell in stage.cells.iter().filter(|cell| !cell.next_cells.is_empty()) {
                    let next_set: BTreeSet<i64> = cell.next_cells.iter().copied().collect();
                    for _ in 0..16 {
                        if let Ok(target) = evaluate_route_destination(cell, stage, context, None) {
                            assert!(
                                next_set.contains(&target),
                                "map {map_id} cell {} routed to {target}, outside next_cells {next_set:?}",
                                cell.cell_no
                            );
                            routed_decisions += 1;
                        }
                    }
                }
            }
        }

        assert!(stages_checked > 0, "expected at least one live codex stage to validate");
        assert!(routed_decisions > 0, "expected at least one routable decision over the matrix");
    }

    fn empty_stage() -> MapStageDefinition {
        MapStageDefinition::default()
    }

    #[test]
    fn route_predicate_matches_ship_set_variants() {
        fn route_entry(
            ship_id: i64,
            ship_type: i64,
            speed: i64,
            slotitem_types: &[i64],
        ) -> FleetRouteShipEntry {
            FleetRouteShipEntry {
                ship_id,
                ship_type,
                speed,
                slotitem_types: slotitem_types.iter().copied().collect(),
                ..Default::default()
            }
        }

        let context = FleetRouteContext {
            fleet_size: 3,
            visited_cell_ids: BTreeSet::new(),
            ship_ids: BTreeSet::from([526, 6001, 6002]),
            flagship_ship_id: Some(526),
            flagship_ship_type: Some(7),
            ship_type_counts: BTreeMap::from([(2, 2), (7, 1)]),
            ship_entries: vec![
                route_entry(526, 7, 10, &[]),
                route_entry(6001, 2, 10, &[]),
                route_entry(6002, 2, 10, &[]),
            ],
            min_speed: 10,
            drum_ships: 0,
            ..Default::default()
        };

        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::ContainsShipSet {
                    ship_types: vec![1],
                    ship_ids: vec![526],
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::OnlyShipSet {
                    ship_types: vec![2],
                    ship_ids: vec![526],
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::ShipSetCount {
                    ship_types: vec![2],
                    ship_ids: vec![526],
                    op: RouteOperator::Eq,
                    value: 3,
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::FlagshipShipId {
                    ship_ids: vec![526],
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn route_predicate_matches_visited_equipment_and_speed_qualified_predicates() {
        let context = FleetRouteContext {
            fleet_size: 4,
            visited_cell_ids: BTreeSet::from([1, 4]),
            ship_ids: BTreeSet::from([9001, 9002, 9003, 9004]),
            flagship_ship_id: Some(9001),
            flagship_ship_type: Some(3),
            ship_type_counts: BTreeMap::from([(3, 1), (8, 2), (11, 1)]),
            ship_entries: vec![
                FleetRouteShipEntry {
                    ship_id: 9001,
                    ship_type: 3,
                    speed: 10,
                    slotitem_types: BTreeSet::from([12]),
                    ..Default::default()
                },
                FleetRouteShipEntry {
                    ship_id: 9002,
                    ship_type: 8,
                    speed: 5,
                    slotitem_types: BTreeSet::new(),
                    ..Default::default()
                },
                FleetRouteShipEntry {
                    ship_id: 9003,
                    ship_type: 8,
                    speed: 5,
                    slotitem_types: BTreeSet::new(),
                    ..Default::default()
                },
                FleetRouteShipEntry {
                    ship_id: 9004,
                    ship_type: 11,
                    speed: 10,
                    slotitem_types: BTreeSet::new(),
                    ..Default::default()
                },
            ],
            min_speed: 5,
            drum_ships: 0,
            ..Default::default()
        };

        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNode {
                    cell_nos: vec![4],
                    visited: true,
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNode {
                    cell_nos: vec![7],
                    visited: false,
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Eq,
                    value: 1,
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::FlagshipShipType {
                    ship_types: vec![3],
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::ShipSetSpeedCount {
                    ship_types: vec![8],
                    ship_ids: vec![],
                    speed_op: RouteOperator::Lte,
                    speed_class: SpeedClass::Slow,
                    op: RouteOperator::Gte,
                    value: 2,
                },
                &context,
                &empty_stage(),
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn route_rules_prefer_executable_predicates_over_static_next_cells() {
        let current = MapCellDefinition {
            cell_no: 1,
            color_no: 4,
            event_id: 4,
            event_kind: 1,
            next_cells: vec![2, 3],
            node_label: None,
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            start_rules: Vec::new(),
            boss_cell_no: 3,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::from([(
                1,
                vec![
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 2,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::ContainsShipType {
                            ship_types: vec![13],
                        },
                        raw_text: "潜水艦を含む".to_string(),
                    },
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 3,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: "それ以外".to_string(),
                    },
                ],
            )]),
            enemy_fleets: BTreeMap::new(),
            air_raid_fleets: Vec::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            advance_on_reach: Vec::new(),
            transport_gauge: false,
            advance_needs_s_rank_at: Vec::new(),
            parse_warnings: Vec::new(),
        };
        let context = FleetRouteContext {
            fleet_size: 4,
            visited_cell_ids: BTreeSet::new(),
            ship_ids: BTreeSet::new(),
            flagship_ship_id: None,
            flagship_ship_type: None,
            ship_type_counts: BTreeMap::from([(2, 4)]),
            ship_entries: vec![
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
            ],
            min_speed: 10,
            drum_ships: 0,
            ..Default::default()
        };

        let next = evaluate_route_destination(&current, &variant, &context, None).unwrap();
        assert_eq!(next, 3);
    }

    #[test]
    fn fallback_rule_does_not_compete_with_matching_specific_rule() {
        let current = MapCellDefinition {
            cell_no: 1,
            color_no: 4,
            event_id: 4,
            event_kind: 1,
            next_cells: vec![2, 3],
            node_label: None,
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            start_rules: Vec::new(),
            boss_cell_no: 3,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::from([(
                1,
                vec![
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 2,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::ContainsShipType {
                            ship_types: vec![13],
                        },
                        raw_text: "潜水艦を含む".to_string(),
                    },
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 3,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: "それ以外".to_string(),
                    },
                ],
            )]),
            enemy_fleets: BTreeMap::new(),
            air_raid_fleets: Vec::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            advance_on_reach: Vec::new(),
            transport_gauge: false,
            advance_needs_s_rank_at: Vec::new(),
            parse_warnings: Vec::new(),
        };
        let context = FleetRouteContext {
            fleet_size: 4,
            visited_cell_ids: BTreeSet::new(),
            ship_ids: BTreeSet::new(),
            flagship_ship_id: None,
            flagship_ship_type: None,
            ship_type_counts: BTreeMap::from([(13, 1)]),
            ship_entries: vec![
                FleetRouteShipEntry {
                    ship_id: 1601,
                    ship_type: 13,
                    speed: 10,
                    slotitem_types: BTreeSet::new(),
                    ..Default::default()
                },
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
            ],
            min_speed: 10,
            drum_ships: 0,
            ..Default::default()
        };

        let next = evaluate_route_destination(&current, &variant, &context, None).unwrap();
        assert_eq!(next, 2);
    }

    #[test]
    fn cell_zero_uses_explicit_start_rules_before_static_next_cells() {
        let current = MapCellDefinition {
            cell_no: 0,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells: vec![1, 2],
            node_label: Some("Start".to_string()),
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            start_rules: Vec::new(),
            boss_cell_no: 2,
            cells: vec![
                current.clone(),
                MapCellDefinition {
                    cell_no: 1,
                    color_no: 4,
                    event_id: 4,
                    event_kind: 1,
                    next_cells: vec![],
                    node_label: Some("A".to_string()),
                    master_cell_id: None,
                    distance: None,
                },
                MapCellDefinition {
                    cell_no: 2,
                    color_no: 5,
                    event_id: 5,
                    event_kind: 1,
                    next_cells: vec![],
                    node_label: Some("C".to_string()),
                    master_cell_id: None,
                    distance: None,
                },
            ],
            routing_rules: BTreeMap::from([(
                0,
                vec![RouteRule {
                    from_cell_no: 0,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: "出撃".to_string(),
                }],
            )]),
            enemy_fleets: BTreeMap::new(),
            air_raid_fleets: Vec::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            advance_on_reach: Vec::new(),
            transport_gauge: false,
            advance_needs_s_rank_at: Vec::new(),
            parse_warnings: Vec::new(),
        };

        let next =
            evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
                .unwrap();
        assert_eq!(next, 2);
    }

    #[test]
    fn cell_zero_without_rules_picks_a_random_successor() {
        let current = MapCellDefinition {
            cell_no: 0,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells: vec![1, 2],
            node_label: Some("Start".to_string()),
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            start_rules: Vec::new(),
            boss_cell_no: 2,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::new(),
            enemy_fleets: BTreeMap::new(),
            air_raid_fleets: Vec::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            advance_on_reach: Vec::new(),
            transport_gauge: false,
            advance_needs_s_rank_at: Vec::new(),
            parse_warnings: Vec::new(),
        };

        for _ in 0..16 {
            let next =
                evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
                    .unwrap();
            assert!([1, 2].contains(&next), "start must lead to a declared successor, got {next}");
        }
    }

    /// Map 1-3 routing must follow the directed-graph edges declared in the codex
    /// (plan 2026-06-22-003 U3.3, R3/R4). Every decision the router makes from a
    /// cell has to land on one of that cell's declared `next_cells` — the fleet
    /// never skips a cell or jumps to a non-adjacent one. This drives the real
    /// `evaluate_route_destination` over the live 1-3 topology (the authoritative
    /// edge list), unlike the synthetic-data unit tests above.
    ///
    /// R4's "fallback never deterministically picks `next_cells[0]`" behavior is
    /// covered against synthetic data by `unknown_rules_fallback_to_random_next_cells`
    /// in `map_route.rs`; here we assert the real-map structural + per-decision
    /// edge-legality invariant.
    #[test]
    fn map_1_3_routing_follows_valid_edges_only() {
        use emukc_model::codex::Codex;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let definition = codex.maps.map_definition(13).expect("map 1-3 (id 13) in codex");
        let stage = definition.stage("").expect("1-3 default stage");

        let valid_cells: std::collections::BTreeSet<i64> =
            stage.cells.iter().map(|cell| cell.cell_no).collect();

        // Structural: the authoritative edge list is self-consistent — every
        // next_cell and every routing-rule target points at a real 1-3 cell, so no
        // edge can route off the topology.
        for cell in &stage.cells {
            for &next in &cell.next_cells {
                assert!(
                    valid_cells.contains(&next),
                    "1-3 cell {} lists next_cell {next} that is not a real cell",
                    cell.cell_no
                );
            }
        }
        for (&from, rules) in &stage.routing_rules {
            let next_cells: std::collections::BTreeSet<i64> = stage
                .cell(from)
                .map(|cell| cell.next_cells.iter().copied().collect())
                .unwrap_or_default();
            for rule in rules {
                assert!(
                    next_cells.contains(&rule.to_cell_no),
                    "1-3 cell {from} routing rule targets {} outside next_cells {next_cells:?}",
                    rule.to_cell_no
                );
            }
        }

        // 1-3 must actually branch, else "follows valid edges" would be vacuous and
        // the multi-edge fallback (U3.2) would never be exercised.
        assert!(
            stage.cells.iter().any(|cell| cell.next_cells.len() > 1),
            "map 1-3 is expected to have at least one branch node"
        );

        // Behavioral: drive the real router from every cell with out-edges, many
        // times so random branches are exercised. Each decision must land on a
        // declared next_cell of the departing cell — proving routing follows the
        // directed graph and never skips/jumps. Asserting membership holds for every
        // outcome, so the random sweep is deterministic in pass/fail.
        let context = FleetRouteContext {
            fleet_size: 6,
            ..Default::default()
        };
        let mut routed = 0usize;
        for cell in stage.cells.iter().filter(|cell| !cell.next_cells.is_empty()) {
            let next_set: std::collections::BTreeSet<i64> =
                cell.next_cells.iter().copied().collect();
            for _ in 0..40 {
                // Headless evaluation (no client-selected cell): some indeterminate
                // rule sets legitimately return Err without a selection. The
                // invariant guarded here is "never an illegal edge", not "always
                // routable headlessly".
                if let Ok(target) = evaluate_route_destination(cell, stage, &context, None) {
                    assert!(
                        next_set.contains(&target),
                        "1-3 cell {} routed to {target}, outside next_cells {next_set:?}",
                        cell.cell_no
                    );
                    routed += 1;
                }
            }
        }
        assert!(routed > 0, "expected at least one routable decision on 1-3");
    }
}
