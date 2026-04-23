use emukc_cache::Kache;
use emukc_model::codex::Codex;

use super::{CacheList, CacheListMakeStrategy, errors::CacheListMakingError, manifest};

mod gadget_html5;
mod kcs;
pub(crate) mod kcs2;

/// Make a list of caches.
///
/// # Arguments
///
/// * `mst` - The API manifest.
/// * `kache` - The cache.
/// * `strategy` - The make strategy.
/// * `list` - The cache list.
///
/// # Returns
///
/// A result indicating success or failure.
pub(super) async fn make(
    codex: &Codex,
    kache: &Kache,
    strategy: CacheListMakeStrategy,
    manifest_override: Option<&manifest::ResourceManifest>,
    decoder_assets_override: Option<&manifest::DecoderCoverageAssets>,
    cache_rules_override: Option<&manifest::CacheRulesAsset>,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    gadget_html5::make(&codex.manifest, kache, list).await?;

    if strategy == CacheListMakeStrategy::Manifest {
        let owned_decoder_assets;
        let decoder_assets = if let Some(decoder_assets) = decoder_assets_override {
            decoder_assets
        } else {
            owned_decoder_assets = manifest::load_decoder_coverage_assets()?;
            &owned_decoder_assets
        };
        make_manifest(&codex.manifest, manifest_override, decoder_assets_override, list)?;
        kcs::make(codex, kache, CacheListMakeStrategy::Manifest, list).await?;
        let categories = decoder_assets.resource_categories.as_ref();
        kcs2::make_manifest_support(
            &codex.manifest,
            kache,
            list,
            Some(decoder_assets),
            categories,
            manifest::path_rules(),
        )
        .await?;
    } else if strategy == CacheListMakeStrategy::Rules {
        let owned_cache_rules;
        let cache_rules = if let Some(cache_rules) = cache_rules_override {
            cache_rules
        } else {
            owned_cache_rules = manifest::load_cache_rules()?;
            &owned_cache_rules
        };
        make_cache_rules(&codex.manifest, cache_rules, list)?;
        kcs::make(codex, kache, CacheListMakeStrategy::Rules, list).await?;
        kcs2::make_manifest_support(
            &codex.manifest,
            kache,
            list,
            None,
            Some(&cache_rules.resource_categories),
            cache_rules.resource_manifest.path_rules.as_ref(),
        )
        .await?;
    } else {
        if matches!(strategy, CacheListMakeStrategy::Default | CacheListMakeStrategy::Greedy(_)) {
            if let Some(manifest_data) = manifest_override {
                manifest::populate_path_rules_locks(manifest_data);
            } else {
                match manifest::load_resource_manifest() {
                    Ok(manifest_data) => manifest::populate_path_rules_locks(&manifest_data),
                    Err(err) => {
                        warn!("Failed to load resource manifest for pathRules fallback: {err}");
                    }
                }
            }
        }
        kcs::make(codex, kache, strategy.clone(), list).await?;
        kcs2::make(&codex.manifest, kache, strategy, list).await?;
    }

    Ok(())
}

fn make_cache_rules(
    mst: &emukc_model::kc2::start2::ApiManifest,
    cache_rules: &manifest::CacheRulesAsset,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    manifest::populate_path_rules_locks(&cache_rules.resource_manifest);

    info!(
        "Loaded cache rules: ship rules {}, slot rules {}",
        cache_rules.summary.ship_rule_count, cache_rules.summary.slot_rule_count
    );

    for entry in &cache_rules.resource_manifest.entries {
        manifest::generate::generate_entry_paths(
            entry,
            mst,
            cache_rules.resource_manifest.path_rules.as_ref(),
            None,
            Some(cache_rules),
            list,
        );
    }

    Ok(())
}

fn make_manifest(
    mst: &emukc_model::kc2::start2::ApiManifest,
    manifest_override: Option<&manifest::ResourceManifest>,
    decoder_assets_override: Option<&manifest::DecoderCoverageAssets>,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    let owned_manifest;
    let owned_decoder_assets;
    let manifest_data = if let Some(manifest_data) = manifest_override {
        manifest_data
    } else {
        owned_manifest = manifest::load_resource_manifest()?;
        &owned_manifest
    };
    let decoder_assets = if let Some(decoder_assets) = decoder_assets_override {
        decoder_assets
    } else {
        owned_decoder_assets = manifest::load_decoder_coverage_assets()?;
        &owned_decoder_assets
    };
    manifest::populate_path_rules_locks(manifest_data);

    info!(
        "Loaded resource manifest: {} entries (v{})",
        manifest_data.entries.len(),
        manifest_data.version
    );

    for entry in &manifest_data.entries {
        manifest::generate::generate_entry_paths(
            entry,
            mst,
            manifest_data.path_rules.as_ref(),
            Some(decoder_assets),
            None,
            list,
        );
    }

    Ok(())
}
