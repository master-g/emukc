use tokio::io::{AsyncBufReadExt, BufReader};

use emukc_cache::{GetOption, Kache, KacheError};

use crate::make_list::CacheListItem;

/// Populate the cache with the list file.
///
/// # Arguments
///
/// * `kache` - The kache instance.
/// * `path_to_list` - The path to the list file.
/// * `skip_checksum` - Whether to skip checksum verification.
pub async fn populate(
	kache: &Kache,
	path_to_list: impl AsRef<std::path::Path>,
	skip_checksum: bool,
) -> Result<(), KacheError> {
	let file = tokio::fs::File::open(path_to_list).await?;
	let reader = BufReader::new(file);
	let mut lines = reader.lines();

	while let Some(line) = lines.next_line().await? {
		let item: CacheListItem =
			serde_json::from_str(&line).map_err(|e| KacheError::InvalidFile(e.to_string()))?;
		let opt = GetOption::default().disable_mod();
		if skip_checksum {
			opt.disable_version_check()
		} else {
			opt
		}
		.get(kache, &item.path, item.version)
		.await?;
	}

	Ok(())
}
