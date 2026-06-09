use emukc_model::codex::map::{
    EnemyFleetDefinition, MapCatalog, MapVariantDefinition, build_cell_no_map,
    merge_routing_overlay,
};

use super::{
    label_overlay::merge_label_overlay,
    report::{MapCatalogBuildReport, MapCatalogStatSource},
    sources::ResolvedMapSources,
};

pub(super) fn assemble_final_map_catalog(
    sources: ResolvedMapSources,
) -> (MapCatalog, MapCatalogBuildReport) {
    let mut overlay_items_dropped = 0usize;
    let mut catalog = match (sources.kcdata_catalog, sources.wikiwiki_overlay) {
        // New path: kcdata topology + label-keyed wikiwiki overlay.
        (Some(mut kcdata), Some(overlay)) => {
            overlay_items_dropped = merge_label_overlay_catalog(&mut kcdata, &overlay);
            kcdata
        }
        // Legacy path: kcdata + pre-built wikiwiki MapCatalog (no overlay available).
        (Some(mut kcdata), None) => {
            if let Some(wikiwiki) = sources.wikiwiki_catalog {
                overlay_items_dropped =
                    merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
            }
            kcdata
        }
        // Fallback path: no kcdata, wikiwiki produces full MapCatalog.
        (None, _) => sources.wikiwiki_catalog.unwrap_or_default(),
    };
    catalog.merge_missing_from(sources.public_overlay_catalog);
    if let Some(ref stat_catalog) = sources.stat_catalog {
        catalog.merge_missing_from(stat_catalog.clone());
    }

    // P-unlock maps (e.g. 7-3) arrive with topology-less pre_p_unlock / post_p_unlock
    // skeletons from the public overlay and a stale empty-key "" default from kcdata.
    // Reconcile them once all three sources are merged: fold the kcdata "" topology into the
    // p_unlock variants, make pre_p_unlock the default, derive gauge_count, and drop "".
    for definition in catalog.maps.values_mut() {
        definition.normalize_p_unlock_variants();
    }

    let output_map_count = catalog.maps.len();

    // Topology validation — warn during bootstrap, not at runtime codex load.
    let mut topology_warnings = 0usize;
    for def in catalog.maps.values() {
        let warnings = def.validate();
        topology_warnings += warnings.len();
        for w in &warnings {
            tracing::warn!("{w:?}");
        }
    }
    if topology_warnings > 0 {
        tracing::warn!(
            topology_warnings,
            map_count = catalog.maps.len(),
            "map catalog validation: topology warnings"
        );
    }

    let stat_source = if sources.stat_catalog.is_some() {
        if sources.stat_from_cache {
            MapCatalogStatSource::Cached
        } else {
            MapCatalogStatSource::Downloaded
        }
    } else {
        MapCatalogStatSource::Unavailable
    };

    (
        catalog,
        MapCatalogBuildReport {
            wikiwiki_source: sources.wikiwiki_source,
            wikiwiki_map_count: sources.wikiwiki_map_count,
            public_overlay_map_count: sources.public_overlay_map_count,
            stat_map_count: sources.stat_map_count,
            stat_source,
            output_map_count,
            fanout_rules_dropped: overlay_items_dropped,
            kcdata_parse_errors: sources.kcdata_parse_errors,
            topology_warnings,
        },
    )
}

/// Merge label-keyed wikiwiki overlay onto kcdata topology using the authoritative `label→cell_no` index.
fn merge_label_overlay_catalog(
    kcdata: &mut MapCatalog,
    overlay_catalog: &crate::parser::wikiwiki_map::WikiwikiMapOverlayCatalog,
) -> usize {
    let mut total_dropped = 0usize;

    for (map_id, overlay_def) in &overlay_catalog.maps {
        let Some(kcdata_map) = kcdata.maps.get_mut(map_id) else {
            continue;
        };
        let definition_has_named_variants = kcdata_map.variants.keys().any(|key| !key.is_empty());

        for (variant_key, overlay) in &overlay_def.variants {
            if variant_key.is_empty() && definition_has_named_variants {
                // Fan out to all named variants.
                let keys: Vec<String> = kcdata_map.variants.keys().cloned().collect();
                for key in &keys {
                    let Some(kcdata_variant) = kcdata_map.variants.get_mut(key.as_str()) else {
                        continue;
                    };
                    total_dropped += merge_label_overlay(kcdata_variant, overlay);
                }
            } else if let Some(kcdata_variant) = kcdata_map.variants.get_mut(variant_key) {
                total_dropped += merge_label_overlay(kcdata_variant, overlay);
            }
        }
    }

    total_dropped
}

