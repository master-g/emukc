use emukc_cache::prelude::*;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

mod plain;
mod resources;
mod versioned;

pub(super) async fn crawl_kcs2(mst: &ApiManifest, cache: &Kache) -> Result<(), KacheError> {
	plain::crawl_kcs2_plain(cache).await?;
	resources::crawl(mst, cache).await?;
	versioned::crawl_kcs2_versioned(cache).await?;
	Ok(())
}

pub(crate) async fn fetch_res_impl<V>(
	cache: &Kache,
	id: i64,
	padding: i8,
	folder: &str,
	category: &str,
	extension: &str,
	ver: V,
) -> Result<tokio::fs::File, KacheError>
where
	V: IntoVersion + std::fmt::Debug,
{
	let id = if padding == 3 {
		format!("{0:03}", id)
	} else {
		format!("{0:04}", id)
	};

	let key = SuffixUtils::create(&id, format!("{folder}_{category}").as_str());

	cache
		.get(format!("kcs2/resources/{folder}/{category}/{id}_{key}.{extension}").as_str(), ver)
		.await
}
