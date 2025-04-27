use std::{
	collections::{BTreeSet, HashMap},
	sync::Arc,
};

use futures::{StreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};

use emukc_cache::{IntoVersion, Kache, KacheError};
use emukc_model::kc2::start2::ApiManifest;

use errors::CacheListMakingError;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

pub mod errors;

mod source;

/// Strategy for making a cache list.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CacheListMakeStrategy {
	/// Default strategy
	Default,
	/// Minimal strategy
	Minimal,
	/// Greedy strategy
	Greedy(usize),
}

#[derive(Debug, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CacheListItem {
	#[serde(rename = "_id")]
	pub id: i64,
	pub path: String,
	#[serde(skip_serializing_if = "Option::is_none")]
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
	strategy: CacheListMakeStrategy,
	overwrite: bool,
) -> Result<(), CacheListMakingError> {
	let out = outpath.as_ref().to_owned();
	if !overwrite && out.exists() {
		return Err(CacheListMakingError::FileExists(out));
	}

	info!("making cache list to {:?}", out);

	let mut list = CacheList::new();

	source::make(mst, kache, strategy, &mut list).await?;

	for item in list.items.iter() {
		let line = serde_json::to_string(item)?;
		debug!("{}", line);
	}

	let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&out).await?;
	for item in list.items.iter() {
		let line = serde_json::to_string(item)?;
		file.write_all(line.as_bytes()).await?;
		file.write_u8(b'\n').await?;
	}

	file.sync_all().await?;

	info!("cache list made to {:?}", out);

	Ok(())
}

const MAX_CHECK_SIZE: usize = 32;

/// Check if a list of URLs exist on the remote cache.
///
/// # Arguments
///
/// * `cache` - The remote cache to check against.
/// * `urls` - The list of URLs to check.
/// * `concurrent` - The maximum number of concurrent checks.
///
/// # Returns
///
/// A `HashMap` mapping each URL to a boolean indicating whether it exists on the remote cache.
pub async fn batch_check_exists(
	cache: Arc<Kache>,
	mut urls: Vec<(String, String)>,
	concurrent: usize,
) -> Result<HashMap<(String, String), bool>, KacheError> {
	let q = concurrent.clamp(1, MAX_CHECK_SIZE);
	let mut result: HashMap<(String, String), bool> = HashMap::new();
	let mut tasks = FuturesUnordered::new();

	loop {
		while tasks.len() < q {
			match urls.pop() {
				Some((url, ver)) => {
					let c = cache.clone();
					let key = url.clone();
					tasks.push(async move {
						let exists = c.exists_on_remote(&key, &ver).await?;
						Ok::<((String, String), bool), KacheError>(((key, ver), exists))
					});
				}
				None => {
					break;
				}
			}
		}

		if tasks.is_empty() {
			break;
		}

		match tasks.next().await {
			Some(Ok(((key, ver), exists))) => {
				result.insert((key, ver), exists);
			}
			Some(Err(err)) => {
				return Err(err);
			}
			None => {
				break;
			}
		}
	}

	Ok(result)
}
