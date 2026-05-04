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
    pub(crate) los_total: i64,
    pub(crate) total_drums: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoutePredicateEval {
    Matched,
    NotMatched,
    SourceUnknown,
    Unsupported,
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
        match route_predicate_matches(&rule.predicate, context) {
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
            {
                return Ok(selected_cell_id);
            }
            return select_route_from_cells(current, stage, None);
        }

        if let Some(selected_cell_id) = selected_cell_id {
            let candidate_targets = rules
                .iter()
                .map(|rule| rule.to_cell_no)
                .chain(current.next_cells.iter().copied())
                .collect::<BTreeSet<_>>();
            if any_indeterminate && candidate_targets.contains(&selected_cell_id) {
                return Ok(selected_cell_id);
            }
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

    let candidate_targets = executable.iter().map(|rule| rule.to_cell_no).collect::<BTreeSet<_>>();
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
            ..
        }
        | RoutePredicate::Unknown {
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
            op,
            value,
            ..
        } => RoutePredicateEval::from_bool(compare_route_value(context.los_total, *op, *value)),
        RoutePredicate::DrumCanisterCount {
            op,
            value,
        } => RoutePredicateEval::from_bool(compare_route_value(context.total_drums, *op, *value)),
        RoutePredicate::And(predicates) => {
            for predicate in predicates {
                match route_predicate_matches(predicate, context) {
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
                match route_predicate_matches(predicate, context) {
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
        RoutePredicate::Not(predicate) => match route_predicate_matches(predicate, context) {
            RoutePredicateEval::Matched => RoutePredicateEval::NotMatched,
            RoutePredicateEval::NotMatched => RoutePredicateEval::Matched,
            RoutePredicateEval::SourceUnknown => RoutePredicateEval::SourceUnknown,
            RoutePredicateEval::Unsupported => RoutePredicateEval::Unsupported,
        },
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
    fn source_unknown_with_selected_cell_in_targets_not_in_next_cells() {
        let mut routing_rules = BTreeMap::new();
        // SourceUnknown rules target cells 10 and 20
        routing_rules.insert(
            1,
            vec![make_source_unknown_rule(1, 10, 0), make_source_unknown_rule(1, 20, 1)],
        );

        // But current cell's next_cells only has 7 and 8
        let stage = MapStageDefinition {
            cells: vec![make_cell(1, vec![7, 8]), make_cell(7, vec![]), make_cell(8, vec![])],
            routing_rules,
            ..Default::default()
        };

        let current = make_cell(1, vec![7, 8]);
        let context = FleetRouteContext::default();

        // selected_cell_id 10 is in rule targets but NOT in next_cells.
        // The source-unknown path returns it because it matches a rule target.
        let result = evaluate_route_destination(&current, &stage, &context, Some(10));
        assert_eq!(
            result.unwrap(),
            10,
            "should return selected_cell_id when it matches a rule target"
        );
    }
}
