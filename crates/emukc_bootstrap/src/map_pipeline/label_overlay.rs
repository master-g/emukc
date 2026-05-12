use std::collections::BTreeMap;

use emukc_model::codex::map::{
    EnemyFleetDefinition, MapVariantDefinition, RoutePredicate, RouteRule,
};

use crate::parser::wikiwiki_map::WikiwikiLabelOverlay;

/// Merge wikiwiki label-keyed overlay onto a kcdata variant via route-cell labels.
///
/// Returns the count of items dropped due to unmatched labels.
pub fn merge_label_overlay(
    kcdata_variant: &mut MapVariantDefinition,
    overlay: &WikiwikiLabelOverlay,
) -> usize {
    let mut dropped = 0usize;
    let label_index = kcdata_variant.multi_label_index();
    let cell_by_no =
        kcdata_variant.cells.iter().map(|cell| (cell.cell_no, cell)).collect::<BTreeMap<_, _>>();
    let mut rules_to_add = Vec::new();

    // Routing rules
    for draft in &overlay.routing_rules {
        let Some(from_cell_nos) = label_index.get(&draft.from_label) else {
            tracing::warn!(
                from_label = %draft.from_label,
                to_label = %draft.to_label,
                "overlay routing rule dropped: from_label not in kcdata index"
            );
            dropped += 1;
            continue;
        };
        if !label_index.contains_key(&draft.to_label) {
            tracing::warn!(
                from_label = %draft.from_label,
                to_label = %draft.to_label,
                "overlay routing rule dropped: to_label not in kcdata index"
            );
            dropped += 1;
            continue;
        };
        let predicate = resolve_predicate_labels(&draft.predicate, &label_index);
        let mut resolved_any_edge = false;

        for &from_cell_no in from_cell_nos {
            let Some(from_cell) = cell_by_no.get(&from_cell_no) else {
                tracing::warn!(
                    from_cell_no,
                    from_label = %draft.from_label,
                    "overlay routing rule dropped: from_cell_no not in kcdata variant cells"
                );
                continue;
            };
            let mut target_cell_nos = Vec::new();
            for next_cell_no in &from_cell.next_cells {
                let Some(next_cell) = cell_by_no.get(next_cell_no) else {
                    continue;
                };
                if next_cell.node_label.as_deref() == Some(draft.to_label.as_str())
                    && !target_cell_nos.contains(next_cell_no)
                {
                    target_cell_nos.push(*next_cell_no);
                }
            }
            if target_cell_nos.is_empty() {
                continue;
            }
            resolved_any_edge = true;
            for to_cell_no in target_cell_nos {
                rules_to_add.push(RouteRule {
                    from_cell_no,
                    to_cell_no,
                    priority: 0,
                    weight: draft.probability_pct.map(probability_to_weight),
                    probability_pct: draft.probability_pct,
                    raw_text: compact_route_raw_text(&predicate, &draft.raw_text),
                    predicate: predicate.clone(),
                });
            }
        }

        if !resolved_any_edge {
            tracing::warn!(
                from_label = %draft.from_label,
                to_label = %draft.to_label,
                "overlay routing rule dropped: labels exist but no route edge connects them"
            );
            dropped += 1;
        }
    }

    for rule in rules_to_add {
        kcdata_variant.routing_rules.entry(rule.from_cell_no).or_default().push(rule);
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
        let Some(cell_nos) = label_index.get(label) else {
            tracing::warn!(label = %label, "overlay enemy fleet dropped: label not in kcdata index");
            dropped += 1;
            continue;
        };
        for &cell_no in cell_nos {
            kcdata_variant.enemy_fleets.entry(cell_no).or_insert_with(|| EnemyFleetDefinition {
                cell_no,
                battle_kind: 1,
                formations: collect_formations(&node.compositions),
                compositions: node.compositions.clone(),
            });
        }
    }

    // Ship drops
    for draft in &overlay.ship_drops {
        let Some(cell_nos) = label_index.get(&draft.node_label) else {
            tracing::warn!(
                label = %draft.node_label,
                "overlay ship drop dropped: label not in kcdata index"
            );
            dropped += 1;
            continue;
        };
        for &cell_no in cell_nos {
            kcdata_variant.ship_drops.entry(cell_no).or_default().push(draft.drop.clone());
        }
    }

    dropped
}

