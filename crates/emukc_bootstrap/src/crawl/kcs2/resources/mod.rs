use emukc_cache::prelude::*;
use emukc_model::kc2::start2::ApiManifest;

mod bgm;
mod furniture;
mod gauge;
// mod map;
mod unversioned;

pub(super) async fn crawl(mst: &ApiManifest, cache: &Kache) -> Result<(), KacheError> {
	bgm::crawl(mst, cache).await?;
	furniture::crawl(mst, cache).await?;
	gauge::crawl(cache).await?;
	// map::crawl(cache).await?;
	unversioned::crawl(cache).await?;

	Ok(())
}
