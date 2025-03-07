use emukc_cache::prelude::*;
use std::collections::BTreeMap;
use tokio::io::AsyncReadExt;

mod img;

pub(super) async fn crawl_kcs2_versioned(cache: &Kache) -> Result<(), KacheError> {
	let mut version_file = cache.get("kcs2/version.json", NoVersion).await?;
	let mut raw: String = String::new();
	version_file.read_to_string(&mut raw).await?;
	let version_info: BTreeMap<String, String> =
		serde_json::from_str(&raw).map_err(|err| KacheError::InvalidFile(err.to_string()))?;

	trace!("Crawling KCS2 versioned data, {:?}", version_info);

	img::crawl(cache, &version_info).await?;
	Ok(())
}
