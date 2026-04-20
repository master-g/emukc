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
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    gadget_html5::make(&codex.manifest, kache, list).await?;

    if strategy == CacheListMakeStrategy::Manifest {
        make_manifest(&codex.manifest, list)?;
    } else {
        kcs::make(codex, kache, strategy.clone(), list).await?;
        kcs2::make(&codex.manifest, kache, strategy, list).await?;
    }

    Ok(())
}

fn make_manifest(
    mst: &emukc_model::kc2::start2::ApiManifest,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    let manifest_data = manifest::load_resource_manifest()?;

    info!(
        "Loaded resource manifest: {} entries (v{})",
        manifest_data.entries.len(),
        manifest_data.version
    );

    for entry in &manifest_data.entries {
        manifest::generate::generate_entry_paths(entry, mst, list);
    }

    Ok(())
}
