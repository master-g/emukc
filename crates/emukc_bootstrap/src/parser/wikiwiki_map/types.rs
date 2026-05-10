use std::{collections::BTreeMap, sync::LazyLock};

use emukc_model::codex::map::{EnemyComposition, RoutePredicate, RouteRule, ShipDropDefinition};
use regex::Regex;
use serde::{Deserialize, Serialize};

pub(super) const ENTRY_NODE_LABEL: &str = "Start";

pub(super) fn is_entry_node_label(label: &str) -> bool {
    label == ENTRY_NODE_LABEL
}

static RE_COUNT_OP_BEFORE_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<name>.+?)[\s（(]*(?P<op>過不足なく|ちょうど)[\s）)]*(?P<count>\d+(?:隻)?)$")
        .expect("valid count op before count regex")
});

/// Normalize count-clause text by rewriting "op count" patterns into "count op" order.
///
/// When `suffix` is `Some`, the regex is extended to expect `を含{suffix}` at the end,
/// and the rewritten text appends `を含{suffix}`.
pub(super) fn normalize_count_clause_text(text: &str, suffix: Option<&str>) -> String {
    let text = super::sanitize_route_text(text)
        .replace("(過不足なく)", "過不足なく")
        .replace("（過不足なく）", "過不足なく")
        .replace("(ちょうど)", "ちょうど")
        .replace("（ちょうど）", "ちょうど");

    let re = match suffix {
        Some(_) => &*RE_CONTAINS_COUNT_OP_BEFORE_COUNT,
        None => &*RE_COUNT_OP_BEFORE_COUNT,
    };

    if let Some(caps) = re.captures(&text) {
        let name = super::normalize_text(
            caps.name("name").map(|value| value.as_str()).unwrap_or_default(),
        );
        let op = caps.name("op").map(|value| value.as_str()).unwrap_or_default();
        let count = caps.name("count").map(|value| value.as_str()).unwrap_or_default();
        if !name.is_empty() && !count.is_empty() {
            if let Some(s) = suffix {
                if let Some(suffix_match) = caps.name("suffix").map(|value| value.as_str()) {
                    return format!("{name}{count}{op}を含{suffix_match}");
                }
                return format!("{name}{count}{op}を含{s}");
            }
            return format!("{name}{count}{op}");
        }
    }
    text
}

static RE_CONTAINS_COUNT_OP_BEFORE_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<name>.+?)[\s（(]*(?P<op>過不足なく|ちょうど)[\s）)]*(?P<count>\d+(?:隻)?)を含(?P<suffix>む|み|まない)$",
    )
    .expect("valid contains count-op-before-count regex")
});

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overlays: BTreeMap<String, WikiwikiLabelOverlay>,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiwikiEnemyFleetDefinition {
    pub node_label: String,
    pub cell_no: i64,
    pub battle_kind: i64,
    pub formations: Vec<i64>,
    pub compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyNodeRows {
    pub is_boss: bool,
    pub compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRuleDraft {
    pub from_label: String,
    pub to_label: String,
    pub probability_pct: Option<f64>,
    pub predicate: RoutePredicate,
    pub raw_text: String,
    pub random_placeholder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipDropDraft {
    pub node_label: String,
    pub drop: ShipDropDefinition,
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
