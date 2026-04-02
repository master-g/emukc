use std::collections::BTreeMap;

use emukc_model::codex::map::{EnemyComposition, RoutePredicate, RouteRule, ShipDropDefinition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Normalized wikiwiki.jp map extraction output keyed by in-game map ID.
pub struct WikiwikiMapCatalog {
	/// Parsed map definitions.
	pub maps: BTreeMap<i64, WikiwikiMapDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiMapDefinition {
	pub map_id: i64,
	pub maparea_id: i64,
	pub mapinfo_no: i64,
	pub name: String,
	pub source_url: String,
	pub default_variant: String,
	pub variants: BTreeMap<String, WikiwikiMapVariantDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiMapVariantDefinition {
	pub variant_key: String,
	pub nodes: Vec<WikiwikiNodeDefinition>,
	pub routing_rules: Vec<RouteRule>,
	pub enemy_fleets: Vec<WikiwikiEnemyFleetDefinition>,
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub ship_drops: BTreeMap<i64, Vec<ShipDropDefinition>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub required_defeat_count: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub clear_to_variant_key: Option<String>,
	pub parse_warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiNodeDefinition {
	pub label: String,
	pub cell_no: i64,
	pub is_boss: bool,
	pub is_battle: bool,
	pub next_cells: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiEnemyFleetDefinition {
	pub node_label: String,
	pub cell_no: i64,
	pub battle_kind: i64,
	pub formations: Vec<i64>,
	pub compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone)]
pub(super) struct EnemyNodeRows {
	pub(super) is_boss: bool,
	pub(super) compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone)]
pub(super) struct RouteRuleDraft {
	pub(super) from_label: String,
	pub(super) to_label: String,
	pub(super) probability_pct: Option<f64>,
	pub(super) predicate: RoutePredicate,
	pub(super) raw_text: String,
	pub(super) random_placeholder: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ShipDropDraft {
	pub(super) node_label: String,
	pub(super) drop: ShipDropDefinition,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ShipTypeResolver {
	pub(super) aliases: BTreeMap<String, i64>,
	pub(super) groups: BTreeMap<String, Vec<i64>>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ShipResolver {
	pub(super) labels: BTreeMap<String, i64>,
	pub(super) class_groups: BTreeMap<String, Vec<i64>>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct RouteSelector {
	pub(super) ship_types: Vec<i64>,
	pub(super) ship_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct RouteTableSection {
	pub(super) summary: String,
	pub(super) rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct CompiledRouteClause {
	pub(super) target_label: String,
	pub(super) probability_pct: Option<f64>,
	pub(super) predicate: RoutePredicate,
	pub(super) random_placeholder: bool,
}

#[derive(Debug, Clone)]
pub(super) struct RouteConditionLine {
	pub(super) indent: usize,
	pub(super) text: String,
}

#[derive(Debug, Clone)]
pub(super) enum DropCellEvent {
	Text {
		text: String,
		tags: Vec<String>,
	},
	Break,
}

#[derive(Debug, Clone)]
pub(super) enum RouteClauseAst {
	Rule {
		target_label: String,
		probability_pct: Option<f64>,
		predicate: RoutePredicate,
	},
	Case {
		guard: RoutePredicate,
		clauses: Vec<RouteClauseAst>,
	},
	Else {
		target_label: String,
	},
}