/// Overlay `WikiWiki` routing rules, enemy fleets, and ship drops onto kcdata topology.
/// Does NOT touch cells or `next_cells` — kcdata is the sole source of graph topology.
///
/// Returns the total number of routing rules dropped because their `from_cell_no` or
/// `to_cell_no` was absent from the target variant's cell set.
fn merge_routing_overlay_from_wikiwiki_legacy(
    kcdata: &mut MapCatalog,
    wikiwiki: &MapCatalog,
) -> usize {
    let mut total_dropped = 0usize;

    for (map_id, wikiwiki_map) in &wikiwiki.maps {
        let Some(kcdata_map) = kcdata.maps.get_mut(map_id) else {
            continue;
        };
        let definition_has_named_variants = kcdata_map.variants.keys().any(|key| !key.is_empty());

        for (variant_key, wikiwiki_variant) in &wikiwiki_map.variants {
            let other_labels: std::collections::BTreeMap<String, i64> = wikiwiki_variant
                .cells
                .iter()
                .filter_map(|cell| {
                    cell.node_label
                        .as_ref()
                        .filter(|label| !label.is_empty())
                        .map(|label| (label.clone(), cell.cell_no))
                })
                .collect();

            // When the wikiwiki variant key is "" and the kcdata map has named variants,
            // fan out to every named variant, but only after validating each rule against
            // that variant's own cell set.
            if variant_key.is_empty() && definition_has_named_variants {
                // Collect the variant keys first to satisfy the borrow checker.
                let keys: Vec<String> = kcdata_map.variants.keys().cloned().collect();
                for key in &keys {
                    let Some(kcdata_variant) = kcdata_map.variants.get_mut(key.as_str()) else {
                        continue;
                    };
                    let dropped = apply_overlay_checked(
                        *map_id,
                        key,
                        kcdata_variant,
                        &other_labels,
                        wikiwiki_variant,
                    );
                    total_dropped += dropped;
                }
            } else if let Some(kcdata_variant) = kcdata_map.variants.get_mut(variant_key) {
                let cell_no_map = build_cell_no_map(kcdata_variant, &other_labels);
                merge_routing_overlay(
                    kcdata_variant,
                    &cell_no_map,
                    &wikiwiki_variant.routing_rules,
                    &std::collections::BTreeMap::new(),
                    &std::collections::BTreeMap::new(),
                );
                merge_legacy_enemy_fleets_and_ship_drops(
                    kcdata_variant,
                    &cell_no_map,
                    wikiwiki_variant,
                );
            }
        }
    }

    total_dropped
}

