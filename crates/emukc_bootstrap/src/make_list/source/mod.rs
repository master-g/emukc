use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use super::{CacheList, errors::CacheListMakingError};

mod gadget_html5;
mod kcs2;

pub(super) async fn make(
	mst: &ApiManifest,
	kache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	gadget_html5::make(mst, kache, list).await?;
	kcs2::make(mst, kache, list).await?;

	Ok(())
}
