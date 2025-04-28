//! Download requests and responses.

use std::{
	fs::{OpenOptions, create_dir_all},
	io::Cursor,
	path::PathBuf,
};

use emukc_crypto::SimpleHash;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::client::new_reqwest_client;

/// Download request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
	/// URL to download
	pub url: String,

	/// Save as
	pub save_as: PathBuf,

	/// If the file already exists, overwrite it
	pub overwrite: bool,

	/// Skip header check
	pub skip_header_check: bool,
}

/// Download request builder
#[derive(Debug, Clone, Default)]
pub struct Builder {
	url: Option<String>,
	save_as: Option<PathBuf>,
	overwrite: bool,
	skip_header_check: bool,
}

/// Errors that can occur when building a request
#[derive(Debug, Error)]
pub enum BuilderError {
	/// URL is required
	#[error("URL is required")]
	UrlRequired,

	/// Save as is required
	#[error("Save as is required")]
	SaveAsRequired,
}

impl Builder {
	/// Set URL
	pub fn url(mut self, url: impl Into<String>) -> Self {
		self.url = Some(url.into());
		self
	}

	/// Set save as
	pub fn save_as(mut self, save_as: impl Into<PathBuf>) -> Self {
		self.save_as = Some(save_as.into());
		self
	}

	/// Set overwrite
	pub fn overwrite(mut self, overwrite: bool) -> Self {
		self.overwrite = overwrite;
		self
	}

	/// Set skip header check
	pub fn skip_header_check(mut self, skip_header_check: bool) -> Self {
		self.skip_header_check = skip_header_check;
		self
	}

	/// Build the request
	pub fn build(self) -> Result<Request, BuilderError> {
		let url = self.url.ok_or(BuilderError::UrlRequired)?;
		let save_as = self.save_as.ok_or(BuilderError::SaveAsRequired)?;
		Ok(Request {
			url,
			save_as,
			overwrite: self.overwrite,
			skip_header_check: self.skip_header_check,
		})
	}
}

/// Errors that can occur when downloading
#[derive(Debug, Error)]
pub enum DownloadError {
	/// Reqwest error
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),

	/// IO error
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Header check failed
	#[error("Header check failed with code: {0}")]
	HeaderCheckFailed(http::StatusCode),

	/// Response error
	#[error("Response error: {0}")]
	ResponseError(http::StatusCode),

	/// File already exists
	#[error("File already exists: {0}")]
	FileAlreadyExists(String),

	/// File not found
	#[error("File not found: {0}")]
	FileNotFound(String),
}

impl Request {
	/// Create a new builder
	pub fn builder() -> Builder {
		Builder::default()
	}

	/// Execute the download
	pub async fn execute(self, client: Option<reqwest::Client>) -> Result<(), DownloadError> {
		let client = match client {
			Some(client) => client,
			None => {
				trace!("using default reqwest client");
				new_reqwest_client(None, None)?
			}
		};

		if !self.skip_header_check {
			trace!("checking if the file exists via a HEAD request");
			// check if the file exists via a HEAD request
			let head = client.head(&self.url).send().await?;
			if !head.status().is_success() {
				error!("HEAD request failed with status code: {}", head.status());
				return Err(DownloadError::HeaderCheckFailed(head.status()));
			}
		}

		// send
		let response = client
			.get(&self.url)
			.header("Cache-Control", "no-cache, no-store, must-revalidate")
			.header("Pragma", "no-cache")
			.header("Expires", "0")
			.send()
			.await?;
		if !response.status().is_success() {
			error!("GET request failed with status code: {}", response.status());
			if response.status() == http::StatusCode::NOT_FOUND {
				return Err(DownloadError::FileNotFound(self.url.clone()));
			}
			return Err(DownloadError::ResponseError(response.status()));
		}

		// get save path
		let save_as = {
			let save_as = self.save_as;
			if save_as.is_dir() {
				// pre calculate the file name hash
				let url_hash = self.url.simple_hash();
				let fname = response
					.url()
					.path_segments()
					.and_then(std::iter::Iterator::last)
					.unwrap_or(&url_hash);
				// join
				let fname = std::path::Path::new(fname);
				save_as.join(fname)
			} else if save_as.exists() && !self.overwrite {
				let save_as = save_as.display().to_string();
				return Err(DownloadError::FileAlreadyExists(save_as));
			} else {
				save_as
			}
		};

		// create parent directory
		let parent = save_as.parent().expect("cannot get parent directory of the save path");
		create_dir_all(parent)?;

		// open
		let mut dest = OpenOptions::new().write(true).create(true).truncate(true).open(&save_as)?;

		// write
		let body = response.bytes().await?;
		let mut content = Cursor::new(body);
		std::io::copy(&mut content, &mut dest)?;

		Ok(())
	}
}
