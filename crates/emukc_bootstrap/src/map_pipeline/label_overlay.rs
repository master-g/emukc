use std::collections::BTreeMap;

use emukc_model::codex::map::{
    EnemyFleetDefinition, MapVariantDefinition, RoutePredicate, RouteRule,
};

use crate::parser::wikiwiki_map::WikiwikiLabelOverlay;

/// Merge wikiwiki label-keyed overlay onto a kcdata variant via the authoritative `label→cell_no` index.
///
/// Returns the count of items dropped due to unmatched labels.
pub fn merge_label_overlay(
    kcdata_variant: &mut MapVariantDefinition,
    overlay: &WikiwikiLabelOverlay,
    label_index: &BTreeMap<String, i64>,
) -> usize {
    let mut dropped = 0usize;
    let cell_set: std::collections::BTreeSet<i64> =
        kcdata_variant.cells.iter().map(|c| c.cell_no).collect();

    // Routing rules
    for draft in &overlay.routing_rules {
        let Some(&from_cell_no) = label_index.get(&draft.from_label) else {
            tracing::warn!(
                from_label = %draft.from_label,
                to_label = %draft.to_label,
                "overlay routing rule dropped: from_label not in kcdata index"
            );
            dropped += 1;
            continue;
        };
        let Some(&to_cell_no) = label_index.get(&draft.to_label) else {
            tracing::warn!(
                from_label = %draft.from_label,
                to_label = %draft.to_label,
                "overlay routing rule dropped: to_label not in kcdata index"
            );
            dropped += 1;
            continue;
        };
        if !cell_set.contains(&from_cell_no) {
            tracing::warn!(
                from_cell_no,
                from_label = %draft.from_label,
                "overlay routing rule dropped: from_cell_no not in kcdata variant cells"
            );
            dropped += 1;
            continue;
        }
        if !cell_set.contains(&to_cell_no) {
            tracing::warn!(
                to_cell_no,
                to_label = %draft.to_label,
                "overlay routing rule dropped: to_cell_no not in kcdata variant cells"
            );
            dropped += 1;
            continue;
        }
        let predicate = resolve_predicate_labels(&draft.predicate, label_index);
        let rule = RouteRule {
            from_cell_no,
            to_cell_no,
            priority: 0,
            weight: draft.probability_pct.map(probability_to_weight),
            probability_pct: draft.probability_pct,
            raw_text: compact_route_raw_text(&predicate, &draft.raw_text),
            predicate,
        };
        kcdata_variant.routing_rules.entry(from_cell_no).or_default().push(rule);
    }

    // Re-index priorities: assign sequential priorities per from_cell_no bucket,
    // preserving relative order of newly inserted overlay rules.
    for rules in kcdata_variant.routing_rules.values_mut() {
        for (i, rule) in rules.iter_mut().enumerate() {
            rule.priority = i as i64;
        }
    }

    // Enemy fleets
    for (label, node) in &overlay.enemy_nodes {
        let Some(&cell_no) = label_index.get(label) else {
            tracing::warn!(label = %label, "overlay enemy fleet dropped: label not in kcdata index");
            dropped += 1;
            continue;
        };
        if !cell_set.contains(&cell_no) {
            tracing::warn!(
                cell_no,
                label = %label,
                "overlay enemy fleet dropped: cell_no not in kcdata variant cells"
            );
            dropped += 1;
            continue;
        }
        kcdata_variant.enemy_fleets.entry(cell_no).or_insert_with(|| EnemyFleetDefinition {
            cell_no,
            battle_kind: 1,
            formations: collect_formations(&node.compositions),
            compositions: node.compositions.clone(),
        });
    }

    // Ship drops
    for draft in &overlay.ship_drops {
        let Some(&cell_no) = label_index.get(&draft.node_label) else {
            tracing::warn!(
                label = %draft.node_label,
                "overlay ship drop dropped: label not in kcdata index"
            );
            dropped += 1;
            continue;
        };
        kcdata_variant.ship_drops.entry(cell_no).or_default().push(draft.drop.clone());
    }

    dropped
}

/// Convert a label-based predicate to a `cell_no`-based predicate.
fn resolve_predicate_labels(
    predicate: &RoutePredicate,
    label_index: &BTreeMap<String, i64>,
) -> RoutePredicate {
    match predicate {
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => {
            let mut cell_nos = Vec::with_capacity(node_labels.len());
            for label in node_labels {
                if let Some(&cell_no) = label_index.get(label) {
                    cell_nos.push(cell_no);
                } else {
                    tracing::warn!(
                        label = %label,
                        "VisitedNodeLabel predicate: label not in kcdata index, partial conversion"
                    );
                }
            }
            RoutePredicate::VisitedNode {
                cell_nos,
                visited: *visited,
            }
        }
        RoutePredicate::And(children) => RoutePredicate::And(
            children.iter().map(|p| resolve_predicate_labels(p, label_index)).collect(),
        ),
        RoutePredicate::Or(children) => RoutePredicate::Or(
            children.iter().map(|p| resolve_predicate_labels(p, label_index)).collect(),
        ),
        RoutePredicate::Not(inner) => {
            RoutePredicate::Not(Box::new(resolve_predicate_labels(inner, label_index)))
        }
        other => other.clone(),
    }
}

