use emukc_cache::Kache;
use emukc_model::codex::Codex;

use super::{CacheList, CacheListMakeStrategy, errors::CacheListMakingError};

mod gadget_html5;
mod kcs;
mod kcs2;

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
	kcs::make(codex, kache, strategy, list).await?;
	kcs2::make(&codex.manifest, kache, strategy, list).await?;

	Ok(())
}
