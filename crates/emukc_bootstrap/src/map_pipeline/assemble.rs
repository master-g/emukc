use emukc_model::codex::map::MapCatalog;

use crate::{
    compass_route_rules::{CompassRouteRulesAsset, apply_route_rules},
    parser::error::ParseError,
};

use super::{
    label_overlay::merge_label_overlay,
    report::{MapCatalogBuildReport, MapCatalogStatSource},
    sources::ResolvedMapSources,
};

/// Assemble the final catalog in its one fixed order: kcdata → public overlay →
/// `stat.json` → `p_unlock` normalization → label overlay → compass
/// routing rules.
///
/// The label overlay and the routing rules go last because they are the only steps
/// that resolve labels to cell numbers, so they must see the final variant set and
/// topology. They only write routing rules, enemy fleets and ship drops, which no
/// earlier source carries, so running them last leaves every metadata authority
/// rule unchanged. The compass rules replace the routing rules the overlay wrote.
pub(super) fn assemble_final_map_catalog(
    sources: ResolvedMapSources,
) -> Result<(MapCatalog, MapCatalogBuildReport), ParseError> {
    let mut catalog = sources.kcdata_catalog;
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

    // Before anything is pinned onto cells: what a cell is decides what belongs on it.
    if let Some(cell_events) = &sources.cell_events {
        let corrected =
            crate::kcnav::apply_cell_events(&mut catalog, cell_events).map_err(|errors| {
                ParseError::Generic(format!(
                    "recorded cell kinds do not fit the map topology:\n{}",
                    errors.join("\n")
                ))
            })?;
        tracing::info!(corrected, "map catalog: cell kinds corrected from recorded routes");
    }

    let overlay_items_dropped = sources
        .label_overlay
        .as_ref()
        .map(|overlay| merge_label_overlay_catalog(&mut catalog, overlay))
        .unwrap_or(0);

    if let Some(route_rules) = &sources.route_rules {
        apply_route_rules_catalog(&mut catalog, route_rules)?;
    }

    mark_enemy_combined_cells(&mut catalog);

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

    Ok((
        catalog,
        MapCatalogBuildReport {
            label_overlay_map_count: sources.label_overlay_map_count,
            public_overlay_map_count: sources.public_overlay_map_count,
            stat_map_count: sources.stat_map_count,
            stat_source,
            output_map_count,
            fanout_rules_dropped: overlay_items_dropped,
            kcdata_parse_errors: sources.kcdata_parse_errors,
            topology_warnings,
        },
    ))
}

/// A cell whose every recorded enemy composition is a combined fleet gets event kind 5,
/// which is what makes the client request `ec_battle` there (`map_info.isVS12()`).
///
/// The captured start responses carry the kind for one cell per node at best, and the
/// recorded routes carry none, so the fleets themselves are the only complete witness.
fn mark_enemy_combined_cells(catalog: &mut MapCatalog) {
    for definition in catalog.maps.values_mut() {
        for variant in definition.variants.values_mut() {
            for (cell_no, fleet) in &variant.enemy_fleets {
                let combined = !fleet.compositions.is_empty()
                    && fleet.compositions.iter().all(|comp| !comp.escort_ship_ids.is_empty());
                if !combined {
                    continue;
                }
                if let Some(cell) = variant.cells.iter_mut().find(|c| c.cell_no == *cell_no) {
                    cell.event_kind = 5;
                }
            }
        }
    }
}

