use std::collections::BTreeMap;

use emukc_model::codex::map::{EnemyComposition, RoutePredicate, RouteRule, ShipDropDefinition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Normalized wikiwiki.jp map extraction output keyed by in-game map ID.
///
/// This is the agent skill output format — produced by the
/// `emukc-scrape-wikiwiki-mapdata` skill and consumed by
/// [`WikiwikiMapCatalog::from_json`](super::WikiwikiMapCatalog::from_json).
pub struct WikiwikiMapCatalog {
    /// Parsed map definitions, keyed by `maparea_id * 10 + mapinfo_no`.
    pub maps: BTreeMap<i64, WikiwikiMapDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// A single map's extraction output.
pub struct WikiwikiMapDefinition {
    /// In-game map ID (`maparea_id * 10 + mapinfo_no`).
    pub map_id: i64,
    /// Map area ID (1–7 for normal maps).
    pub maparea_id: i64,
    /// Map number within the area.
    pub mapinfo_no: i64,
    /// Japanese map name from wikiwiki.
    pub name: String,
    /// Source wikiwiki URL.
    pub source_url: String,
    /// Default variant key (empty string for normal maps).
    pub default_variant: String,
    /// Variant definitions keyed by variant key.
    pub variants: BTreeMap<String, WikiwikiMapVariantDefinition>,
    /// Optional overlay data (usually empty for normal maps).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overlays: BTreeMap<String, WikiwikiLabelOverlay>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// A single map variant's extraction output.
pub struct WikiwikiMapVariantDefinition {
    /// Variant identifier (empty string for normal single-variant maps).
    pub variant_key: String,
    /// All map nodes with agent-assigned BFS cell numbers.
    pub nodes: Vec<WikiwikiNodeDefinition>,
    /// Route rules between cells (using cell numbers, not labels).
    pub routing_rules: Vec<RouteRule>,
    /// Enemy fleet definitions per battle node.
    pub enemy_fleets: Vec<WikiwikiEnemyFleetDefinition>,
    /// Ship drops keyed by cell number.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ship_drops: BTreeMap<i64, Vec<ShipDropDefinition>>,
    /// Required boss defeat count, if specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_defeat_count: Option<i64>,
    /// Next variant key after clearing this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_to_variant_key: Option<String>,
    /// Non-fatal warnings collected during extraction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parse_warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// A single node in the map topology.
pub struct WikiwikiNodeDefinition {
    /// Node label from wikiwiki (`Start`, `A`, `B`, ...).
    pub label: String,
    /// BFS-assigned cell number (Start = 0).
    pub cell_no: i64,
    /// Whether this is the boss node.
    pub is_boss: bool,
    /// Whether enemy encounters exist at this node.
    pub is_battle: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Enemy fleet data for a single battle node.
pub struct WikiwikiEnemyFleetDefinition {
    /// Node label (A, B, etc.).
    pub node_label: String,
    /// Cell number matching the node.
    pub cell_no: i64,
    /// Battle kind (0 = normal, 1 = normal battle, 5 = boss).
    pub battle_kind: i64,
    /// Possible formations (1=単縦, 2=複縦, 3=輪形, 4=梯形, 5=単横).
    pub formations: Vec<i64>,
    /// Enemy fleet compositions.
    pub compositions: Vec<EnemyComposition>,
}

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
    pub ship_drops: Vec<ShipDropDraft>,
    /// Required boss defeat count, if specified.
    pub required_defeat_count: Option<i64>,
    /// Non-fatal warnings collected during parsing.
    pub parse_warnings: Vec<String>,
}
