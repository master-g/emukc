use std::{collections::HashMap, path::Path, sync::Arc};

use emukc_cache::kache;
use emukc_crypto::md5_file_async;
use emukc_model::cache::KcFileEntry;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;

const SEMAPHORE_LIMIT: usize = 128;

#[derive(Debug, Error)]
pub enum ImportKccpCacheError {
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	#[error("JSON error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("Kache error: {0}")]
	Kache(#[from] kache::Error),

	#[error("Tokio error: {0}")]
	Tokio(#[from] tokio::task::JoinError),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct KccpCacheEntry {
	version: Option<String>,
}

/// import `KCCP` cache from a JSON file.
///
/// # Arguments
///
/// * `kache` - The `Kache` instance.
/// * `path_to_json` - The path to the JSON file.
/// * `path_to_cache_root` - The path to the cache root.
#[instrument(skip_all)]
pub async fn import_kccp_cache(
	kache: &kache::Kache,
	path_to_json: impl AsRef<Path>,
	path_to_cache_root: Option<impl AsRef<Path>>,
) -> Result<(), ImportKccpCacheError> {
	let raw = std::fs::read_to_string(path_to_json.as_ref())?;
	let records = serde_json::from_str::<HashMap<String, KccpCacheEntry>>(&raw)?;

	let cache_root = if let Some(p) = path_to_cache_root {
		p.as_ref().to_path_buf()
	} else {
		path_to_json.as_ref().parent().unwrap().to_path_buf()
	};

	info!("importing {} records, cache root: {:?}", records.len(), cache_root);

	trace!("filtering out invalid entries...");
	let mut now = std::time::SystemTime::now();

	let mut tasks: Vec<(String, std::path::PathBuf, Option<String>)> = Vec::new();
	for (key, value) in records.iter() {
		let version = if let Some(v) = &value.version {
			v.split('=').last()
		} else {
			None
		};

		let key = key.trim_start_matches('/');

		let full_path = cache_root.join(key);
		if !full_path.exists() || !full_path.is_file() {
			warn!("file not found or not a file: {:?}", full_path);
			continue;
		}

		tasks.push((key.to_owned(), full_path, version.map(std::string::ToString::to_string)));
	}
	trace!("filtering done, elapsed: {:?}", now.elapsed().unwrap());

	trace!("calucating md5 for each file, this might take a while...");
	now = std::time::SystemTime::now();

	let semaphore = Arc::new(Semaphore::new(SEMAPHORE_LIMIT));
	let mut handles = vec![];

	for task in tasks {
		// let key = task.0.to_string();
		// let full_path = task.1.clone();
		// let version = task.2.map(|s| s.to_string());

		let permit = semaphore.clone().acquire_owned().await.unwrap();
		handles.push(tokio::spawn(async move {
			let result = md5_file_async(&task.1).await;
			drop(permit);
			result.map(|md5| KcFileEntry::new(task.0.as_str(), md5.as_str(), task.2.as_deref()))
		}));
	}

	let mut entries = vec![];
	for handle in handles {
		entries.push(handle.await??);
	}

	trace!("md5 calculation done, elapsed: {:?}", now.elapsed().unwrap());

	info!("{} entries to import", entries.len());
	now = std::time::SystemTime::now();

	let (inserted, updated) = kache.import(&entries).await?;

	info!(
		"{} entries imported, {} entries updated, elapsed: {:?}",
		inserted,
		updated,
		now.elapsed().unwrap()
	);

	Ok(())
}
