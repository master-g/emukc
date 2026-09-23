use std::collections::BTreeMap;

use emukc_model::codex::map::{EnemyComposition, RoutePredicate};

mod types;
#[allow(unused_imports)]
use types::*;
#[allow(unused_imports)]
pub use types::{
    EnemyNodeRows, RouteRuleDraft, ShipDropDraft, WikiwikiEnemyFleetDefinition,
    WikiwikiLabelOverlay, WikiwikiMapCatalog, WikiwikiMapDefinition, WikiwikiMapOverlayCatalog,
    WikiwikiMapOverlayDefinition, WikiwikiMapVariantDefinition, WikiwikiNodeDefinition,
};

/// Entry node label used as the implicit cell 0 in all map topologies.
const ENTRY_NODE_LABEL: &str = "Start";

impl WikiwikiMapCatalog {
    /// Deserialize agent-produced JSON as a [`WikiwikiMapCatalog`].
    ///
    /// This is the seam between the `emukc-scrape-wikiwiki-mapdata` agent skill
    /// and the Rust type system. Returns the deserialized catalog on success,
    /// or a [`serde_json::Error`] on malformed input.
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    /// Validate that the catalog has at least one map with at least one variant.
    ///
    /// Returns `Ok(())` if the catalog is structurally sound, or an error
    /// message describing the issue.
    pub fn validate(&self) -> Result<(), String> {
        if self.maps.is_empty() {
            return Err("catalog has no maps".to_string());
        }
        for (map_id, def) in &self.maps {
            if def.variants.is_empty() {
                return Err(format!("map {map_id} has no variants"));
            }
            for (vk, variant) in &def.variants {
                if variant.nodes.is_empty() && !variant.enemy_fleets.is_empty() {
                    return Err(format!(
                        "map {map_id} variant '{vk}' has enemy fleets but no nodes"
                    ));
                }
            }
        }
        Ok(())
    }

    /// Convert the extractor output into the label-space asset the map build consumes.
    ///
    /// The agent numbers cells by its own BFS. Those numbers mean nothing outside
    /// one agent run, so every reference to a cell — rule endpoints, enemy and
    /// drop keys, and the cells inside a `VisitedNode` predicate — is lifted to
    /// the node label here, once. Assembly resolves labels to kcdata cell numbers.
    pub fn into_label_overlay_catalog(self) -> WikiwikiMapOverlayCatalog {
        let mut catalog = WikiwikiMapOverlayCatalog::default();
        for (map_id, definition) in self.maps {
            let variants = definition
                .variants
                .into_iter()
                .map(|(variant_key, variant)| (variant_key, variant.into_label_overlay()))
                .collect::<BTreeMap<_, _>>();
            if !variants.is_empty() {
                catalog.maps.insert(
                    map_id,
                    WikiwikiMapOverlayDefinition {
                        map_id,
                        variants,
                    },
                );
            }
        }
        catalog
    }

    /// Serialize to a [`serde_json::Value`] for debugging.
    pub fn to_debug_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }
}

// ── Conversion helpers ─────────────────────────────────────────────────

/// Remove raw ship names from an enemy composition (they are for human
/// verification only and not needed at runtime).
fn compact_enemy_composition(mut composition: EnemyComposition) -> EnemyComposition {
    composition.raw_ship_names.clear();
    composition
}

