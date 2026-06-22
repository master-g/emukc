use std::path::Path;

use emukc_model::{codex::map::MapCatalog, kc2::start2::ApiManifest};

use crate::parser::{error::ParseError, wikiwiki_map::WikiwikiMapOverlayCatalog};

mod assemble;
mod kcdata;
mod label_overlay;
mod report;
mod sources;
#[cfg(test)]
mod verify;

pub use report::{MapCatalogBuildReport, MapCatalogStatSource, MapCatalogWikiwikiSource};

/// Build the final runtime `MapCatalog` from explicit normalized inputs.
pub fn build_final_map_catalog(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
    wikiwiki_catalog: Option<MapCatalog>,
) -> Result<MapCatalog, ParseError> {
    build_final_map_catalog_with_report(data_root, manifest, wikiwiki_catalog, None)
        .map(|(catalog, _)| catalog)
}

/// Build the final runtime `MapCatalog` from explicit inputs with overlay.
pub fn build_final_map_catalog_with_overlay(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
    wikiwiki_catalog: Option<MapCatalog>,
    wikiwiki_overlay: Option<WikiwikiMapOverlayCatalog>,
) -> Result<MapCatalog, ParseError> {
    build_final_map_catalog_with_report(data_root, manifest, wikiwiki_catalog, wikiwiki_overlay)
        .map(|(catalog, _)| catalog)
}

/// Remove maps not present in the manifest's `api_mst_mapinfo`.
///
/// Event/seasonal maps that exist in kcdata but aren't in the current
/// `start2.json` are excluded — they produce topology warnings and aren't
/// playable without the event being active.
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

/// Build the final runtime `MapCatalog` using the repo-tracked wikiwiki asset.
pub fn build_final_map_catalog_from_repo_assets(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
) -> Result<MapCatalog, ParseError> {
    build_final_map_catalog_from_repo_assets_with_report(data_root, manifest)
        .map(|(catalog, _)| catalog)
}

/// Build the final runtime `MapCatalog` and return bootstrap-owned provenance metadata.
pub fn build_final_map_catalog_with_report(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
    wikiwiki_catalog: Option<MapCatalog>,
    wikiwiki_overlay: Option<WikiwikiMapOverlayCatalog>,
) -> Result<(MapCatalog, MapCatalogBuildReport), ParseError> {
    let source_set = sources::load_explicit_source_set(
        data_root.as_ref(),
        manifest,
        wikiwiki_catalog,
        wikiwiki_overlay,
    )?;
    let (mut catalog, report) = assemble::assemble_final_map_catalog(source_set);
    filter_to_manifest_maps(&mut catalog, manifest);
    Ok((catalog, report))
}

/// Build the final runtime `MapCatalog` from repo assets and return provenance metadata.
pub fn build_final_map_catalog_from_repo_assets_with_report(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
) -> Result<(MapCatalog, MapCatalogBuildReport), ParseError> {
    let source_set = sources::load_repo_source_set(data_root.as_ref(), manifest)?;
    let (mut catalog, report) = assemble::assemble_final_map_catalog(source_set);
    filter_to_manifest_maps(&mut catalog, manifest);
    Ok((catalog, report))
}
