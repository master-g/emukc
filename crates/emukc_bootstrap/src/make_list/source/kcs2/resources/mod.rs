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
mod use_item;

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	bgm::make(mst, strategy, list).await?;
	furniture::make(mst, cache, strategy, list).await?;
	gauge::make(cache, strategy, list).await?;
	map::make(cache, strategy, list).await?;
	ship::make(mst, cache, strategy, list).await?;
	slot::make(mst, cache, strategy, list).await?;
	unversioned::make(list).await?;
	use_item::make(mst, cache, strategy, list).await?;

	Ok(())
}
