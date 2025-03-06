use emukc_cache::kache;
use emukc_model::kc2::start2::ApiManifest;

mod bgm;
mod furniture;
mod unversioned;

pub(super) async fn crawl(mst: &ApiManifest, cache: &kache::Kache) -> Result<(), kache::Error> {
	bgm::crawl(mst, cache).await?;
	furniture::crawl(mst, cache).await?;
	unversioned::crawl(cache).await?;

	Ok(())
}
