use emukc_model::codex::map::{build_cell_no_map, merge_routing_overlay, MapCatalog};

use super::{
	report::{MapCatalogBuildReport, MapCatalogStatSource},
	sources::ResolvedMapSources,
};

pub(super) fn assemble_final_map_catalog(
	sources: ResolvedMapSources,
) -> (MapCatalog, MapCatalogBuildReport) {
	let mut catalog = match sources.kcdata_catalog {
		Some(mut kcdata) => {
			if let Some(wikiwiki) = sources.wikiwiki_catalog {
				merge_routing_overlay_from_wikiwiki(&mut kcdata, &wikiwiki);
			}
			kcdata
		}
		None => sources.wikiwiki_catalog.unwrap_or_default(),
	};
	catalog.merge_missing_from(sources.public_overlay_catalog);
	if let Some(ref stat_catalog) = sources.stat_catalog {
		catalog.merge_missing_from(stat_catalog.clone());
	}
	let output_map_count = catalog.maps.len();

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
		},
	)
}

/// Overlay WikiWiki routing rules, enemy fleets, and ship drops onto kcdata topology.
/// Does NOT touch cells or next_cells — kcdata is the sole source of graph topology.
fn merge_routing_overlay_from_wikiwiki(kcdata: &mut MapCatalog, wikiwiki: &MapCatalog) {
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

			let apply_overlay = |kcdata_variant: &mut _| {
				let cell_no_map = build_cell_no_map(kcdata_variant, &other_labels);
				merge_routing_overlay(
					kcdata_variant,
					&cell_no_map,
					&wikiwiki_variant.routing_rules,
					&wikiwiki_variant.enemy_fleets,
					&wikiwiki_variant.ship_drops,
				);
			};

			if variant_key.is_empty() && definition_has_named_variants {
				for variant in kcdata_map.variants.values_mut() {
					apply_overlay(variant);
				}
			} else if let Some(kcdata_variant) = kcdata_map.variants.get_mut(variant_key) {
				apply_overlay(kcdata_variant);
			}
		}
	}
}
