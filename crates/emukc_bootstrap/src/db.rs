//! Bootstrap database

use emukc_db::entity::bootstrap;
use emukc_db::sea_orm::{self, Database, DbConn};
use thiserror::Error;

/// Database bootstrap error
#[derive(Error, Debug)]
pub enum DbBootstrapError {
	/// Invalid path
	#[error("Invalid path: {0}")]
	InvalidPath(String),

	/// IO error
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Database error
	#[error("Database error: {0}")]
	Db(#[from] sea_orm::error::DbErr),
}

/// Prepare the database
///
/// # Arguments
///
/// * `path` - The path to the database
/// * `overwrite` - Whether to overwrite the database
///
/// # Returns
///
/// A `DatabaseConnection` object
pub async fn prepare(
	path: impl AsRef<std::path::Path>,
	overwrite: bool,
) -> Result<DbConn, DbBootstrapError> {
	let path = path.as_ref();
	if path.is_dir() {
		return Err(DbBootstrapError::InvalidPath(path.to_string_lossy().to_string()));
	}
	if path.exists() && overwrite {
		std::fs::remove_file(path)?;
	}

	let sqlite_url = format!("sqlite:{}?mode=rwc", path.to_str().unwrap());
	let db = Database::connect(&sqlite_url).await?;
	bootstrap(&db).await?;

	Ok(db)
}
