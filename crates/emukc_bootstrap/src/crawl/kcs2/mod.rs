use emukc_cache::kache;
use emukc_model::kc2::start2::ApiManifest;

mod plain;
mod resources;
mod versioned;

pub(super) async fn crawl_kcs2(
	mst: &ApiManifest,
	cache: &kache::Kache,
) -> Result<(), kache::Error> {
	plain::crawl_kcs2_plain(cache).await?;
	resources::crawl(mst, cache).await?;
	versioned::crawl_kcs2_versioned(cache).await?;
	Ok(())
}
