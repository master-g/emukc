//! Download the resources

use emukc_crypto::md5_file;
use emukc_network::{client::new_reqwest_client, download::DownloadError, reqwest};
use futures::{StreamExt, stream::FuturesUnordered};
use std::sync::Arc;
use thiserror::Error;

use crate::res::RES_LIST;

/// Error that can occur during the download process
#[derive(Debug, Error)]
pub enum BootstrapDownloadError {
	/// IO error
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Request builder error
	#[error("Request builder error: {0}")]
	Builder(#[from] emukc_network::download::BuilderError),

	/// Reqwest error
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),

	/// Download error
	#[error("Download error: {0}")]
	Download(#[from] emukc_network::download::DownloadError),

	/// Unzip error
	#[error("Unzip error: {0}")]
	Unzip(#[from] zip_extract::ZipExtractError),
}

/// Download all the resources
///
/// # Arguments
///
/// * `dir` - The output directory
/// * `overwrite` - Whether to overwrite existing files
/// * `proxy` - The proxy server
/// * `concurrent` - The maximum number of concurrent downloads, default is 4
pub async fn download_all(
	dir: impl AsRef<std::path::Path>,
	overwrite: bool,
	proxy: Option<&str>,
	concurrent: Option<usize>,
) -> Result<(), BootstrapDownloadError> {
	let output_dir = dir.as_ref();
	if !output_dir.exists() {
		std::fs::create_dir_all(output_dir)?;
	}

	let client = Arc::new(new_reqwest_client(proxy, None)?);
	let max_concurrent = concurrent.unwrap_or(4).max(1);
	let mut tasks = FuturesUnordered::new();

	for res in RES_LIST.iter() {
		let client = client.clone();
		let fullpath = output_dir.join(res.save_as);

		if fullpath.exists() && !overwrite {
			debug!("Skipping {:?} as it already exists", res);
			continue;
		}

		let output_dir = output_dir.to_path_buf();

		let task = async move {
			let since = std::time::Instant::now();
			let fullpath = output_dir.join(res.save_as);

			let result = emukc_network::download::Request::builder()
				.url(res.url)
				.save_as(&fullpath)
				.overwrite(overwrite)
				.skip_header_check(true)
				.build()?
				.execute(Some((*client).clone()))
				.await;

			match result {
				Err(DownloadError::FileAlreadyExists(f)) if !overwrite => {
					return Err(BootstrapDownloadError::Download(
						DownloadError::FileAlreadyExists(f),
					));
				}
				Err(e) => return Err(BootstrapDownloadError::Download(e)),
				Ok(_) => {}
			}

			let size = fullpath.metadata()?.len();
			let md5 = md5_file(&fullpath)?;

			info!(
				"{} downloaded, size: {}, md5: {}, time: {:?}",
				res.save_as,
				size,
				md5,
				since.elapsed()
			);

			if let Some(unzip_to) = res.unzip_to {
				let unzip_to_path = output_dir.join(unzip_to);
				if !unzip_to_path.exists() {
					std::fs::create_dir_all(&unzip_to_path)?;
				}

				debug!("unzipping {} to {}", res.save_as, unzip_to);

				let file = std::fs::File::open(&fullpath)?;
				zip_extract::extract(&file, &unzip_to_path, true)?;

				info!("{} unzipped to {}", res.save_as, unzip_to);
			}

			Ok(())
		};

		tasks.push(task);

		// Limit the number of concurrent downloads
		if tasks.len() >= max_concurrent {
			if let Some(result) = tasks.next().await {
				result?;
			}
		}
	}

	// Process remaining tasks
	while let Some(result) = tasks.next().await {
		result?;
	}

	Ok(())
}
