//! Possible errors that can occur during cache list making.

use thiserror::Error;

use emukc_cache::KacheError;

/// Possible errors that can occur during cache list making.
#[derive(Debug, Error)]
pub enum CacheListMakingError {
	/// IO error
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	/// JSON error
	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),

	/// Kache error
	#[error("kache error: {0}")]
	Kache(#[from] KacheError),

	/// File already exists
	#[error("file already exists: {0}")]
	FileExists(std::path::PathBuf),
}
