use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
	make_list::{CacheList, CacheListMakeStrategy},
	prelude::CacheListMakingError,
};

mod kc9997;
mod kc9998;
mod kc9999;
mod purchase;
mod voice;

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	kc9997::make(cache, strategy, list).await?;
	kc9998::make(list);
	kc9999::make(cache, strategy, list).await?;
	purchase::make(mst, list);
	voice::make(mst, cache, strategy, list).await?;
	Ok(())
}
