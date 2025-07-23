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

	/// File version not matched error.
	#[error("file version not matched: {0}")]
	InvalidFileVersion(String),

	/// IO error.
	#[error(transparent)]
	Io(#[from] std::io::Error),

	/// Database error.
	#[error(transparent)]
	Db(#[from] redb::DatabaseError),

	/// Database transaction error.
	#[error(transparent)]
	DbTxn(#[from] redb::TransactionError),

	/// Database table error.
	#[error(transparent)]
	DbTable(#[from] redb::TableError),

	/// Database table storage error.
	#[error(transparent)]
	DbStorage(#[from] redb::StorageError),

	/// Database commit error.
	#[error(transparent)]
	DbCommit(#[from] redb::CommitError),

	/// Download error.
	#[error(transparent)]
	DownloadRequestBuilder(#[from] download::BuilderError),

	/// Download error.
	#[error(transparent)]
	Download(#[from] download::DownloadError),

	/// Failed on all CDN.
	#[error("failed on all CDN")]
	FailedOnAllCdn,

	/// Reqwest error.
	#[error(transparent)]
	Reqwest(#[from] reqwest::Error),
}
