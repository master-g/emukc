use emukc_model::codex::map::MapCatalog;

use super::{report::MapCatalogBuildReport, sources::ResolvedMapSources};

pub(super) fn assemble_final_map_catalog(
    sources: ResolvedMapSources,
) -> (MapCatalog, MapCatalogBuildReport) {
    let mut catalog = if let Some(mut wikiwiki_catalog) = sources.wikiwiki_catalog {
        wikiwiki_catalog.merge_missing_from(sources.kcdata_catalog);
        wikiwiki_catalog
    } else {
        sources.kcdata_catalog
    };
    catalog.merge_missing_from(sources.public_overlay_catalog);
    let output_map_count = catalog.maps.len();

    (
        catalog,
        MapCatalogBuildReport {
            wikiwiki_source: sources.wikiwiki_source,
            wikiwiki_map_count: sources.wikiwiki_map_count,
            kcdata_map_count: sources.kcdata_map_count,
            public_overlay_map_count: sources.public_overlay_map_count,
            output_map_count,
        },
    )
}
