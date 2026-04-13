use std::path::{Path, PathBuf};

use emukc_model::{codex::map::MapCatalog, kc2::start2::ApiManifest};

use crate::{
    parser::error::ParseError,
    wikiwiki_map_asset::{
        RepoWikiwikiMapCatalogSource, load_repo_wikiwiki_map_catalog_asset,
        repo_wikiwiki_map_catalog_path,
    },
};

use super::{kcdata::load_map_catalog_from_kcdata_root, report::MapCatalogWikiwikiSource};

pub(super) struct ResolvedMapSources {
    pub(super) wikiwiki_source: MapCatalogWikiwikiSource,
    pub(super) wikiwiki_map_count: usize,
    pub(super) wikiwiki_catalog: Option<MapCatalog>,
    pub(super) kcdata_map_count: usize,
    pub(super) kcdata_catalog: MapCatalog,
    pub(super) public_overlay_map_count: usize,
    pub(super) public_overlay_catalog: MapCatalog,
}

pub(super) fn load_explicit_source_set(
    data_root: &Path,
    manifest: &ApiManifest,
    wikiwiki_catalog: Option<MapCatalog>,
) -> Result<ResolvedMapSources, ParseError> {
    let wikiwiki_map_count =
        wikiwiki_catalog.as_ref().map(|catalog| catalog.maps.len()).unwrap_or(0);
    let kcdata_catalog = load_map_catalog_from_kcdata_root(data_root.join("kc_data"), manifest)?;
    let kcdata_map_count = kcdata_catalog.maps.len();
    let public_overlay_catalog = load_public_map_catalog_overlays()?;
    let public_overlay_map_count = public_overlay_catalog.maps.len();

    Ok(ResolvedMapSources {
        wikiwiki_source: if wikiwiki_catalog.is_some() {
            MapCatalogWikiwikiSource::Provided
        } else {
            MapCatalogWikiwikiSource::None
        },
        wikiwiki_map_count,
        wikiwiki_catalog,
        kcdata_map_count,
        kcdata_catalog,
        public_overlay_map_count,
        public_overlay_catalog,
    })
}

pub(super) fn load_repo_source_set(
    data_root: &Path,
    manifest: &ApiManifest,
) -> Result<ResolvedMapSources, ParseError> {
    let (wikiwiki_source, wikiwiki_catalog) = load_repo_wikiwiki_map_catalog()?;
    let wikiwiki_map_count =
        wikiwiki_catalog.as_ref().map(|catalog| catalog.maps.len()).unwrap_or(0);
    let kcdata_catalog = load_map_catalog_from_kcdata_root(data_root.join("kc_data"), manifest)?;
    let kcdata_map_count = kcdata_catalog.maps.len();
    let public_overlay_catalog = load_public_map_catalog_overlays()?;
    let public_overlay_map_count = public_overlay_catalog.maps.len();

    Ok(ResolvedMapSources {
        wikiwiki_source,
        wikiwiki_map_count,
        wikiwiki_catalog,
        kcdata_map_count,
        kcdata_catalog,
        public_overlay_map_count,
        public_overlay_catalog,
    })
}

fn load_public_map_catalog_overlays() -> Result<MapCatalog, ParseError> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/public_map_catalog_overlays.json");
    serde_json::from_str::<MapCatalog>(include_str!(
        "../../assets/public_map_catalog_overlays.json"
    ))
    .map_err(|source| ParseError::json_at(&path, source))
}

fn load_repo_wikiwiki_map_catalog()
-> Result<(MapCatalogWikiwikiSource, Option<MapCatalog>), ParseError> {
    let path = repo_wikiwiki_map_catalog_path();
    let asset = load_repo_wikiwiki_map_catalog_asset()
        .map_err(|source| ParseError::io_at(&path, source))?;
    let source_name = match &asset.source {
        RepoWikiwikiMapCatalogSource::Filesystem(asset_path) => asset_path.display().to_string(),
        RepoWikiwikiMapCatalogSource::Embedded => {
            info!(
                "repo wikiwiki map catalog not found at {}; using embedded catalog asset",
                path.display()
            );
            "embedded wikiwiki map catalog".to_string()
        }
    };
    let source_kind = match asset.source {
        RepoWikiwikiMapCatalogSource::Filesystem(_) => MapCatalogWikiwikiSource::Filesystem,
        RepoWikiwikiMapCatalogSource::Embedded => MapCatalogWikiwikiSource::Embedded,
    };

    match serde_json::from_str::<MapCatalog>(asset.raw_json()) {
        Ok(catalog) => Ok((source_kind, Some(catalog))),
        Err(source) => {
            warn!("failed to parse {}: {}. Using kc_data-only map catalog", source_name, source);
            Ok((source_kind, None))
        }
    }
}
