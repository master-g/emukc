use std::path::{Path, PathBuf};

use super::types::{
    CacheRulesAsset, DecoderAssetSource, DecoderCoverageAssetSources, DecoderCoverageAssets,
    DecoderRulesBundle, MANIFEST_VERSION, ResourceManifest,
};
use crate::prelude::CacheListMakingError;

const MANIFEST_FILE: &str = "resource_manifest.json";
const CACHE_RULES_FILE: &str = "cache_rules.json";
const RESOURCE_CATEGORIES_FILE: &str = "resource_categories.json";
const RESOURCE_ID_SETS_FILE: &str = "resource_id_sets.json";
const AUDIO_RESOURCES_FILE: &str = "audio_resources.json";
const UI_RESOURCES_FILE: &str = "ui_resources.json";
const RESOURCE_TEMPLATES_FILE: &str = "resource_templates.json";

fn manifest_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("assets");
    p.push(MANIFEST_FILE);
    p
}

pub(crate) fn load_resource_manifest() -> Result<ResourceManifest, CacheListMakingError> {
    load_resource_manifest_from_path(manifest_path())
}

pub(crate) fn load_cache_rules_bundle() -> Result<DecoderRulesBundle, CacheListMakingError> {
    load_cache_rules_bundle_from_path(repo_asset_path(CACHE_RULES_FILE))
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

    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            warn!("Failed to read optional decoder coverage asset {:?}: {e}", path);
            return Ok(None);
        }
    };
    let parsed = match serde_json::from_str(&raw) {
        Ok(parsed) => parsed,
        Err(e) => {
            warn!("Failed to parse optional decoder coverage asset {:?}: {e}", path);
            return Ok(None);
        }
    };
    Ok(Some(parsed))
}

fn resolve_decoder_asset_path(
    sibling_dir: Option<&Path>,
    file_name: &str,
) -> (PathBuf, DecoderAssetSource) {
    if let Some(sibling_dir) = sibling_dir {
        let sibling = sibling_dir.join(file_name);
        if sibling.exists() {
            return (sibling, DecoderAssetSource::Sibling);
        }
    }

    let repo = repo_asset_path(file_name);
    if repo.exists() {
        return (repo, DecoderAssetSource::Repo);
    }

    (sibling_dir.unwrap_or(Path::new(".")).join(file_name), DecoderAssetSource::Missing)
}

fn load_decoder_coverage_assets_with_sources(
    sibling_dir: Option<&Path>,
) -> Result<(DecoderCoverageAssets, DecoderCoverageAssetSources), CacheListMakingError> {
    let (categories_path, categories_source) =
        resolve_decoder_asset_path(sibling_dir, RESOURCE_CATEGORIES_FILE);
    let (id_sets_path, id_sets_source) =
        resolve_decoder_asset_path(sibling_dir, RESOURCE_ID_SETS_FILE);
    let (audio_path, audio_source) = resolve_decoder_asset_path(sibling_dir, AUDIO_RESOURCES_FILE);
    let (ui_path, ui_source) = resolve_decoder_asset_path(sibling_dir, UI_RESOURCES_FILE);
    let (templates_path, templates_source) =
        resolve_decoder_asset_path(sibling_dir, RESOURCE_TEMPLATES_FILE);

    let resource_categories = load_optional_json_file(&categories_path)?;
    let resource_id_sets = load_optional_json_file(&id_sets_path)?;
    let audio_resources = load_optional_json_file(&audio_path)?;
    let ui_resources = load_optional_json_file(&ui_path)?;
    let resource_templates = load_optional_json_file(&templates_path)?;
    let resource_categories_source =
        source_for_loaded_asset(categories_source, &resource_categories);
    let resource_id_sets_source = source_for_loaded_asset(id_sets_source, &resource_id_sets);
    let audio_resources_source = source_for_loaded_asset(audio_source, &audio_resources);
    let ui_resources_source = source_for_loaded_asset(ui_source, &ui_resources);
    let resource_templates_source = source_for_loaded_asset(templates_source, &resource_templates);

    Ok((
        DecoderCoverageAssets {
            resource_categories,
            resource_id_sets,
            audio_resources,
            ui_resources,
            resource_templates,
        },
        DecoderCoverageAssetSources {
            resource_categories: resource_categories_source,
            resource_id_sets: resource_id_sets_source,
            audio_resources: audio_resources_source,
            ui_resources: ui_resources_source,
            resource_templates: resource_templates_source,
        },
    ))
}

fn source_for_loaded_asset<T>(source: DecoderAssetSource, asset: &Option<T>) -> DecoderAssetSource {
    if source == DecoderAssetSource::Missing || asset.is_some() {
        source
    } else {
        DecoderAssetSource::Missing
    }
}

