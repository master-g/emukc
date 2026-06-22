use std::collections::BTreeMap;

use emukc_model::{
    codex::map::{
        EnemyComposition, EnemyFleetDefinition, MapCatalog, MapCellDefinition, MapDefinition,
        MapResetPolicy, MapVariantDefinition, RoutePredicate, RouteRule,
    },
    kc2::start2::ApiManifest,
};

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

    /// Convert the extractor output into the runtime [`MapCatalog`] model.
    pub fn into_map_catalog(self, manifest: &ApiManifest) -> MapCatalog {
        self.into_map_catalog_with_overlay(manifest).0
    }

    /// Convert into runtime [`MapCatalog`] and extract label-keyed overlay catalog.
    pub fn into_map_catalog_with_overlay(
        self,
        manifest: &ApiManifest,
    ) -> (MapCatalog, WikiwikiMapOverlayCatalog) {
        let mut catalog = MapCatalog::default();
        let mut overlay_catalog = WikiwikiMapOverlayCatalog::default();
        for (map_id, definition) in self.maps {
            let manifest_map = manifest.api_mst_mapinfo.iter().find(|map| map.api_id == map_id);
            let area_type = manifest
                .api_mst_maparea
                .iter()
                .find(|area| area.api_id == definition.maparea_id)
                .map(|area| area.api_type)
                .unwrap_or_default();
            let default_variant = definition.default_variant.clone();
            let first_variant_required_defeat_count = definition
                .variants
                .get(&default_variant)
                .and_then(|variant| variant.required_defeat_count)
                .or_else(|| {
                    definition
                        .variants
                        .values()
                        .next()
                        .and_then(|variant| variant.required_defeat_count)
                });
            let gauge_count =
                (definition.variants.len() > 1).then_some(definition.variants.len() as i64);
            let variants = definition
                .variants
                .into_iter()
                .map(|(variant_key, variant)| {
                    let mut node_labels_to_cell = variant
                        .nodes
                        .iter()
                        .map(|node| (node.label.clone(), node.cell_no))
                        .collect::<BTreeMap<_, _>>();
                    node_labels_to_cell.insert(ENTRY_NODE_LABEL.to_string(), 0);
                    let mut routing_rules = BTreeMap::<i64, Vec<RouteRule>>::new();
                    for rule in &variant.routing_rules {
                        let mut rule = rule.clone();
                        rule.predicate =
                            rewrite_route_predicate_labels(rule.predicate, &node_labels_to_cell);
                        routing_rules.entry(rule.from_cell_no).or_default().push(rule);
                    }
                    for rules in routing_rules.values_mut() {
                        rules.sort_by_key(|rule| rule.priority);
                    }

                    let inferred_root_targets = variant
                        .nodes
                        .iter()
                        .filter(|node| {
                            !variant.routing_rules.iter().any(|rule| {
                                rule.to_cell_no == node.cell_no && rule.from_cell_no != node.cell_no
                            })
                        })
                        .map(|node| node.cell_no)
                        .collect::<Vec<_>>();
                    let mut parse_warnings = variant.parse_warnings;
                    let start_targets = routing_rules
                        .get(&0)
                        .map(|rules| ordered_route_targets(rules))
                        .filter(|targets| !targets.is_empty())
                        .unwrap_or_else(|| {
                            if inferred_root_targets.len() > 1 {
                                parse_warnings.push(format!(
                                    "inferred_multi_root_start:{}",
                                    inferred_root_targets
                                        .iter()
                                        .map(i64::to_string)
                                        .collect::<Vec<_>>()
                                        .join(",")
                                ));
                            }
                            if inferred_root_targets.is_empty() && !variant.nodes.is_empty() {
                                parse_warnings.push("missing_start_routes".to_string());
                            }
                            if inferred_root_targets.is_empty() {
                                variant
                                    .nodes
                                    .first()
                                    .map(|node| vec![node.cell_no])
                                    .unwrap_or_default()
                            } else {
                                inferred_root_targets
                            }
                        });

                    let mut cells = Vec::with_capacity(variant.nodes.len() + 1);
                    cells.push(MapCellDefinition {
                        cell_no: 0,
                        color_no: 0,
                        event_id: 0,
                        event_kind: 0,
                        next_cells: start_targets,
                        node_label: Some(ENTRY_NODE_LABEL.to_string()),
                        master_cell_id: None,
                        distance: None,
                    });

                    let boss_cell_no = variant
                        .nodes
                        .iter()
                        .find(|node| node.is_boss)
                        .map(|node| node.cell_no)
                        .or_else(|| {
                            variant
                                .nodes
                                .iter()
                                .filter(|node| node.is_battle)
                                .map(|node| node.cell_no)
                                .max()
                        })
                        .unwrap_or(1);

                    for node in &variant.nodes {
                        let (color_no, event_id, event_kind) = if node.is_boss {
                            (5, 5, 1)
                        } else if node.is_battle {
                            (4, 4, 1)
                        } else {
                            (6, 1, 0)
                        };
                        cells.push(MapCellDefinition {
                            cell_no: node.cell_no,
                            color_no,
                            event_id,
                            event_kind,
                            next_cells: vec![],
                            node_label: Some(node.label.clone()),
                            master_cell_id: None,
                            distance: None,
                        });
                    }

                    let enemy_fleets = variant
                        .enemy_fleets
                        .into_iter()
                        .map(|fleet| {
                            (
                                fleet.cell_no,
                                EnemyFleetDefinition {
                                    cell_no: fleet.cell_no,
                                    battle_kind: fleet.battle_kind,
                                    formations: fleet.formations,
                                    compositions: fleet
                                        .compositions
                                        .into_iter()
                                        .map(compact_enemy_composition)
                                        .collect(),
                                },
                            )
                        })
                        .collect::<BTreeMap<_, _>>();

                    (
                        variant_key.clone(),
                        MapVariantDefinition {
                            variant_key,
                            boss_cell_no,
                            cells,
                            routing_rules,
                            enemy_fleets,
                            ship_drops: variant.ship_drops,
                            required_defeat_count: variant.required_defeat_count,
                            clear_to_variant_key: variant.clear_to_variant_key,
                            parse_warnings,
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            catalog.maps.insert(
                map_id,
                MapDefinition {
                    map_id,
                    maparea_id: definition.maparea_id,
                    mapinfo_no: definition.mapinfo_no,
                    name: manifest_map.map(|map| map.api_name.clone()).unwrap_or(definition.name),
                    level: manifest_map.map(|map| map.api_level).unwrap_or(1),
                    sally_flag: manifest_map
                        .map(|map| map.api_sally_flag.clone())
                        .unwrap_or_default(),
                    is_event: area_type == 1,
                    reset_policy: MapResetPolicy::Never,
                    airbase_count: None,
                    gauge_type: None,
                    gauge_count,
                    required_defeat_count: first_variant_required_defeat_count
                        .or_else(|| manifest_map.and_then(|map| map.api_required_defeat_count)),
                    max_hp: None,
                    default_variant,
                    rank_stage_ids: BTreeMap::new(),
                    variants,
                },
            );

            if !definition.overlays.is_empty() {
                overlay_catalog.maps.insert(
                    map_id,
                    WikiwikiMapOverlayDefinition {
                        map_id,
                        variants: definition.overlays,
                    },
                );
            }
        }

        (catalog, overlay_catalog)
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

/// Extract route targets in priority order from a group of routing rules.
fn ordered_route_targets(rules: &[RouteRule]) -> Vec<i64> {
    let mut seen = std::collections::BTreeSet::new();
    let mut targets = Vec::new();
    for rule in rules {
        if seen.insert(rule.to_cell_no) {
            targets.push(rule.to_cell_no);
        }
    }
    targets
}

/// Rewrite label-based predicates (`VisitedNodeLabel`) into cell-number-based
/// predicates (`VisitedNode`) using the label→cell mapping.
fn rewrite_route_predicate_labels(
    predicate: RoutePredicate,
    node_labels_to_cell: &BTreeMap<String, i64>,
) -> RoutePredicate {
    match predicate {
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => RoutePredicate::VisitedNode {
            cell_nos: node_labels
                .into_iter()
                .filter_map(|label| node_labels_to_cell.get(&label).copied())
                .collect(),
            visited,
        },
        RoutePredicate::And(predicates) => RoutePredicate::And(
            predicates
                .into_iter()
                .map(|p| rewrite_route_predicate_labels(p, node_labels_to_cell))
                .collect(),
        ),
        RoutePredicate::Or(predicates) => RoutePredicate::Or(
            predicates
                .into_iter()
                .map(|p| rewrite_route_predicate_labels(p, node_labels_to_cell))
                .collect(),
        ),
        RoutePredicate::Not(predicate) => RoutePredicate::Not(Box::new(
            rewrite_route_predicate_labels(*predicate, node_labels_to_cell),
        )),
        other => other,
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

    #[test]
    fn ordered_route_targets_deduplicates() {
        let rules = vec![
            RouteRule {
                from_cell_no: 0,
                to_cell_no: 1,
                priority: 0,
                weight: None,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text: String::new(),
            },
            RouteRule {
                from_cell_no: 0,
                to_cell_no: 1,
                priority: 1,
                weight: None,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text: String::new(),
            },
            RouteRule {
                from_cell_no: 0,
                to_cell_no: 2,
                priority: 2,
                weight: None,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text: String::new(),
            },
        ];
        let targets = ordered_route_targets(&rules);
        assert_eq!(targets, vec![1, 2]);
    }
}
