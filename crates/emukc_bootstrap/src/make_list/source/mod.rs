use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use super::{CacheList, CacheListMakeStrategy, errors::CacheListMakingError};

mod gadget_html5;
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
	mst: &ApiManifest,
	kache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	gadget_html5::make(mst, kache, list).await?;
	kcs2::make(mst, kache, strategy, list).await?;

	Ok(())
}
