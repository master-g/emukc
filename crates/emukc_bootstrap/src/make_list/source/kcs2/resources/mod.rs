use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
	make_list::{CacheList, CacheListMakeStrategy},
	prelude::CacheListMakingError,
};

mod bgm;
mod furniture;
mod gauge;
mod map;
mod ship;
mod slot;
mod unversioned;

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	bgm::make(mst, list).await?;
	furniture::make(mst, cache, list).await?;
	gauge::make(cache, list).await?;
	map::make(cache, list).await?;
	ship::make(mst, cache, strategy, list).await?;
	slot::make(mst, cache, strategy, list).await?;
	unversioned::make(list).await?;

	Ok(())
}