impl WikiwikiMapVariantDefinition {
    fn into_label_overlay(self) -> WikiwikiLabelOverlay {
        // The entry node is implicit in the agent output: it is always cell 0.
        let mut cell_to_label = BTreeMap::from([(0, ENTRY_NODE_LABEL.to_string())]);
        let mut label_to_cell = BTreeMap::new();
        for node in &self.nodes {
            if !node.label.is_empty() {
                cell_to_label.insert(node.cell_no, node.label.clone());
            }
            label_to_cell.insert(node.label.clone(), node.cell_no);
        }
        label_to_cell.insert(ENTRY_NODE_LABEL.to_string(), 0);
        let label_of = |cell_no: i64| cell_to_label.get(&cell_no).cloned();

        let mut rules = self.routing_rules;
        rules.sort_by_key(|rule| (rule.from_cell_no, rule.priority));
        let routing_rules = rules
            .into_iter()
            .filter_map(|rule| {
                let (Some(from_label), Some(to_label)) =
                    (label_of(rule.from_cell_no), label_of(rule.to_cell_no))
                else {
                    tracing::warn!(
                        variant = %self.variant_key,
                        from_cell_no = rule.from_cell_no,
                        to_cell_no = rule.to_cell_no,
                        "route rule names a cell with no node, dropped"
                    );
                    return None;
                };
                Some(RouteRuleDraft {
                    from_label,
                    to_label,
                    probability_pct: rule.probability_pct,
                    predicate: lift_predicate_to_labels(
                        rule.predicate,
                        &cell_to_label,
                        &label_to_cell,
                    ),
                    raw_text: rule.raw_text,
                    random_placeholder: false,
                })
            })
            .collect();

        let fleets = self
            .enemy_fleets
            .into_iter()
            .map(|fleet| (fleet.cell_no, fleet))
            .collect::<BTreeMap<_, _>>();
        let mut enemy_nodes = BTreeMap::<String, EnemyNodeRows>::new();
        for (cell_no, fleet) in fleets {
            let Some(label) = label_of(cell_no) else {
                continue;
            };
            let node = enemy_nodes.entry(label).or_insert_with(|| EnemyNodeRows {
                is_boss: false,
                compositions: Vec::new(),
            });
            node.is_boss |= fleet.battle_kind == 5;
            node.compositions.extend(fleet.compositions.into_iter().map(compact_enemy_composition));
        }

        let mut ship_drops = Vec::new();
        for (cell_no, drops) in self.ship_drops {
            let Some(label) = label_of(cell_no) else {
                continue;
            };
            ship_drops.extend(drops.into_iter().map(|drop| ShipDropDraft {
                node_label: label.clone(),
                drop,
            }));
        }

        WikiwikiLabelOverlay {
            variant_key: self.variant_key,
            routing_rules,
            enemy_nodes,
            ship_drops,
            required_defeat_count: self.required_defeat_count,
            parse_warnings: self.parse_warnings,
        }
    }
}

/// Lift the cells a route-history predicate names into labels.
///
/// The agent may name them either way; a cell number, or a label the agent
/// never placed on a node, has no meaning in the target and is dropped.
fn lift_predicate_to_labels(
    predicate: RoutePredicate,
    cell_to_label: &BTreeMap<i64, String>,
    label_to_cell: &BTreeMap<String, i64>,
) -> RoutePredicate {
    let lift = |p| lift_predicate_to_labels(p, cell_to_label, label_to_cell);
    let cells = match predicate {
        RoutePredicate::VisitedNode {
            cell_nos,
            visited,
        } => (cell_nos, visited),
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => (
            node_labels.iter().filter_map(|label| label_to_cell.get(label).copied()).collect(),
            visited,
        ),
        RoutePredicate::And(predicates) => {
            return RoutePredicate::And(predicates.into_iter().map(lift).collect());
        }
        RoutePredicate::Or(predicates) => {
            return RoutePredicate::Or(predicates.into_iter().map(lift).collect());
        }
        RoutePredicate::Not(predicate) => return RoutePredicate::Not(Box::new(lift(*predicate))),
        other => return other,
    };
    let (cell_nos, visited) = cells;
    let mut node_labels = Vec::new();
    for cell_no in cell_nos {
        match cell_to_label.get(&cell_no) {
            Some(label) if !node_labels.contains(label) => node_labels.push(label.clone()),
            Some(_) => {}
            None => tracing::warn!(cell_no, "VisitedNode predicate: cell has no label, dropped"),
        }
    }
    RoutePredicate::VisitedNodeLabel {
        node_labels,
        visited,
    }
}

