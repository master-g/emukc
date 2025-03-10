use std::collections::BTreeMap;

use emukc_cache::{Kache, NoVersion};
use tokio::io::AsyncReadExt;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

mod img;

pub(super) async fn make(cache: &Kache, list: &mut CacheList) -> Result<(), CacheListMakingError> {
	let mut version_file = cache.get("kcs2/version.json", NoVersion).await?;
	let mut raw: String = String::new();
	version_file.read_to_string(&mut raw).await?;
	let version_info: BTreeMap<String, String> = serde_json::from_str(&raw)?;

	img::make(&version_info, list).await?;

	Ok(())
}
