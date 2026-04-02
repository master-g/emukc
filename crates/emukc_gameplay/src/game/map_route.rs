use std::collections::{BTreeMap, BTreeSet};

use emukc_model::codex::map::{
	MapCellDefinition, MapStageDefinition, RouteOperator, RoutePredicate, RouteRule, SpeedClass,
};
use rand::{RngExt, rng};

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
		return select_route_from_cells(current, selected_cell_id);
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
			return targets.iter().next().copied().ok_or_else(|| {
				GameplayError::WrongType(format!(
					"cell {} has no executable route",
					current.cell_no
				))
			});
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

	let mut random = rng();
	let roll = random.random_range(0..total_weight);
	select_route_target_for_roll(&weights, roll).ok_or_else(|| {
		GameplayError::WrongType(format!("cell {} has no executable route", current.cell_no))
	})
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
		Ok(current.next_cells[0])
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
	weights.keys().next().copied()
}

fn route_predicate_key(predicate: &RoutePredicate) -> String {
	serde_json::to_string(predicate).unwrap_or_else(|_| format!("{predicate:?}"))
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