/// Convert a probability percentage (0–100) to a weight (0–10000).
///
/// This helper is retained for testing and potential future use by agent
/// skill consumers that need to compute weights from percentages.
#[cfg(test)]
fn probability_to_weight(probability_pct: f64) -> i64 {
    (probability_pct * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::map::{RouteRule, ShipDropDefinition};

    use super::*;

    #[test]
    fn from_json_loads_example_successfully() {
        let raw = include_str!(
            "../../../../../.claude/skills/emukc-scrape-wikiwiki-mapdata/reference/map-example.json"
        );
        let catalog = WikiwikiMapCatalog::from_json(raw).expect("example JSON should deserialize");
        assert_eq!(catalog.maps.len(), 1, "example should have exactly 1 map");
        let def = &catalog.maps[&12];
        assert_eq!(def.maparea_id, 1);
        assert_eq!(def.mapinfo_no, 2);
        let variant = def.variants.get("").expect("default variant should exist");
        assert!(!variant.nodes.is_empty(), "variant should have nodes");
        assert!(variant.nodes.iter().any(|n| n.is_boss), "should have a boss node");
    }

    #[test]
    fn from_json_rejects_malformed() {
        let result = WikiwikiMapCatalog::from_json("{broken");
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_empty_catalog() {
        let catalog = WikiwikiMapCatalog::default();
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn validate_passes_on_example() {
        let raw = include_str!(
            "../../../../../.claude/skills/emukc-scrape-wikiwiki-mapdata/reference/map-example.json"
        );
        let catalog = WikiwikiMapCatalog::from_json(raw).expect("example JSON should deserialize");
        catalog.validate().expect("example should pass validation");
    }

    #[test]
    fn probability_to_weight_converts_correctly() {
        assert_eq!(probability_to_weight(60.0), 6000);
        assert_eq!(probability_to_weight(100.0), 10000);
        assert_eq!(probability_to_weight(0.0), 0);
    }

    fn node(label: &str, cell_no: i64) -> WikiwikiNodeDefinition {
        WikiwikiNodeDefinition {
            label: label.to_string(),
            cell_no,
            is_boss: false,
            is_battle: true,
        }
    }

    fn rule(from: i64, to: i64, predicate: RoutePredicate) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            predicate,
            ..Default::default()
        }
    }

    fn catalog_of(variant: WikiwikiMapVariantDefinition) -> WikiwikiMapCatalog {
        WikiwikiMapCatalog {
            maps: BTreeMap::from([(
                45,
                WikiwikiMapDefinition {
                    map_id: 45,
                    variants: BTreeMap::from([(String::new(), variant)]),
                    ..Default::default()
                },
            )]),
        }
    }

    fn only_variant(catalog: WikiwikiMapOverlayCatalog) -> WikiwikiLabelOverlay {
        catalog.maps[&45].variants[""].clone()
    }

    /// The agent's cell numbers are its own BFS and mean nothing to kcdata, so a
    /// route-history predicate must leave ingestion naming the node, not the number.
    #[test]
    fn ingestion_lifts_visited_node_to_its_label() {
        let variant = WikiwikiMapVariantDefinition {
            nodes: vec![node("A", 1), node("C", 7), node("M", 9)],
            routing_rules: vec![
                rule(
                    1,
                    9,
                    RoutePredicate::VisitedNode {
                        cell_nos: vec![7],
                        visited: true,
                    },
                ),
                // Cell 42 is no node: the rule cannot be placed and is dropped.
                rule(1, 42, RoutePredicate::Always),
            ],
            ..Default::default()
        };

        let overlay = only_variant(catalog_of(variant).into_label_overlay_catalog());

        assert_eq!(overlay.routing_rules.len(), 1);
        let draft = &overlay.routing_rules[0];
        assert_eq!((draft.from_label.as_str(), draft.to_label.as_str()), ("A", "M"));
        match &draft.predicate {
            RoutePredicate::VisitedNodeLabel {
                node_labels,
                visited,
            } => {
                assert_eq!(node_labels, &vec!["C".to_string()]);
                assert!(visited);
            }
            other => panic!("expected VisitedNodeLabel, got {other:?}"),
        }
    }

    /// A label the agent placed on two BFS cells is one node: its enemy rows and
    /// drops must all survive the collapse to the label.
    #[test]
    fn ingestion_merges_rows_of_a_label_split_across_cells() {
        let variant = WikiwikiMapVariantDefinition {
            nodes: vec![node("E", 3), node("E", 11)],
            enemy_fleets: [3, 11]
                .into_iter()
                .map(|cell_no| WikiwikiEnemyFleetDefinition {
                    node_label: "E".to_string(),
                    cell_no,
                    battle_kind: if cell_no == 11 {
                        5
                    } else {
                        1
                    },
                    formations: vec![1],
                    compositions: vec![EnemyComposition {
                        ship_ids: vec![1500 + cell_no],
                        ..Default::default()
                    }],
                })
                .collect(),
            ship_drops: BTreeMap::from([
                (
                    3,
                    vec![ShipDropDefinition {
                        ship_id: 1,
                        ..Default::default()
                    }],
                ),
                (
                    11,
                    vec![ShipDropDefinition {
                        ship_id: 2,
                        ..Default::default()
                    }],
                ),
            ]),
            ..Default::default()
        };

        let overlay = only_variant(catalog_of(variant).into_label_overlay_catalog());

        let rows = &overlay.enemy_nodes["E"];
        assert!(rows.is_boss);
        assert_eq!(
            rows.compositions.iter().map(|c| c.ship_ids.clone()).collect::<Vec<_>>(),
            vec![vec![1503], vec![1511]]
        );
        assert_eq!(
            overlay
                .ship_drops
                .iter()
                .map(|d| (d.node_label.as_str(), d.drop.ship_id))
                .collect::<Vec<_>>(),
            vec![("E", 1), ("E", 2)]
        );
    }
}
