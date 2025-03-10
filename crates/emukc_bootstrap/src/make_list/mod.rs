use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use emukc_cache::{IntoVersion, Kache};
use emukc_model::kc2::start2::ApiManifest;

use errors::CacheListMakingError;

pub mod errors;

mod source;

#[derive(Debug, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CacheListItem {
	#[serde(rename = "_id")]
	pub id: i64,
	pub path: String,
	pub version: Option<String>,
}

#[derive(Debug)]
pub(crate) struct CacheList {
	pub items: BTreeSet<CacheListItem>,

	next_id: i64,
}

impl CacheList {
	pub fn new() -> Self {
		Self {
			items: BTreeSet::new(),
			next_id: 0,
		}
	}

	pub fn add(&mut self, path: String, version: impl IntoVersion) -> &mut Self {
		let version = version.into_version();
		let item = CacheListItem {
			id: self.next_id,
			path,
			version,
		};
		self.items.insert(item);
		self.next_id += 1;

		self
	}

	pub fn add_unversioned(&mut self, path: String) -> &mut Self {
		let item = CacheListItem {
			id: self.next_id,
			path,
			version: None,
		};
		self.items.insert(item);
		self.next_id += 1;

		self
	}
}

/// Make a cache list.
///
/// # Arguments
///
/// * `mst` - The API manifest.
/// * `kache` - The kache instance.
/// * `outpath` - The output path.
/// * `overwrite` - Whether to overwrite the output file if it already exists.
///
/// # Returns
///
/// A `Result` containing either `Ok(())` if the cache list was successfully made, or an error if it failed.
pub async fn make(
	mst: &ApiManifest,
	kache: &Kache,
	outpath: impl AsRef<std::path::Path>,
	overwrite: bool,
) -> Result<(), CacheListMakingError> {
	let out = outpath.as_ref().to_owned();
	if !overwrite && out.exists() {
		return Err(CacheListMakingError::FileExists(out));
	}

	let mut list = CacheList::new();

	source::make(mst, kache, &mut list).await?;

	for item in list.items.iter() {
		let line = serde_json::to_string(item)?;
		info!("{}", line);
	}

	Ok(())
}