/// Pin the converted routing rules onto every map the catalog has. A rule the topology
/// cannot carry fails the build: the rules and the topology are meant to describe the
/// same maps, and a silent drop would turn a branch into a coin toss.
fn apply_route_rules_catalog(
    catalog: &mut MapCatalog,
    route_rules: &CompassRouteRulesAsset,
) -> Result<(), ParseError> {
    let mut errors = Vec::new();
    for (map_id, variants) in &route_rules.maps {
        let Some(definition) = catalog.maps.get_mut(map_id) else {
            continue;
        };
        for (variant_key, rules) in variants {
            match definition.variants.get_mut(variant_key) {
                Some(variant) => {
                    if let Err(variant_errors) = apply_route_rules(variant, rules) {
                        errors.extend(
                            variant_errors
                                .into_iter()
                                .map(|err| format!("map {map_id} variant `{variant_key}`: {err}")),
                        );
                    }
                }
                None => errors.push(format!("map {map_id} has no variant `{variant_key}`")),
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ParseError::Generic(format!(
            "routing rules do not fit the map topology:\n{}",
            errors.join("\n")
        )))
    }
}

/// Merge the label-keyed overlay onto the assembled topology, resolving
/// each label to the cells that carry it.
fn merge_label_overlay_catalog(
    catalog: &mut MapCatalog,
    overlay_catalog: &crate::parser::label_overlay::LabelOverlayCatalog,
) -> usize {
    let mut total_dropped = 0usize;

    for (map_id, overlay_def) in &overlay_catalog.maps {
        let Some(definition) = catalog.maps.get_mut(map_id) else {
            continue;
        };
        for (variant_key, overlay) in &overlay_def.variants {
            for key in definition.fan_out_variant_keys(variant_key) {
                match definition.variants.get_mut(&key) {
                    Some(variant) => total_dropped += merge_label_overlay(variant, overlay),
                    None => tracing::warn!(
                        map_id,
                        variant_key = %key,
                        "label overlay names a variant the assembled map does not have; skipped"
                    ),
                }
            }
        }
    }

    total_dropped
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use emukc_model::codex::map::{
        MapCatalog, MapCellDefinition, MapDefinition, MapVariantDefinition, RoutePredicate,
    };

    use super::assemble_final_map_catalog;
    use crate::map_pipeline::sources::ResolvedMapSources;
    use crate::parser::label_overlay::{
        LabelOverlay, LabelOverlayCatalog, LabelOverlayDefinition, RouteRuleDraft,
    };

    /// Build a [`MapCellDefinition`] with an auto-generated node label `C{cell_no}`.
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

    /// Build a `MapVariantDefinition` with the given cell numbers, auto-labeled `"C{n}"`.
    fn make_variant(key: &str, cell_nos: &[i64]) -> MapVariantDefinition {
        MapVariantDefinition {
            variant_key: key.to_owned(),
            cells: cell_nos.iter().map(|&n| make_cell(n)).collect(),
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

    /// A label-space overlay for one variant carrying a single `from → to` rule.
    fn label_rule(variant_key: &str, from: &str, to: &str) -> (String, LabelOverlay) {
        (
            variant_key.to_owned(),
            LabelOverlay {
                variant_key: variant_key.to_owned(),
                routing_rules: vec![RouteRuleDraft {
                    from_label: from.to_owned(),
                    to_label: to.to_owned(),
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: String::new(),
                    random_placeholder: false,
                }],
                ..Default::default()
            },
        )
    }

    // ------------------------------------------------------------------ P-unlock normalization

    fn cell_with_next(cell_no: i64, next_cells: Vec<i64>) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            next_cells,
            node_label: Some(format!("C{cell_no}")),
            ..Default::default()
        }
    }

    fn skeleton(cell_no: i64) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            master_cell_id: Some(1000 + cell_no),
            ..Default::default()
        }
    }

    /// A `p_unlock` map arrives as one unnamed kcdata variant while its overlay
    /// routing is keyed `pre_p_unlock` / `post_p_unlock`; those variants only exist
    /// once the public overlay is merged and normalized. Each must still get its own
    /// routing — 7-3 once silently lost all of it.
    #[test]
    fn p_unlock_variants_each_receive_their_own_routing() {
        // The kcdata base carries the topology, so cell 0 has to lead somewhere:
        // normalization refuses to split a map whose default would be unroutable.
        let mut kcdata = make_catalog(73, vec![make_variant("", &[0, 1, 2, 3])]);
        {
            // 0 → 1 → 2 → 3: the overlay resolves its label pairs against this graph,
            // and normalization refuses to split a map whose default is unroutable.
            let base = kcdata.maps.get_mut(&73).unwrap().variants.get_mut("").unwrap();
            for (index, next) in [1, 2, 3].into_iter().enumerate() {
                base.cells[index].next_cells = vec![next];
            }
        }
        let overlay = LabelOverlayCatalog {
            maps: BTreeMap::from([(
                73,
                LabelOverlayDefinition {
                    map_id: 73,
                    variants: BTreeMap::from([
                        label_rule("pre_p_unlock", "C1", "C2"),
                        label_rule("post_p_unlock", "C2", "C3"),
                    ]),
                },
            )]),
        };
        // The public overlay is what introduces the two p_unlock variants.
        let public_overlay = make_catalog(
            73,
            vec![
                make_variant("pre_p_unlock", &[0, 1, 2, 3]),
                make_variant("post_p_unlock", &[0, 1, 2, 3]),
            ],
        );

        let sources = ResolvedMapSources {
            label_overlay: Some(overlay),
            route_rules: None,
            cell_events: None,
            public_overlay_catalog: public_overlay,
            ..sources_from_kcdata(kcdata)
        };
        let (catalog, _report) = assemble_final_map_catalog(sources).unwrap();

        let variants = &catalog.maps[&73].variants;
        assert!(!variants.contains_key(""), "normalization drops the unnamed base");
        for (key, from, to) in [("pre_p_unlock", 1, 2), ("post_p_unlock", 2, 3)] {
            let rules = &variants[key].routing_rules;
            assert!(
                rules.get(&from).is_some_and(|rs| rs.iter().any(|r| r.to_cell_no == to)),
                "expected rule {from}\u{2192}{to} in {key}, got {rules:?}"
            );
        }
    }

    fn sources_from_kcdata(kcdata: MapCatalog) -> ResolvedMapSources {
        ResolvedMapSources {
            label_overlay_map_count: 0,
            label_overlay: None,
            route_rules: None,
            cell_events: None,
            kcdata_catalog: kcdata,
            kcdata_parse_errors: 0,
            public_overlay_map_count: 0,
            public_overlay_catalog: MapCatalog::default(),
            stat_map_count: 0,
            stat_catalog: None,
            stat_from_cache: false,
        }
    }

    /// `assemble_final_map_catalog` must run `normalize_p_unlock_variants` on every map: an
    /// aligned P-unlock map exits canonical (pre/post, `""` dropped, topology folded) while a
    /// plain map is untouched. This pins the assembly wiring that the disk-dependent sortie
    /// tests only cover indirectly.
    #[test]
    fn assemble_normalizes_p_unlock_and_leaves_plain_maps_untouched() {
        let mut kcdata = MapCatalog::default();

        // Map 73: "" base carries topology; pre/post are aligned topology-less skeletons.
        let base = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 9,
            cells: vec![
                cell_with_next(0, vec![1]),
                cell_with_next(1, vec![2]),
                cell_with_next(2, vec![]),
            ],
            ..Default::default()
        };
        let pre = MapVariantDefinition {
            variant_key: "pre_p_unlock".to_string(),
            boss_cell_no: 1,
            cells: vec![skeleton(0), skeleton(1)],
            ..Default::default()
        };
        let post = MapVariantDefinition {
            variant_key: "post_p_unlock".to_string(),
            boss_cell_no: 2,
            cells: vec![skeleton(0), skeleton(1), skeleton(2)],
            ..Default::default()
        };
        kcdata.maps.insert(
            73,
            MapDefinition {
                map_id: 73,
                required_defeat_count: Some(3),
                gauge_count: Some(1),
                variants: BTreeMap::from([
                    (String::new(), base),
                    ("pre_p_unlock".to_string(), pre),
                    ("post_p_unlock".to_string(), post),
                ]),
                ..Default::default()
            },
        );

        // Control: a plain single-variant map must be left alone.
        kcdata.maps.insert(
            11,
            MapDefinition {
                map_id: 11,
                variants: BTreeMap::from([(
                    String::new(),
                    MapVariantDefinition {
                        variant_key: String::new(),
                        boss_cell_no: 1,
                        cells: vec![cell_with_next(0, vec![1]), cell_with_next(1, vec![])],
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );

        let (catalog, _report) = assemble_final_map_catalog(sources_from_kcdata(kcdata)).unwrap();

        let m73 = &catalog.maps[&73];
        assert_eq!(m73.default_variant, "pre_p_unlock");
        assert_eq!(m73.gauge_count, Some(2));
        assert!(!m73.variants.contains_key(""), "spurious \"\" variant dropped");
        assert!(
            m73.variants.contains_key("pre_p_unlock") && m73.variants.contains_key("post_p_unlock")
        );
        // pre received the folded start (base 0 → [1], restricted to pre's {0,1}).
        assert_eq!(m73.variants["pre_p_unlock"].cell(0).unwrap().next_cells, vec![1]);
        assert_eq!(
            m73.variants["pre_p_unlock"].clear_to_variant_key.as_deref(),
            Some("post_p_unlock")
        );

        // Control map untouched.
        let m11 = &catalog.maps[&11];
        assert_eq!(m11.default_variant, "");
        assert!(m11.variants.contains_key(""));
    }

    /// The public overlay's real `api_req_map/start` captures are the only source of
    /// `master_cell_id`; assembly must carry them onto the kcdata cells.
    #[test]
    fn assemble_applies_public_master_cell_ids() {
        let mut kcdata = MapCatalog::default();
        kcdata.maps.insert(
            11,
            MapDefinition {
                map_id: 11,
                variants: BTreeMap::from([(
                    String::new(),
                    MapVariantDefinition {
                        cells: vec![
                            cell_with_next(0, vec![1]),
                            cell_with_next(1, vec![2, 3]),
                            cell_with_next(2, vec![]),
                            cell_with_next(3, vec![]),
                        ],
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );
        let sources = ResolvedMapSources {
            public_overlay_catalog: crate::map_pipeline::sources::load_public_map_catalog_overlays(
            )
            .unwrap(),
            ..sources_from_kcdata(kcdata)
        };

        let (catalog, _report) = assemble_final_map_catalog(sources).unwrap();
        let ids = catalog.maps[&11].variants[""]
            .cells
            .iter()
            .map(|cell| cell.master_cell_id)
            .collect::<Vec<_>>();

        assert_eq!(ids, vec![Some(3001), Some(3002), Some(3003), Some(3004)]);
    }

    /// An overlay keyed `""` is map-wide: it reaches every named variant when the
    /// map has any, and the unnamed variant when it does not.
    #[test]
    fn map_wide_overlay_fans_out_to_named_variants() {
        let linked = |key: &str| {
            let mut variant = make_variant(key, &[0, 1]);
            variant.cells[0].next_cells = vec![1];
            variant
        };
        let mut kcdata = make_catalog(15, vec![linked("first"), linked("second")]);
        kcdata.maps.insert(11, make_catalog(11, vec![linked("")]).maps.remove(&11).unwrap());
        let overlay = LabelOverlayCatalog {
            maps: [15, 11]
                .into_iter()
                .map(|map_id| {
                    (
                        map_id,
                        LabelOverlayDefinition {
                            map_id,
                            variants: BTreeMap::from([label_rule("", "C0", "C1")]),
                        },
                    )
                })
                .collect(),
        };

        let (catalog, _report) = assemble_final_map_catalog(ResolvedMapSources {
            label_overlay: Some(overlay),
            route_rules: None,
            cell_events: None,
            ..sources_from_kcdata(kcdata)
        })
        .unwrap();

        for (map_id, key) in [(15, "first"), (15, "second"), (11, "")] {
            let rules = &catalog.maps[&map_id].variants[key].routing_rules;
            assert!(
                rules.get(&0).is_some_and(|rs| rs.iter().any(|r| r.to_cell_no == 1)),
                "map {map_id} variant `{key}` should carry the map-wide rule, got {rules:?}"
            );
        }
    }
}
