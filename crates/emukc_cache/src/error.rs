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
	Db(#[from] redb::DatabaseError),

	/// Database transaction error.
	#[error("database transaction error: {0}")]
	DbTxn(#[from] redb::TransactionError),

	/// Database table error.
	#[error("database table error: {0}")]
	DbTable(#[from] redb::TableError),

	/// Database table storage error.
	#[error("database table storage error: {0}")]
	DbStorage(#[from] redb::StorageError),

	/// Database commit error.
	#[error("database commit error: {0}")]
	DbCommit(#[from] redb::CommitError),

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
