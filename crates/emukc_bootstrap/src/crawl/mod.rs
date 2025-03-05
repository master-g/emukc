use emukc_cache::kache;
use emukc_model::kc2::start2::ApiManifest;

mod gadget_html5;
mod kcs2;

/// Crawl the CDN with the given manifest and cache.
///
/// # Arguments
/// * `mst` - The manifest to use for finding what to crawl.
/// * `kache` - The cache to use for storing crawled data.
pub async fn crawl(_mst: &ApiManifest, kache: &kache::Kache) -> Result<(), kache::Error> {
	info!("Starting crawl");

	debug!("crawling gadgets");
	gadget_html5::crawl_gadget_html5(kache).await?;

	debug!("crawling kcs2");
	kcs2::crawl_kcs2(kache).await?;

	Ok(())
}
