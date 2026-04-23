use std::path::{Path, PathBuf};

use super::types::{
    AudioResourcesAsset, CacheRulesAsset, DecoderCoverageAssets, MANIFEST_VERSION,
    ResourceCategoriesAsset, ResourceIdSetsAsset, ResourceManifest, UiResourcesAsset,
};
use crate::prelude::CacheListMakingError;

const MANIFEST_FILE: &str = "resource_manifest.json";
const CACHE_RULES_FILE: &str = "cache_rules.json";
const RESOURCE_CATEGORIES_FILE: &str = "resource_categories.json";
const RESOURCE_ID_SETS_FILE: &str = "resource_id_sets.json";
const AUDIO_RESOURCES_FILE: &str = "audio_resources.json";
const UI_RESOURCES_FILE: &str = "ui_resources.json";

fn manifest_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("assets");
    p.push(MANIFEST_FILE);
    p
}

pub(crate) fn load_resource_manifest() -> Result<ResourceManifest, CacheListMakingError> {
    load_resource_manifest_from_path(manifest_path())
}

pub(crate) fn load_cache_rules() -> Result<CacheRulesAsset, CacheListMakingError> {
    load_cache_rules_from_path(repo_asset_path(CACHE_RULES_FILE))
}

fn repo_asset_path(file_name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("assets");
    p.push(file_name);
    p
}

fn load_optional_json_file<T>(path: &Path) -> Result<Option<T>, CacheListMakingError>
where
    T: serde::de::DeserializeOwned,
{
    if !path.exists() {
        return Ok(None);
    }

    let raw = std::fs::read_to_string(path).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to read decoder coverage asset: {e}"))
    })?;
    let parsed = serde_json::from_str(&raw).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to parse decoder coverage asset: {e}"))
    })?;
    Ok(Some(parsed))
}

pub(crate) fn load_decoder_coverage_assets() -> Result<DecoderCoverageAssets, CacheListMakingError>
{
    Ok(DecoderCoverageAssets {
        resource_categories: load_optional_json_file(&repo_asset_path(RESOURCE_CATEGORIES_FILE))?,
        resource_id_sets: load_optional_json_file(&repo_asset_path(RESOURCE_ID_SETS_FILE))?,
        audio_resources: load_optional_json_file(&repo_asset_path(AUDIO_RESOURCES_FILE))?,
        ui_resources: load_optional_json_file(&repo_asset_path(UI_RESOURCES_FILE))?,
    })
}

pub(crate) fn load_cache_rules_from_path(
    path: impl AsRef<std::path::Path>,
) -> Result<CacheRulesAsset, CacheListMakingError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(CacheListMakingError::Other(format!(
            "Cache rules asset not found: {:?}\nRun `bun run decode -- --sync-assets` to generate it.",
            path
        )));
    }

    let raw = std::fs::read_to_string(path).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to read cache rules asset: {e}"))
    })?;

    serde_json::from_str(&raw)
        .map_err(|e| CacheListMakingError::Other(format!("Failed to parse cache rules asset: {e}")))
}

pub(crate) fn load_decoder_coverage_assets_from_manifest_path(
    manifest_path: impl AsRef<Path>,
) -> Result<DecoderCoverageAssets, CacheListMakingError> {
    let manifest_path = manifest_path.as_ref();
    let sibling_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let repo_assets = load_decoder_coverage_assets()?;

    let load_or_fallback = |file_name: &str| -> Result<PathBuf, CacheListMakingError> {
        let sibling = sibling_dir.join(file_name);
        if sibling.exists() {
            Ok(sibling)
        } else {
            Ok(repo_asset_path(file_name))
        }
    };

    let categories_path = load_or_fallback(RESOURCE_CATEGORIES_FILE)?;
    let id_sets_path = load_or_fallback(RESOURCE_ID_SETS_FILE)?;
    let audio_path = load_or_fallback(AUDIO_RESOURCES_FILE)?;
    let ui_path = load_or_fallback(UI_RESOURCES_FILE)?;

    Ok(DecoderCoverageAssets {
        resource_categories: load_optional_json_file::<ResourceCategoriesAsset>(&categories_path)?
            .or(repo_assets.resource_categories),
        resource_id_sets: load_optional_json_file::<ResourceIdSetsAsset>(&id_sets_path)?
            .or(repo_assets.resource_id_sets),
        audio_resources: load_optional_json_file::<AudioResourcesAsset>(&audio_path)?
            .or(repo_assets.audio_resources),
        ui_resources: load_optional_json_file::<UiResourcesAsset>(&ui_path)?
            .or(repo_assets.ui_resources),
    })
}

pub(crate) fn load_resource_manifest_from_path(
    path: impl AsRef<std::path::Path>,
) -> Result<ResourceManifest, CacheListMakingError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(CacheListMakingError::Other(format!(
            "Resource manifest not found: {:?}\nRun `bun run decode -- --sync-resource-manifest` to generate it.",
            path
        )));
    }

    let raw = std::fs::read_to_string(path).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to read resource manifest: {e}"))
    })?;

    let manifest: ResourceManifest = serde_json::from_str(&raw).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to parse resource manifest: {e}"))
    })?;

    if manifest.version != MANIFEST_VERSION {
        warn!(
            "Resource manifest version mismatch: expected {}, got {}. Proceeding anyway.",
            MANIFEST_VERSION, manifest.version
        );
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_real_manifest() {
        let result = load_resource_manifest();
        assert!(result.is_ok(), "Should load the real resource_manifest.json");
        let manifest = result.unwrap();
        assert!(matches!(manifest.version, 1 | 2));
        assert!(!manifest.entries.is_empty());
    }
}
