use emukc_model::codex::map::MapCatalog;

use super::{
    report::{MapCatalogBuildReport, MapCatalogStatSource},
    sources::ResolvedMapSources,
};

pub(super) fn assemble_final_map_catalog(
    sources: ResolvedMapSources,
) -> (MapCatalog, MapCatalogBuildReport) {
    let mut catalog = match sources.kcdata_catalog {
        Some(mut kcdata) => {
            if let Some(ref wikiwiki) = sources.wikiwiki_catalog {
                for map_id in wikiwiki.maps.keys() {
                    if let Some(kcdata_map) = kcdata.maps.get_mut(map_id) {
                        for variant in kcdata_map.variants.values_mut() {
                            variant.routing_rules.clear();
                        }
                    }
                }
            }
            if let Some(wikiwiki) = sources.wikiwiki_catalog {
                // Neither wikiwiki nor kcdata parsers set master_cell_id;
                // runtime falls back to map_id * 100 + cell_no.
                kcdata.merge_missing_from(wikiwiki);
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