fn probability_to_weight(pct: f64) -> i64 {
    (pct * 100.0).round() as i64
}

fn compact_route_raw_text(predicate: &RoutePredicate, raw: &str) -> String {
    if matches!(predicate, RoutePredicate::Always) && !raw.is_empty() {
        raw.to_string()
    } else {
        String::new()
    }
}

fn collect_formations(compositions: &[emukc_model::codex::map::EnemyComposition]) -> Vec<i64> {
    compositions.iter().filter_map(|c| c.formation).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::wikiwiki_map::{EnemyNodeRows, RouteRuleDraft, ShipDropDraft};
    use emukc_model::codex::map::{MapCellDefinition, ShipDropDefinition};

    fn make_cell(cell_no: i64, label: &str) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells: vec![],
            node_label: if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            },
            master_cell_id: None,
            distance: None,
        }
    }

    fn make_variant(cells: Vec<MapCellDefinition>) -> MapVariantDefinition {
        MapVariantDefinition {
            variant_key: String::new(),
            cells,
            ..Default::default()
        }
    }

    fn make_label_index(variant: &MapVariantDefinition) -> BTreeMap<String, i64> {
        variant.label_to_cell_no()
    }

    fn draft_rule(from: &str, to: &str, predicate: RoutePredicate) -> RouteRuleDraft {
        RouteRuleDraft {
            from_label: from.to_string(),
            to_label: to.to_string(),
            probability_pct: None,
            predicate,
            raw_text: String::new(),
            random_placeholder: false,
        }
    }

    fn draft_drop(label: &str, ship_id: i64) -> ShipDropDraft {
        ShipDropDraft {
            node_label: label.to_string(),
            drop: ShipDropDefinition {
                ship_id,
                ..Default::default()
            },
        }
    }

    #[test]
    fn happy_path_all_labels_match() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell(1, "A"),
            make_cell(2, "B"),
            make_cell(3, "C"),
        ]);
        let index = make_label_index(&variant);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![draft_rule("A", "B", RoutePredicate::Always)],
            enemy_nodes: vec![(
                "C".to_string(),
                EnemyNodeRows {
                    is_boss: false,
                    compositions: vec![],
                },
            )]
            .into_iter()
            .collect(),
            ship_drops: vec![draft_drop("B", 100)],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay, &index);
        assert_eq!(dropped, 0);
        assert!(target.routing_rules.get(&1).is_some_and(|r| r[0].to_cell_no == 2));
        assert!(target.enemy_fleets.contains_key(&3));
        assert!(target.ship_drops.contains_key(&2));
    }

    #[test]
    fn partial_match_some_labels_missing() {
        let variant =
            make_variant(vec![make_cell(0, "Start"), make_cell(1, "A"), make_cell(2, "B")]);
        let index = make_label_index(&variant);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![
                draft_rule("A", "B", RoutePredicate::Always),
                draft_rule("A", "Z", RoutePredicate::Always),
            ],
            enemy_nodes: BTreeMap::new(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay, &index);
        assert_eq!(dropped, 1);
        assert!(target.routing_rules.get(&1).is_some_and(|r| r.len() == 1));
    }

    #[test]
    fn no_match_all_dropped() {
        let variant = make_variant(vec![make_cell(0, "Start"), make_cell(1, "X")]);
        let index = make_label_index(&variant);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![draft_rule("A", "B", RoutePredicate::Always)],
            enemy_nodes: vec![(
                "C".to_string(),
                EnemyNodeRows {
                    is_boss: false,
                    compositions: vec![],
                },
            )]
            .into_iter()
            .collect(),
            ship_drops: vec![draft_drop("D", 100)],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay, &index);
        assert_eq!(dropped, 3);
        assert!(target.routing_rules.is_empty());
        assert!(target.enemy_fleets.is_empty());
        assert!(target.ship_drops.is_empty());
    }

    #[test]
    fn visited_node_label_converted_to_visited_node() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell(1, "A"),
            make_cell(2, "B"),
            make_cell(3, "C"),
        ]);
        let index = make_label_index(&variant);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![draft_rule(
                "A",
                "B",
                RoutePredicate::VisitedNodeLabel {
                    node_labels: vec!["C".to_string()],
                    visited: true,
                },
            )],
            enemy_nodes: BTreeMap::new(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay, &index);
        assert_eq!(dropped, 0);

        let rule = &target.routing_rules.get(&1).unwrap()[0];
        match &rule.predicate {
            RoutePredicate::VisitedNode {
                cell_nos,
                visited,
            } => {
                assert_eq!(*cell_nos, vec![3]);
                assert!(*visited);
            }
            other => panic!("expected VisitedNode, got {:?}", other),
        }
    }

    #[test]
    fn enemy_fleet_at_matched_label() {
        let variant = make_variant(vec![make_cell(0, "Start"), make_cell(5, "E")]);
        let index = make_label_index(&variant);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![],
            enemy_nodes: vec![(
                "E".to_string(),
                EnemyNodeRows {
                    is_boss: true,
                    compositions: vec![],
                },
            )]
            .into_iter()
            .collect(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay, &index);
        assert_eq!(dropped, 0);
        assert!(target.enemy_fleets.contains_key(&5));
        assert_eq!(target.enemy_fleets.get(&5).unwrap().cell_no, 5);
    }
}
