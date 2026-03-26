//! Download the resources

use emukc_crypto::md5_file;
use emukc_network::{client::new_reqwest_client, download::DownloadError, reqwest};
use futures::{StreamExt, TryStreamExt, stream::FuturesUnordered};
use std::sync::Arc;
use thiserror::Error;

use crate::res::RES_LIST;

/// Error that can occur during the download process
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum BootstrapDownloadError {
	/// IO error
	#[error(transparent)]
	Io(#[from] std::io::Error),

	/// Request builder error
	#[error(transparent)]
	Builder(#[from] emukc_network::download::BuilderError),

	/// Reqwest client error
	#[error("failed to build bootstrap reqwest client with proxy {proxy:?}: {source}")]
	ReqwestClient {
		proxy: Option<String>,
		#[source]
		source: reqwest::Error,
	},

	/// Download error
	#[error(transparent)]
	Download(#[from] emukc_network::download::DownloadError),

	/// Unzip error
	#[error("bootstrap resource {save_as} ({url}) failed while {action}: {source}")]
	Unzip {
		url: String,
		save_as: String,
		action: &'static str,
		#[source]
		source: zip::result::ZipError,
	},

	/// Resource IO error
	#[error(
		"bootstrap resource {save_as} ({url}) failed while {action} at {}: {source}",
		path.display()
	)]
	ResourceIo {
		url: String,
		save_as: String,
		action: &'static str,
		path: std::path::PathBuf,
		#[source]
		source: std::io::Error,
	},
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

	debug!("proxy: {:?}", proxy);

	let client = Arc::new(new_reqwest_client(proxy, None).map_err(|source| {
		BootstrapDownloadError::ReqwestClient {
			proxy: proxy.map(ToOwned::to_owned),
			source,
		}
	})?);
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
			trace!("downloading {} from {}", res.save_as, res.url);

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

			let size = fullpath
				.metadata()
				.map_err(|source| BootstrapDownloadError::ResourceIo {
					url: res.url.to_owned(),
					save_as: res.save_as.to_owned(),
					action: "reading downloaded file metadata",
					path: fullpath.clone(),
					source,
				})?
				.len();
			let md5 = md5_file(&fullpath).map_err(|source| BootstrapDownloadError::ResourceIo {
				url: res.url.to_owned(),
				save_as: res.save_as.to_owned(),
				action: "hashing downloaded file",
				path: fullpath.clone(),
				source,
			})?;

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
					std::fs::create_dir_all(&unzip_to_path).map_err(|source| {
						BootstrapDownloadError::ResourceIo {
							url: res.url.to_owned(),
							save_as: res.save_as.to_owned(),
							action: "creating unzip target directory",
							path: unzip_to_path.clone(),
							source,
						}
					})?;
				}

				debug!("unzipping {} to {}", res.save_as, unzip_to);

				let file = std::fs::File::open(&fullpath).map_err(|source| {
					BootstrapDownloadError::ResourceIo {
						url: res.url.to_owned(),
						save_as: res.save_as.to_owned(),
						action: "opening downloaded zip file",
						path: fullpath.clone(),
						source,
					}
				})?;
				let mut archive =
					zip::ZipArchive::new(file).map_err(|source| BootstrapDownloadError::Unzip {
						url: res.url.to_owned(),
						save_as: res.save_as.to_owned(),
						action: "reading zip archive",
						source,
					})?;
				archive
					.extract_unwrapped_root_dir(unzip_to_path, zip::read::root_dir_common_filter)
					.map_err(|source| BootstrapDownloadError::Unzip {
						url: res.url.to_owned(),
						save_as: res.save_as.to_owned(),
						action: "extracting zip archive",
						source,
					})?;
				// zip_extract::extract(&file, &unzip_to_path, true)?;

				info!("{} unzipped to {}", res.save_as, unzip_to);
			}

			Ok(())
		};

		tasks.push(task);

		// Limit the number of concurrent downloads
		if tasks.len() >= max_concurrent
			&& let Some(result) = tasks.next().await
		{
			result?;
		}
	}

	// Process remaining tasks
	tasks.try_collect::<Vec<_>>().await?;

	Ok(())
}
