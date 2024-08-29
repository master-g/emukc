//! Bootstrap database

use emukc_db::entity::bootstrap;
use sea_orm::{Database, DatabaseConnection};

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
) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
	let path = path.as_ref();
	if path.is_dir() {
		return Err("Path is a directory".into());
	}
	if path.exists() && overwrite {
		std::fs::remove_file(path)?;
	}

	let sqlite_url = format!("sqlite:{}?mode=rwc", path.to_str().unwrap());
	let db = Database::connect(&sqlite_url).await?;
	bootstrap(&db).await?;

	Ok(db)
}