/// Convert a label-based predicate to a `cell_no`-based predicate.
fn resolve_predicate_labels(
    predicate: &RoutePredicate,
    label_index: &BTreeMap<String, Vec<i64>>,
) -> RoutePredicate {
    match predicate {
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => {
            let mut cell_nos = Vec::new();
            for label in node_labels {
                if let Some(label_cell_nos) = label_index.get(label) {
                    for &cell_no in label_cell_nos {
                        if !cell_nos.contains(&cell_no) {
                            cell_nos.push(cell_no);
                        }
                    }
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
        make_cell_with_next(cell_no, label, vec![])
    }

    fn make_cell_with_next(cell_no: i64, label: &str, next_cells: Vec<i64>) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells,
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
            make_cell_with_next(1, "A", vec![2]),
            make_cell(2, "B"),
            make_cell(3, "C"),
        ]);
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

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 0);
        assert!(target.routing_rules.get(&1).is_some_and(|r| r[0].to_cell_no == 2));
        assert!(target.enemy_fleets.contains_key(&3));
        assert!(target.ship_drops.contains_key(&2));
    }

    #[test]
    fn partial_match_some_labels_missing() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell_with_next(1, "A", vec![2]),
            make_cell(2, "B"),
        ]);
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

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 1);
        assert!(target.routing_rules.get(&1).is_some_and(|r| r.len() == 1));
    }

    #[test]
    fn no_match_all_dropped() {
        let variant = make_variant(vec![make_cell(0, "Start"), make_cell(1, "X")]);
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

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 3);
        assert!(target.routing_rules.is_empty());
        assert!(target.enemy_fleets.is_empty());
        assert!(target.ship_drops.is_empty());
    }

    #[test]
    fn visited_node_label_converted_to_visited_node() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell_with_next(1, "A", vec![2]),
            make_cell(2, "B"),
            make_cell(3, "C"),
        ]);
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

        let dropped = merge_label_overlay(&mut target, &overlay);
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

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 0);
        assert!(target.enemy_fleets.contains_key(&5));
        assert_eq!(target.enemy_fleets.get(&5).unwrap().cell_no, 5);
    }

    #[test]
    fn enemy_fleet_fans_out_to_duplicate_labels() {
        let variant =
            make_variant(vec![make_cell(0, "Start"), make_cell(5, "E"), make_cell(11, "E")]);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![],
            enemy_nodes: vec![(
                "E".to_string(),
                EnemyNodeRows {
                    is_boss: false,
                    compositions: vec![],
                },
            )]
            .into_iter()
            .collect(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 0);
        assert!(target.enemy_fleets.contains_key(&5));
        assert!(target.enemy_fleets.contains_key(&11));
    }

    #[test]
    fn ship_drops_fan_out_to_duplicate_labels() {
        let variant =
            make_variant(vec![make_cell(0, "Start"), make_cell(10, "J"), make_cell(13, "J")]);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![],
            enemy_nodes: BTreeMap::new(),
            ship_drops: vec![draft_drop("J", 100)],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 0);
        assert_eq!(target.ship_drops.get(&10).unwrap()[0].ship_id, 100);
        assert_eq!(target.ship_drops.get(&13).unwrap()[0].ship_id, 100);
    }

    #[test]
    fn duplicate_label_route_rules_resolve_through_route_topology() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell(7, "G"),
            make_cell_with_next(8, "H", vec![7, 9, 13]),
            make_cell(9, "I"),
            make_cell(10, "J"),
            make_cell_with_next(6, "F", vec![8, 10]),
            make_cell_with_next(12, "F", vec![8, 10]),
            make_cell(13, "J"),
        ]);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![
                draft_rule("F", "J", RoutePredicate::Always),
                draft_rule("H", "J", RoutePredicate::Always),
            ],
            enemy_nodes: BTreeMap::new(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 0);
        assert_eq!(target.routing_rules.get(&6).unwrap()[0].to_cell_no, 10);
        assert_eq!(target.routing_rules.get(&12).unwrap()[0].to_cell_no, 10);
        assert_eq!(target.routing_rules.get(&8).unwrap()[0].to_cell_no, 13);
    }

    #[test]
    fn visited_node_label_expands_to_all_duplicate_label_cells() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell_with_next(1, "A", vec![2]),
            make_cell(2, "B"),
            make_cell(5, "E"),
            make_cell(11, "E"),
        ]);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![draft_rule(
                "A",
                "B",
                RoutePredicate::VisitedNodeLabel {
                    node_labels: vec!["E".to_string()],
                    visited: true,
                },
            )],
            enemy_nodes: BTreeMap::new(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 0);
        match &target.routing_rules.get(&1).unwrap()[0].predicate {
            RoutePredicate::VisitedNode {
                cell_nos,
                visited,
            } => {
                assert_eq!(*cell_nos, vec![5, 11]);
                assert!(*visited);
            }
            other => panic!("expected VisitedNode, got {:?}", other),
        }
    }

    #[test]
    fn route_rule_with_existing_labels_but_no_edge_is_dropped() {
        let variant = make_variant(vec![
            make_cell(0, "Start"),
            make_cell_with_next(1, "A", vec![2]),
            make_cell(2, "B"),
            make_cell(3, "C"),
        ]);
        let mut target = variant.clone();

        let overlay = WikiwikiLabelOverlay {
            variant_key: String::new(),
            routing_rules: vec![draft_rule("A", "C", RoutePredicate::Always)],
            enemy_nodes: BTreeMap::new(),
            ship_drops: vec![],
            required_defeat_count: None,
            parse_warnings: vec![],
        };

        let dropped = merge_label_overlay(&mut target, &overlay);
        assert_eq!(dropped, 1);
        assert!(target.routing_rules.is_empty());
    }
}
