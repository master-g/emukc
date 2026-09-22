use emukc_model::codex::map::MapCatalog;

use super::{
    label_overlay::merge_label_overlay,
    report::{MapCatalogBuildReport, MapCatalogStatSource},
    sources::ResolvedMapSources,
};

pub(super) fn assemble_final_map_catalog(
    sources: ResolvedMapSources,
) -> (MapCatalog, MapCatalogBuildReport) {
    let mut overlay_items_dropped = 0usize;

    // Auto-derive label-keyed overlay from the wikiwiki catalog when no explicit
    // overlay was provided. This bridges the incompatible cell-number spaces
    // (wikiwiki BFS vs kcdata route-ID) by converting cell-number-keyed data to
    // label-keyed data that merge_label_overlay() can apply correctly.
    let wikiwiki_overlay = sources.wikiwiki_overlay.or_else(|| {
        let wikiwiki_catalog = sources.wikiwiki_catalog.as_ref()?;
        let mut overlay_catalog = crate::parser::wikiwiki_map::WikiwikiMapOverlayCatalog::default();
        for (map_id, wikiwiki_map) in &wikiwiki_catalog.maps {
            let mut overlay_def = crate::parser::wikiwiki_map::WikiwikiMapOverlayDefinition {
                map_id: *map_id,
                variants: std::collections::BTreeMap::new(),
            };
            for (variant_key, variant) in &wikiwiki_map.variants {
                let overlay = super::label_overlay::auto_derive_label_overlay(variant);
                overlay_def.variants.insert(variant_key.clone(), overlay);
            }
            if !overlay_def.variants.is_empty() {
                overlay_catalog.maps.insert(*map_id, overlay_def);
            }
        }
        if overlay_catalog.maps.is_empty() {
            None
        } else {
            Some(overlay_catalog)
        }
    });

    let mut deferred_overlays = Vec::new();
    let mut catalog = match (sources.kcdata_catalog, wikiwiki_overlay) {
        // New path: kcdata topology + label-keyed wikiwiki overlay.
        (Some(mut kcdata), Some(overlay)) => {
            overlay_items_dropped =
                merge_label_overlay_catalog(&mut kcdata, &overlay, &mut deferred_overlays);
            kcdata
        }
        // No overlay derivable, which only happens when the wikiwiki catalog is
        // absent or carries no variants — either way there is nothing to merge.
        (Some(kcdata), None) => kcdata,
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
    overlay_items_dropped += apply_deferred_label_overlays(&mut catalog, deferred_overlays);

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
    deferred: &mut Vec<DeferredLabelOverlay>,
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
            } else {
                // The variant does not exist in kcdata *yet*. A p_unlock map such as
                // 7-3 arrives here as a single unnamed kcdata variant, while the
                // wikiwiki routing is keyed `pre_p_unlock` / `post_p_unlock`; those
                // variants only appear once the public overlay is merged and
                // `normalize_p_unlock_variants` has run. Dropping the overlay here
                // silently cost 7-3 all of its routing, so hold it back instead.
                deferred.push(DeferredLabelOverlay {
                    map_id: *map_id,
                    variant_key: variant_key.clone(),
                    overlay: overlay.clone(),
                });
            }
        }
    }

    total_dropped
}

/// A label overlay whose target variant did not exist when the overlay was first
/// merged. Applied again once the variant set is final.
struct DeferredLabelOverlay {
    map_id: i64,
    variant_key: String,
    overlay: crate::parser::wikiwiki_map::WikiwikiLabelOverlay,
}

/// Apply the overlays held back by [`merge_label_overlay_catalog`], skipping any
/// whose variant still does not exist.
fn apply_deferred_label_overlays(
    catalog: &mut MapCatalog,
    deferred: Vec<DeferredLabelOverlay>,
) -> usize {
    let mut total_dropped = 0usize;

    for entry in deferred {
        if let Some(map) = catalog.maps.get_mut(&entry.map_id)
            && let Some(variant) = map.variants.get_mut(&entry.variant_key)
        {
            total_dropped += merge_label_overlay(variant, &entry.overlay);
        }
    }

    total_dropped
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use emukc_model::codex::map::{
        MapCatalog, MapCellDefinition, MapDefinition, MapVariantDefinition, RouteRule,
    };

    use super::assemble_final_map_catalog;
    use crate::map_pipeline::{report::MapCatalogWikiwikiSource, sources::ResolvedMapSources};

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

    // ------------------------------------------------------------------ happy path

    // ------------------------------------------------------------------ edge: to_cell_no missing

    // ------------------------------------------------------------------ edge: from_cell_no missing

    // ------------------------------------------------------------------ edge: both cells missing

    // ------------------------------------------------------------------ edge: other rules in same batch still merged

    // ------------------------------------------------------------------ integration: multi-gauge

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

    /// A `p_unlock` map arrives as one unnamed kcdata variant while its wikiwiki
    /// routing is keyed `pre_p_unlock` / `post_p_unlock`; those variants only exist
    /// after the public overlay is merged. The routing must survive that gap — 7-3
    /// silently lost all of it, and nothing counted the loss.
    #[test]
    fn p_unlock_routing_survives_the_variant_it_needs_appearing_late() {
        // The kcdata base carries the topology, so cell 0 has to lead somewhere:
        // normalization refuses to split a map whose default would be unroutable.
        let mut kcdata = make_catalog(73, vec![make_variant("", &[0, 1, 2, 3], vec![])]);
        {
            // 0 → 1 → 2 → 3: the overlay resolves its label pairs against this graph,
            // and normalization refuses to split a map whose default is unroutable.
            let base = kcdata.maps.get_mut(&73).unwrap().variants.get_mut("").unwrap();
            for (index, next) in [1, 2, 3].into_iter().enumerate() {
                base.cells[index].next_cells = vec![next];
            }
        }
        let wikiwiki = make_catalog(
            73,
            vec![
                make_variant("pre_p_unlock", &[0, 1, 2, 3], vec![rule(1, 2)]),
                make_variant("post_p_unlock", &[0, 1, 2, 3], vec![rule(2, 3)]),
            ],
        );
        // The public overlay is what introduces the two p_unlock variants.
        let public_overlay = make_catalog(
            73,
            vec![
                make_variant("pre_p_unlock", &[0, 1, 2, 3], vec![]),
                make_variant("post_p_unlock", &[0, 1, 2, 3], vec![]),
            ],
        );

        let sources = ResolvedMapSources {
            wikiwiki_catalog: Some(wikiwiki),
            public_overlay_catalog: public_overlay,
            ..sources_from_kcdata(kcdata)
        };
        let (catalog, _report) = assemble_final_map_catalog(sources);

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
            wikiwiki_source: MapCatalogWikiwikiSource::None,
            wikiwiki_map_count: 0,
            wikiwiki_catalog: None,
            wikiwiki_overlay: None,
            kcdata_catalog: Some(kcdata),
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

        let (catalog, _report) = assemble_final_map_catalog(sources_from_kcdata(kcdata));

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
}