pub(crate) fn load_decoder_coverage_assets() -> Result<DecoderCoverageAssets, CacheListMakingError>
{
    Ok(load_decoder_coverage_assets_with_sources(None)?.0)
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

pub(crate) fn load_cache_rules_bundle_from_path(
    path: impl AsRef<Path>,
) -> Result<DecoderRulesBundle, CacheListMakingError> {
    let path = path.as_ref();
    let cache_rules = load_cache_rules_from_path(path)?;
    let sibling_dir = path.parent().unwrap_or(Path::new("."));
    let (decoder_assets, decoder_asset_sources) =
        load_decoder_coverage_assets_with_sources(Some(sibling_dir))?;

    Ok(DecoderRulesBundle {
        cache_rules,
        decoder_assets,
        decoder_asset_sources,
    })
}

pub(crate) fn load_decoder_coverage_assets_from_manifest_path(
    manifest_path: impl AsRef<Path>,
) -> Result<DecoderCoverageAssets, CacheListMakingError> {
    let manifest_path = manifest_path.as_ref();
    let sibling_dir = manifest_path.parent().unwrap_or(Path::new("."));
    Ok(load_decoder_coverage_assets_with_sources(Some(sibling_dir))?.0)
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
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = repo_root().join(".data/tmp").join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_rules_file(path: &Path) {
        let payload = json!({
            "version": 1,
            "generatedAt": "2026-04-24T00:00:00Z",
            "scriptVersion": "6.2.8.0",
            "summary": {
                "shipRuleCount": 0,
                "slotRuleCount": 0,
                "observedCompleteRuleCount": 0,
                "partialRuleCount": 0,
                "unresolvedRuleCount": 0
            },
            "resourceManifest": {
                "version": 2,
                "generatedAt": "2026-04-24T00:00:00Z",
                "summary": {
                    "totalEntries": 0,
                    "shipEntryCount": 0,
                    "slotitemEntryCount": 0,
                    "textureProviderEntryCount": 0,
                    "explicitPathEntryCount": 0,
                    "totalExplicitPaths": 0,
                    "modulesCovered": 0
                },
                "pathRules": null,
                "entries": []
            },
            "resourceCategories": {
                "version": 1,
                "generatedAt": "2026-04-24T00:00:00Z",
                "scriptVersion": "6.2.8.0",
                "shipTargetTypes": [],
                "slotTargetTypes": [],
                "shipGenerationGroups": {
                    "defaultFriendly": [],
                    "defaultAbyssal": [],
                    "friendGraph": [],
                    "enemyGraph": []
                },
                "slotGenerationGroups": {
                    "default": [],
                    "baga": [],
                    "airunit": []
                },
                "spRemodelSubcategories": []
            },
            "shipRules": {
                "special": {
                    "coverageMode": "unresolved",
                    "kind": "special_cases",
                    "cases": [],
                    "moduleIds": [],
                    "moduleNames": []
                },
                "targetSemantics": {
                    "coverageMode": "unresolved",
                    "kind": "ship_target_semantics",
                    "cases": [],
                    "moduleIds": [],
                    "moduleNames": []
                }
            },
            "slotRules": {
                "itemUp": {
                    "coverageMode": "unresolved",
                    "kind": "item_up_normalization",
                    "replaceMap": {},
                    "exclude": [],
                    "moduleIds": [],
                    "moduleNames": []
                },
                "btxtFlat": {
                    "coverageMode": "unresolved",
                    "kind": "btxt_flat_non_enemy_runtime_slots",
                    "excludeEnemyItems": false,
                    "moduleIds": [],
                    "moduleNames": []
                },
                "itemUp2": {
                    "coverageMode": "unresolved",
                    "kind": "observed_slot_subset",
                    "ids": [],
                    "moduleIds": [],
                    "moduleNames": []
                },
                "itemOn2": {
                    "coverageMode": "unresolved",
                    "kind": "observed_slot_subset",
                    "ids": [],
                    "moduleIds": [],
                    "moduleNames": []
                }
            },
            "unresolvedRules": []
        });
        fs::write(path, serde_json::to_string_pretty(&payload).unwrap()).unwrap();
    }

    #[test]
    fn test_load_real_manifest() {
        let result = load_resource_manifest();
        assert!(result.is_ok(), "Should load the real resource_manifest.json");
        let manifest = result.unwrap();
        assert!(matches!(manifest.version, 1 | 2));
        assert!(!manifest.entries.is_empty());
    }

    #[test]
    fn test_load_cache_rules_bundle_from_path_prefers_sibling_assets() {
        let dir = tmp_dir("loader-rules-bundle-sibling");
        let rules_path = dir.join(CACHE_RULES_FILE);
        let audio_path = dir.join(AUDIO_RESOURCES_FILE);
        let templates_path = dir.join(RESOURCE_TEMPLATES_FILE);
        write_rules_file(&rules_path);
        fs::write(
            &audio_path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "generatedAt": "2026-04-24T00:00:00Z",
                "scriptVersion": "6.2.8.0",
                "seIds": { "coverageMode": "observed-complete", "ids": [999] },
                "bgm": {},
                "voice": {},
                "explicitPaths": []
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            &templates_path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "generatedAt": "2026-04-24T00:00:00Z",
                "scriptVersion": "6.2.8.0",
                "families": [{
                    "key": "map.base",
                    "domain": "map",
                    "outputPrefix": "kcs2/resources/map",
                    "pathTemplate": [],
                    "requiredInputs": ["manifest.mapinfo"],
                    "coverageMode": "observed-complete",
                    "provenance": { "moduleIds": ["2156"], "moduleNames": ["MapThumbnailImage"] }
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let bundle = load_cache_rules_bundle_from_path(&rules_path).unwrap();

        assert_eq!(bundle.decoder_asset_sources.audio_resources, DecoderAssetSource::Sibling);
        assert_eq!(bundle.decoder_asset_sources.resource_templates, DecoderAssetSource::Sibling);
        assert_eq!(bundle.decoder_assets.audio_resources.unwrap().se_ids.ids, vec![999]);
        assert_eq!(bundle.decoder_assets.resource_templates.unwrap().families[0].key, "map.base");
    }

    #[test]
    fn test_load_cache_rules_bundle_from_path_falls_back_to_repo_assets() {
        let dir = tmp_dir("loader-rules-bundle-repo-fallback");
        let rules_path = dir.join(CACHE_RULES_FILE);
        write_rules_file(&rules_path);

        let bundle = load_cache_rules_bundle_from_path(&rules_path).unwrap();

        assert_eq!(bundle.decoder_asset_sources.resource_categories, DecoderAssetSource::Repo);
        assert_eq!(bundle.decoder_asset_sources.resource_id_sets, DecoderAssetSource::Repo);
        assert_eq!(bundle.decoder_asset_sources.audio_resources, DecoderAssetSource::Repo);
        assert_eq!(bundle.decoder_asset_sources.ui_resources, DecoderAssetSource::Repo);
        assert_eq!(bundle.decoder_asset_sources.resource_templates, DecoderAssetSource::Repo);
        assert!(bundle.decoder_assets.resource_categories.is_some());
        assert!(bundle.decoder_assets.resource_id_sets.is_some());
        assert!(bundle.decoder_assets.audio_resources.is_some());
        assert!(bundle.decoder_assets.ui_resources.is_some());
        assert!(bundle.decoder_assets.resource_templates.is_some());
    }

    #[test]
    fn test_repo_fallback_asset_names_report_missing_siblings() {
        let dir = tmp_dir("loader-rules-bundle-fallback-report");
        let rules_path = dir.join(CACHE_RULES_FILE);
        write_rules_file(&rules_path);

        let bundle = load_cache_rules_bundle_from_path(&rules_path).unwrap();

        assert_eq!(
            bundle.decoder_asset_sources.repo_fallback_asset_names(),
            vec![
                "resource_categories.json",
                "resource_id_sets.json",
                "audio_resources.json",
                "ui_resources.json",
                "resource_templates.json"
            ]
        );
        assert!(bundle.decoder_asset_sources.missing_asset_names().is_empty());
    }

    #[test]
    fn test_load_cache_rules_bundle_from_path_tolerates_malformed_optional_sibling_asset() {
        let dir = tmp_dir("loader-rules-bundle-malformed-optional");
        let rules_path = dir.join(CACHE_RULES_FILE);
        let audio_path = dir.join(AUDIO_RESOURCES_FILE);
        write_rules_file(&rules_path);
        fs::write(&audio_path, "{ definitely not json").unwrap();

        let bundle = load_cache_rules_bundle_from_path(&rules_path).unwrap();

        assert_eq!(bundle.decoder_asset_sources.audio_resources, DecoderAssetSource::Missing);
        assert!(bundle.decoder_assets.audio_resources.is_none());
    }

    #[test]
    fn test_load_cache_rules_bundle_from_path_rejects_malformed_required_rules_asset() {
        let dir = tmp_dir("loader-rules-bundle-malformed-required");
        let rules_path = dir.join(CACHE_RULES_FILE);
        fs::write(&rules_path, "{ definitely not json").unwrap();

        let err = load_cache_rules_bundle_from_path(&rules_path).unwrap_err();

        assert!(format!("{err}").contains("Failed to parse cache rules asset"));
    }
}
