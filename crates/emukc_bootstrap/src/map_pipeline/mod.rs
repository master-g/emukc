use std::path::Path;

use emukc_model::{codex::map::MapCatalog, kc2::start2::ApiManifest};

use crate::parser::error::ParseError;

mod assemble;
mod kcdata;
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
    build_final_map_catalog_with_report(data_root, manifest, wikiwiki_catalog)
        .map(|(catalog, _)| catalog)
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
) -> Result<(MapCatalog, MapCatalogBuildReport), ParseError> {
    let source_set =
        sources::load_explicit_source_set(data_root.as_ref(), manifest, wikiwiki_catalog)?;
    Ok(assemble::assemble_final_map_catalog(source_set))
}

/// Build the final runtime `MapCatalog` from repo assets and return provenance metadata.
pub fn build_final_map_catalog_from_repo_assets_with_report(
    data_root: impl AsRef<Path>,
    manifest: &ApiManifest,
) -> Result<(MapCatalog, MapCatalogBuildReport), ParseError> {
    let source_set = sources::load_repo_source_set(data_root.as_ref(), manifest)?;
    Ok(assemble::assemble_final_map_catalog(source_set))
}
