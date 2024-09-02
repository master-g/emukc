use std::{collections::HashMap, path::Path};

use emukc_cache::kache;
use emukc_crypto::md5_file;
use emukc_model::cache::KcFileEntry;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportKccpCacheError {
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	#[error("JSON error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("Kache error: {0}")]
	Kache(#[from] kache::Error),
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

	let mut entries: Vec<KcFileEntry> = Vec::new();
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

		let md5 = md5_file(&full_path)?;
		entries.push(KcFileEntry::new(key, &md5, version));
	}

	info!("{} entries to import", entries.len());

	let (inserted, updated) = kache.import(&entries).await?;

	info!("{} entries imported, {} entries updated", inserted, updated);

	Ok(())
}