/// Apply a wikiwiki overlay onto a single kcdata variant, validating that each routing
/// rule's `from_cell_no` and `to_cell_no` both exist in the target variant's cell set.
/// Rules that reference absent cells are dropped and counted.
///
/// Routing rule validation happens after remapping wikiwiki cell numbers to kcdata cell
/// numbers via `cell_no_map`. Both the remapped `from_cell_no` and remapped `to_cell_no`
/// must exist in the target variant's cell set for the rule to be accepted.
///
/// Enemy fleet and ship drop overlays are applied separately so duplicate target labels
/// fan out to all matching kcdata cells.
fn apply_overlay_checked(
    map_id: i64,
    variant_key: &str,
    kcdata_variant: &mut MapVariantDefinition,
    other_labels: &std::collections::BTreeMap<String, i64>,
    wikiwiki_variant: &MapVariantDefinition,
) -> usize {
    use emukc_model::codex::map::RouteRule;
    use std::collections::{BTreeMap, BTreeSet};

    let cell_no_map = build_cell_no_map(kcdata_variant, other_labels);

    // Build the set of kcdata cell numbers for membership tests.
    let cell_set: BTreeSet<i64> = kcdata_variant.cells.iter().map(|c| c.cell_no).collect();

    // `remap` translates a wikiwiki cell number to its kcdata equivalent, falling back to
    // identity when no label match exists (mirrors `remap_cell_no` in merge.rs).
    let remap = |n: i64| -> i64 { cell_no_map.get(&n).copied().unwrap_or(n) };

    let mut validated_rules: BTreeMap<i64, Vec<RouteRule>> = BTreeMap::new();
    let mut dropped = 0usize;

    for (raw_from, rules) in &wikiwiki_variant.routing_rules {
        let mapped_from = remap(*raw_from);
        if !cell_set.contains(&mapped_from) {
            let count = rules.len();
            tracing::warn!(
                map_id,
                variant_key,
                raw_from_cell_no = raw_from,
                mapped_from_cell_no = mapped_from,
                count,
                "fan-out routing rules dropped: from_cell_no absent in target variant topology"
            );
            dropped += count;
            continue;
        }
        for rule in rules {
            let mapped_to = remap(rule.to_cell_no);
            if !cell_set.contains(&mapped_to) {
                tracing::warn!(
                    map_id,
                    variant_key,
                    raw_to_cell_no = rule.to_cell_no,
                    mapped_to_cell_no = mapped_to,
                    "fan-out routing rule dropped: to_cell_no absent in target variant topology"
                );
                dropped += 1;
            } else {
                validated_rules.entry(mapped_from).or_default().push(RouteRule {
                    from_cell_no: mapped_from,
                    to_cell_no: mapped_to,
                    ..rule.clone()
                });
            }
        }
    }

    if dropped > 0 {
        tracing::warn!(
            map_id,
            variant_key,
            dropped,
            "fan-out dropped routing rules due to topology mismatch"
        );
    }

    // Insert the pre-validated, pre-remapped routing rules directly (bypassing the
    // `cell_no_map.is_empty()` early exit in `merge_routing_overlay`).
    for (from_cell_no, rules) in validated_rules {
        kcdata_variant.routing_rules.entry(from_cell_no).or_default().extend(rules);
    }

    merge_legacy_enemy_fleets_and_ship_drops(kcdata_variant, &cell_no_map, wikiwiki_variant);

    dropped
}

