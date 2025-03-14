use std::sync::Arc;

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::io::{AsyncBufReadExt, BufReader};

use emukc_cache::{GetOption, Kache, KacheError};

use crate::make_list::CacheListItem;

const MAX_CONCURRENT: usize = 32;

/// Populate the cache with the list file.
///
/// # Arguments
///
/// * `kache` - The kache instance.
/// * `path_to_list` - The path to the list file.
/// * `concurrent` - The number of concurrent downloads.
/// * `skip_checksum` - Whether to skip checksum verification.
pub async fn populate(
	kache: Arc<Kache>,
	path_to_list: impl AsRef<std::path::Path>,
	concurrent: usize,
	skip_checksum: bool,
) -> Result<(), KacheError> {
	let file = tokio::fs::File::open(path_to_list).await?;
	let reader = BufReader::new(file);
	let mut lines = reader.lines();

	let q = concurrent.clamp(1, MAX_CONCURRENT);

	let mut tasks = FuturesUnordered::new();

	loop {
		while tasks.len() < q {
			match lines.next_line().await? {
				Some(line) => {
					let item: CacheListItem = serde_json::from_str(&line)
						.map_err(|e| KacheError::InvalidFile(e.to_string()))?;
					let mut opt = GetOption::default().disable_mod();
					if skip_checksum {
						opt = opt.disable_checksum();
					};

					let kache_clone = kache.clone();
					let item_path = item.path.clone();
					let item_version = item.version.clone();

					tasks
						.push(async move { opt.get(&kache_clone, &item_path, item_version).await });
				}
				None => break,
			}
		}

		if tasks.is_empty() {
			break;
		}

		match tasks.next().await {
			Some(result) => {
				result?;
			}
			None => {
				break;
			}
		}
	}

	Ok(())
}
