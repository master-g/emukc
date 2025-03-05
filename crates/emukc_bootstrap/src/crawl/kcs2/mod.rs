use emukc_cache::kache;

mod plain;
mod versioned;

pub(super) async fn crawl_kcs2(cache: &kache::Kache) -> Result<(), kache::Error> {
	plain::crawl_kcs2_plain(cache).await?;
	versioned::crawl_kcs2_versioned(cache).await?;
	Ok(())
}
