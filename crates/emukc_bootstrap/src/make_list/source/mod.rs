use emukc_cache::Kache;
use emukc_model::codex::Codex;

use super::{
    CacheList, CacheListAuthorityStage, CacheListMakeStrategy, errors::CacheListMakingError,
    manifest,
};

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
    rules_bundle_override: Option<&manifest::DecoderRulesBundle>,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    if strategy == CacheListMakeStrategy::Rules {
        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
        gadget_html5::make(&codex.manifest, kache, list).await?;
        list.set_authority_stage(previous);
    } else {
        gadget_html5::make(&codex.manifest, kache, list).await?;
    }

    if strategy == CacheListMakeStrategy::Manifest {
        let owned_decoder_assets;
        let decoder_assets = if let Some(decoder_assets) = decoder_assets_override {
            decoder_assets
        } else {
            owned_decoder_assets = manifest::load_decoder_coverage_assets()?;
            &owned_decoder_assets
        };
        make_manifest(&codex.manifest, manifest_override, decoder_assets_override, list)?;
        kcs::make(codex, kache, CacheListMakeStrategy::Manifest, None, list).await?;
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
        let owned_rules_bundle;
        let rules_bundle = if let Some(rules_bundle) = rules_bundle_override {
            rules_bundle
        } else {
            owned_rules_bundle = manifest::load_cache_rules_bundle()?;
            &owned_rules_bundle
        };
        list.record_unresolved_rule_blockers(rules_bundle.cache_rules.unresolved_rules.clone());
        list.record_repo_fallback_bundle_assets(
            rules_bundle
                .decoder_asset_sources
                .repo_fallback_asset_names()
                .into_iter()
                .map(str::to_string),
        );
        list.record_missing_bundle_assets(
            rules_bundle
                .decoder_asset_sources
                .missing_asset_names()
                .into_iter()
                .map(str::to_string),
        );

        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        make_cache_rules(&codex.manifest, rules_bundle, list)?;
        list.set_authority_stage(previous);

        kcs::make(codex, kache, CacheListMakeStrategy::Rules, Some(rules_bundle), list).await?;

        kcs2::make_manifest_support(
            &codex.manifest,
            kache,
            list,
            Some(&rules_bundle.decoder_assets),
            Some(&rules_bundle.cache_rules.resource_categories),
            rules_bundle.cache_rules.resource_manifest.path_rules.as_ref(),
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
        kcs::make(codex, kache, strategy.clone(), None, list).await?;
        kcs2::make(&codex.manifest, kache, strategy, list).await?;
    }

    Ok(())
}

fn make_cache_rules(
    mst: &emukc_model::kc2::start2::ApiManifest,
    rules_bundle: &manifest::DecoderRulesBundle,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    let cache_rules = &rules_bundle.cache_rules;
    manifest::populate_path_rules_locks(&cache_rules.resource_manifest);

    info!(
        "Loaded cache rules: ship rules {}, slot rules {}, sound rules {}",
        cache_rules.summary.ship_rule_count,
        cache_rules.summary.slot_rule_count,
        cache_rules.summary.sound_rule_count
    );
    let missing_assets = rules_bundle.decoder_asset_sources.missing_asset_names();
    if !missing_assets.is_empty() {
        warn!(
            "Decoder rules bundle is missing optional sibling assets: {}",
            missing_assets.join(", ")
        );
    }

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