fn merge_legacy_enemy_fleets_and_ship_drops(
    kcdata_variant: &mut MapVariantDefinition,
    cell_no_map: &std::collections::BTreeMap<i64, i64>,
    wikiwiki_variant: &MapVariantDefinition,
) {
    let kcdata_label_index = kcdata_variant.multi_label_index();
    let wikiwiki_label_by_cell_no = wikiwiki_variant
        .cells
        .iter()
        .filter_map(|cell| {
            cell.node_label
                .as_ref()
                .filter(|label| !label.is_empty())
                .map(|label| (cell.cell_no, label.clone()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    for (wikiwiki_cell_no, fleet) in &wikiwiki_variant.enemy_fleets {
        let targets = legacy_overlay_targets(
            kcdata_variant,
            &kcdata_label_index,
            &wikiwiki_label_by_cell_no,
            cell_no_map,
            *wikiwiki_cell_no,
        );
        for target_cell_no in targets {
            kcdata_variant.enemy_fleets.entry(target_cell_no).or_insert_with(|| {
                EnemyFleetDefinition {
                    cell_no: target_cell_no,
                    ..fleet.clone()
                }
            });
        }
    }

    for (wikiwiki_cell_no, drops) in &wikiwiki_variant.ship_drops {
        let targets = legacy_overlay_targets(
            kcdata_variant,
            &kcdata_label_index,
            &wikiwiki_label_by_cell_no,
            cell_no_map,
            *wikiwiki_cell_no,
        );
        for target_cell_no in targets {
            kcdata_variant.ship_drops.entry(target_cell_no).or_insert_with(|| drops.clone());
        }
    }
}

fn legacy_overlay_targets(
    kcdata_variant: &MapVariantDefinition,
    kcdata_label_index: &std::collections::BTreeMap<String, Vec<i64>>,
    wikiwiki_label_by_cell_no: &std::collections::BTreeMap<i64, String>,
    cell_no_map: &std::collections::BTreeMap<i64, i64>,
    wikiwiki_cell_no: i64,
) -> Vec<i64> {
    let mut targets = std::collections::BTreeSet::new();

    if let Some(label) = wikiwiki_label_by_cell_no.get(&wikiwiki_cell_no)
        && let Some(cell_nos) = kcdata_label_index.get(label)
    {
        for &cell_no in cell_nos {
            if kcdata_variant.cell(cell_no).is_some() {
                targets.insert(cell_no);
            }
        }
    }

    if targets.is_empty() {
        let mapped_cell_no =
            cell_no_map.get(&wikiwiki_cell_no).copied().unwrap_or(wikiwiki_cell_no);
        if kcdata_variant.cell(mapped_cell_no).is_some() {
            targets.insert(mapped_cell_no);
        }
    }

    targets.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use emukc_model::codex::map::{
        EnemyFleetDefinition, MapCatalog, MapCellDefinition, MapDefinition, MapVariantDefinition,
        RouteRule, ShipDropDefinition,
    };

    use super::merge_routing_overlay_from_wikiwiki_legacy;

    /// Build a [`MapCellDefinition`] with an auto-generated node label `C{cell_no}`.
    /// The label ensures [`build_cell_no_map`] produces a non-empty map, which in turn
    /// prevents [`merge_routing_overlay`] from bailing out early.
    fn make_cell(cell_no: i64) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells: vec![],
            // Label cells so that cell_no_map is non-empty when both sides share labels.
            node_label: Some(format!("C{cell_no}")),
            master_cell_id: None,
            distance: None,
        }
    }

    /// Build a `MapVariantDefinition` with the given cell numbers and routing rules.
    /// Cells are auto-labeled `"C{n}"` so that `build_cell_no_map` produces identity
    /// mappings and `merge_routing_overlay` does not short-circuit on an empty map.
    fn make_variant(key: &str, cell_nos: &[i64], rules: Vec<RouteRule>) -> MapVariantDefinition {
        let routing_rules: BTreeMap<i64, Vec<RouteRule>> = {
            let mut m: BTreeMap<i64, Vec<RouteRule>> = BTreeMap::new();
            for rule in rules {
                m.entry(rule.from_cell_no).or_default().push(rule);
            }
            m
        };
        MapVariantDefinition {
            variant_key: key.to_owned(),
            cells: cell_nos.iter().map(|&n| make_cell(n)).collect(),
            routing_rules,
            ..Default::default()
        }
    }

    /// Build a `MapCatalog` with one map identified by `map_id`, containing the given
    /// variants.
    fn make_catalog(map_id: i64, variants: Vec<MapVariantDefinition>) -> MapCatalog {
        let mut variant_map: BTreeMap<String, MapVariantDefinition> = BTreeMap::new();
        for v in variants {
            variant_map.insert(v.variant_key.clone(), v);
        }
        let map_def = MapDefinition {
            variants: variant_map,
            ..Default::default()
        };
        let mut maps = BTreeMap::new();
        maps.insert(map_id, map_def);
        MapCatalog {
            maps,
            ..Default::default()
        }
    }

    fn rule(from: i64, to: i64) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            ..Default::default()
        }
    }

    fn fleet(cell_no: i64) -> EnemyFleetDefinition {
        EnemyFleetDefinition {
            cell_no,
            battle_kind: 1,
            formations: vec![1],
            compositions: Vec::new(),
        }
    }

    // ------------------------------------------------------------------ happy path

    /// Both cells present in target → rule merged, counter = 0.
    #[test]
    fn test_fanout_happy_path_no_drops() {
        let mut kcdata = make_catalog(
            11,
            vec![
                make_variant("gauge_1", &[1, 3, 5], vec![]),
                make_variant("gauge_2", &[1, 3, 5], vec![]),
            ],
        );
        // Wikiwiki variant must include cells so build_cell_no_map produces a non-empty map.
        let wikiwiki = make_catalog(11, vec![make_variant("", &[3, 5], vec![rule(3, 5)])]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 0);

        // Rule should appear in both variants.
        for key in ["gauge_1", "gauge_2"] {
            let v = &kcdata.maps[&11].variants[key];
            assert!(
                v.routing_rules
                    .get(&3)
                    .is_some_and(|rules| rules.iter().any(|r| r.to_cell_no == 5)),
                "expected rule 3→5 in {key}"
            );
        }
    }

    #[test]
    fn test_legacy_enemy_fleet_fans_out_to_duplicate_target_labels() {
        let mut kcdata = make_catalog(14, vec![make_variant("", &[1, 2, 3, 4], vec![])]);
        {
            let variant = kcdata.maps.get_mut(&14).unwrap().variants.get_mut("").unwrap();
            variant.cells[0].node_label = Some("Start".to_string());
            variant.cells[1].node_label = Some("A".to_string());
            variant.cells[2].node_label = Some("L".to_string());
            variant.cells[3].node_label = Some("L".to_string());
        }

        let mut wikiwiki_variant = make_variant("", &[1, 6], vec![]);
        wikiwiki_variant.cells[0].node_label = Some("Start".to_string());
        wikiwiki_variant.cells[1].node_label = Some("L".to_string());
        wikiwiki_variant.enemy_fleets = BTreeMap::from([(6, fleet(6))]);
        wikiwiki_variant.ship_drops = BTreeMap::from([(
            6,
            vec![ShipDropDefinition {
                ship_id: 1,
                raw_ship_name: "Mutsuki".to_string(),
                tags: Vec::new(),
            }],
        )]);
        let wikiwiki = make_catalog(14, vec![wikiwiki_variant]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 0);

        let variant = &kcdata.maps[&14].variants[""];
        assert!(variant.enemy_fleets.contains_key(&3));
        assert!(variant.enemy_fleets.contains_key(&4));
        assert_eq!(variant.enemy_fleets.get(&3).unwrap().cell_no, 3);
        assert_eq!(variant.enemy_fleets.get(&4).unwrap().cell_no, 4);
        assert!(variant.ship_drops.contains_key(&3));
        assert!(variant.ship_drops.contains_key(&4));
    }

    #[test]
    fn test_legacy_enemy_fleet_and_ship_drops_fan_out_to_named_variants() {
        let kcdata = make_catalog(
            14,
            vec![
                make_variant("gauge_1", &[1, 6], vec![]),
                make_variant("gauge_2", &[1, 6], vec![]),
            ],
        );

        let mut wikiwiki_variant = make_variant("", &[1, 6], vec![]);
        wikiwiki_variant.enemy_fleets = BTreeMap::from([(6, fleet(6))]);
        wikiwiki_variant.ship_drops = BTreeMap::from([(
            6,
            vec![ShipDropDefinition {
                ship_id: 1,
                raw_ship_name: "Mutsuki".to_string(),
                tags: Vec::new(),
            }],
        )]);
        let wikiwiki = make_catalog(14, vec![wikiwiki_variant]);

        let mut kcdata = kcdata;
        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 0);

        for key in ["gauge_1", "gauge_2"] {
            let variant = &kcdata.maps[&14].variants[key];
            assert!(variant.enemy_fleets.contains_key(&6), "missing fleet in {key}");
            assert!(variant.ship_drops.contains_key(&6), "missing drop table in {key}");
        }
    }

    // ------------------------------------------------------------------ edge: to_cell_no missing

    /// Target variant missing cell 5 → rule dropped, counter = 1.
    #[test]
    fn test_fanout_drops_rule_missing_to_cell() {
        let mut kcdata = make_catalog(11, vec![make_variant("gauge_1", &[1, 3], vec![])]);
        // Wikiwiki includes cell 3 and 5 so cell_no_map has {3→3}; 5 has no kcdata match
        // (remaps to 5 via identity) and merge_routing_overlay drops it.
        let wikiwiki = make_catalog(11, vec![make_variant("", &[3, 5], vec![rule(3, 5)])]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 1);

        let v = &kcdata.maps[&11].variants["gauge_1"];
        assert!(v.routing_rules.is_empty(), "dropped rule must not appear");
    }

    // ------------------------------------------------------------------ edge: from_cell_no missing

    /// Target variant missing cell 3 (from) → rule dropped, counter = 1.
    #[test]
    fn test_fanout_drops_rule_missing_from_cell() {
        let mut kcdata = make_catalog(11, vec![make_variant("gauge_1", &[1, 5], vec![])]);
        let wikiwiki = make_catalog(11, vec![make_variant("", &[], vec![rule(3, 5)])]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 1);

        let v = &kcdata.maps[&11].variants["gauge_1"];
        assert!(v.routing_rules.is_empty());
    }

    // ------------------------------------------------------------------ edge: both cells missing

    /// Both from and to absent → still only 1 rule dropped (caught at from check), counter = 1.
    #[test]
    fn test_fanout_drops_rule_both_cells_missing() {
        let mut kcdata = make_catalog(11, vec![make_variant("gauge_1", &[1, 2], vec![])]);
        let wikiwiki = make_catalog(11, vec![make_variant("", &[], vec![rule(3, 5)])]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 1);
    }

    // ------------------------------------------------------------------ edge: other rules in same batch still merged

    /// Two rules in the same batch: one valid, one with bad `to_cell_no`.
    /// Valid rule is merged; only the bad one is counted.
    #[test]
    fn test_fanout_partial_drop_other_rules_still_merged() {
        let mut kcdata = make_catalog(11, vec![make_variant("gauge_1", &[1, 3, 5], vec![])]);
        // rule(3,5) is valid; rule(3,99) has bad to_cell_no
        let wikiwiki = make_catalog(11, vec![make_variant("", &[], vec![rule(3, 5), rule(3, 99)])]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 1);

        let v = &kcdata.maps[&11].variants["gauge_1"];
        let rules_from_3 = v.routing_rules.get(&3).expect("rule 3→5 should be present");
        assert!(rules_from_3.iter().any(|r| r.to_cell_no == 5));
        assert!(!rules_from_3.iter().any(|r| r.to_cell_no == 99));
    }

    // ------------------------------------------------------------------ integration: multi-gauge

    /// gauge_1 has cell 5; gauge_2 does not.
    /// Rule with to=5 merges into gauge_1 only, dropped from gauge_2. Counter = 1.
    #[test]
    fn test_fanout_multi_gauge_selective_merge() {
        let mut kcdata = make_catalog(
            33,
            vec![
                make_variant("gauge_1", &[1, 3, 5], vec![]),
                make_variant("gauge_2", &[1, 3], vec![]),
            ],
        );
        let wikiwiki = make_catalog(33, vec![make_variant("", &[], vec![rule(3, 5)])]);

        let dropped = merge_routing_overlay_from_wikiwiki_legacy(&mut kcdata, &wikiwiki);
        assert_eq!(dropped, 1);

        // Rule present in gauge_1.
        let g1 = &kcdata.maps[&33].variants["gauge_1"];
        assert!(g1.routing_rules.get(&3).is_some_and(|r| r.iter().any(|x| x.to_cell_no == 5)));

        // Rule absent from gauge_2.
        let g2 = &kcdata.maps[&33].variants["gauge_2"];
        assert!(g2.routing_rules.is_empty());
    }
}
