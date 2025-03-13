use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

mod album;

pub(super) async fn make(
	mst: &ApiManifest,
	_cache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	album::make(mst, list);
	Ok(())
}
