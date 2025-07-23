use std::{
	path,
	sync::{Arc, atomic::AtomicUsize},
};

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::io::{AsyncBufReadExt, BufReader};

use emukc_cache::{GetOption, Kache, KacheError};

use crate::make_list::CacheListItem;

const MAX_CONCURRENT: usize = 32;

async fn count_lines(path: impl AsRef<path::Path>) -> Result<usize, KacheError> {
	let file = tokio::fs::File::open(path).await?;
	let reader = BufReader::new(file);
	let mut lines = reader.lines();
	let mut count = 0;

	while lines.next_line().await?.is_some() {
		count += 1;
	}

	Ok(count)
}

fn print_progress(completed: usize, total: usize) {
	let percentage = if total > 0 {
		(completed as f64 / total as f64 * 100.0) as usize
	} else {
		0
	};

	print!("\r[{completed}/{total}][{percentage}%]");
	std::io::Write::flush(&mut std::io::stdout()).expect("Could not flush stdout");
}

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
) -> Result<(), KacheError> {
	let total_files = count_lines(&path_to_list).await?;
	let completed = Arc::new(AtomicUsize::new(0));

	let file = tokio::fs::File::open(path_to_list).await?;
	let reader = BufReader::new(file);
	let mut lines = reader.lines();

	let q = concurrent.clamp(1, MAX_CONCURRENT);

	let mut tasks = FuturesUnordered::new();

	print_progress(0, total_files);

	loop {
		while tasks.len() < q {
			match lines.next_line().await? {
				Some(line) => {
					let item: CacheListItem = serde_json::from_str(&line)
						.map_err(|e| KacheError::InvalidFile(e.to_string()))?;
					let opt = GetOption::default().disable_mod();
					let kache_clone = kache.clone();
					let item_path = item.path.clone();
					let item_version = item.version.clone();
					let completed_clone = completed.clone();

					tasks.push(async move {
						let result = opt.get(&kache_clone, &item_path, item_version).await;
						let count =
							completed_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
						print_progress(count, total_files);

						result
					});
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

	println!();
	Ok(())
}
