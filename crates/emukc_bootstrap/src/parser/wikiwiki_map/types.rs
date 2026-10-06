use std::collections::BTreeMap;

use emukc_model::codex::map::{EnemyComposition, RoutePredicate, ShipDropDefinition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Intermediate route rule in label space (before conversion to cell numbers).
pub struct RouteRuleDraft {
    /// Source node label.
    pub from_label: String,
    /// Target node label.
    pub to_label: String,
    /// Probability percentage (0–100), if applicable.
    pub probability_pct: Option<f64>,
    /// Routing condition.
    pub predicate: RoutePredicate,
    /// Original Japanese condition text.
    pub raw_text: String,
    /// Whether this rule uses a random placeholder (unresolved probability).
    pub random_placeholder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Enemy encounter data for a node, keyed by node label.
pub struct EnemyNodeRows {
    /// Whether this node is the boss.
    pub is_boss: bool,
    /// Enemy compositions at this node.
    pub compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Ship drop entry in label space (before conversion to cell numbers).
pub struct ShipDropDraft {
    /// Node label where the drop occurs.
    pub node_label: String,
    /// Drop definition.
    pub drop: ShipDropDefinition,
}

// ── Label-keyed overlay types ──────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Label-keyed overlay data keyed by in-game map ID.
pub struct WikiwikiMapOverlayCatalog {
    /// Parsed map overlay definitions.
    pub maps: BTreeMap<i64, WikiwikiMapOverlayDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Overlay data for a single map, keyed by variant.
pub struct WikiwikiMapOverlayDefinition {
    /// In-game map ID.
    pub map_id: i64,
    /// Variant-keyed label overlays.
    pub variants: BTreeMap<String, WikiwikiLabelOverlay>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Parsed overlay data for a single map variant, using label-based keys.
pub struct WikiwikiLabelOverlay {
    /// Variant identifier.
    pub variant_key: String,
    /// Routing rules extracted from the route table.
    pub routing_rules: Vec<RouteRuleDraft>,
    /// Enemy compositions keyed by node label.
    pub enemy_nodes: BTreeMap<String, EnemyNodeRows>,
    /// Ship drop entries extracted from drop tables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ship_drops: Vec<ShipDropDraft>,
    /// Required boss defeat count, if specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_defeat_count: Option<i64>,
    /// Non-fatal warnings collected during parsing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parse_warnings: Vec<String>,
}
