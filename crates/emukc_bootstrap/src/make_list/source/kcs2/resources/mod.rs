use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

mod bgm;
mod furniture;
mod gauge;
mod map;
mod ship;
mod unversioned;

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	bgm::make(mst, list).await?;
	furniture::make(mst, cache, list).await?;
	gauge::make(cache, list).await?;
	map::make(cache, list).await?;
	ship::make(mst, list).await?;
	unversioned::make(list).await?;

	Ok(())
}
