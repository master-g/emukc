use std::path::Path;

use emukc_model::{codex::map::MapCatalog, kc2::start2::ApiManifest};

use crate::parser::error::ParseError;

mod assemble;
mod kcdata;
mod label_overlay;
mod report;
mod sources;
#[cfg(test)]
mod verify;

pub use report::{MapCatalogBuildReport, MapCatalogStatSource};

/// Build the final runtime `MapCatalog` and its provenance report.
///
/// Reads kcdata, `stat.json` and the manifest from `data_root`; everything else comes
/// from the repo-tracked assets.
pub fn build_final_map_catalog(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
) -> Result<(MapCatalog, MapCatalogBuildReport), ParseError> {
    let source_set = sources::load_repo_source_set(data_root.as_ref(), manifest)?;
    let (mut catalog, report) = assemble::assemble_final_map_catalog(source_set)?;
    filter_to_manifest_maps(&mut catalog, manifest);
    Ok((catalog, report))
}

/// Remove maps not present in the manifest's `api_mst_mapinfo`.
///
/// Event/seasonal maps that exist in kcdata but aren't in the current
/// `start2.json` are excluded — they produce topology warnings and aren't
/// playable without the event being active.
fn filter_to_manifest_maps(catalog: &mut MapCatalog, manifest: &ApiManifest) {
    if manifest.api_mst_mapinfo.is_empty() {
        return;
    }
    let known_ids: std::collections::BTreeSet<i64> =
        manifest.api_mst_mapinfo.iter().map(|m| m.api_id).collect();
    catalog.maps.retain(|map_id, _| known_ids.contains(map_id));
}
