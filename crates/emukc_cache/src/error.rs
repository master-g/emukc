use emukc_db::sea_orm::DbErr;
use emukc_network::{download, reqwest};

/// Error type for `Kache`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Missing field error.
	#[error("missing field: {0}")]
	MissingField(String),

	/// File not found error.
	#[error("file not found: {0}")]
	FileNotFound(String),

	/// Invalid file error.
	#[error("invalid file: {0}")]
	InvalidFile(String),

	/// File expired error.
	#[error("file expired: {0}")]
	FileExpired(String),

	/// IO error.
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Database error.
	#[error("database error: {0}")]
	Db(#[from] DbErr),

	/// Download error.
	#[error("download request builder error: {0}")]
	DownloadRequestBuilder(#[from] download::BuilderError),

	/// Download error.
	#[error("download error: {0}")]
	Download(#[from] download::DownloadError),

	/// Failed on all CDN.
	#[error("failed on all CDN")]
	FailedOnAllCdn,

	/// Reqwest error.
	#[error("reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
}
