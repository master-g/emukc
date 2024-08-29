//! Kache is for `KanColle` Cache, a simple cache system.

use std::sync::Arc;

use emukc_db::sea_orm::DbConn;

/// The `Kache` struct is the main struct for the `KanColle` CDN file cache utilities.
#[derive(Debug, Clone)]
pub struct Kache {
	/// Root directory for the cache.
	cache_root: std::path::PathBuf,

	/// Root directory for the mods.
	mods_root: std::path::PathBuf,

	/// CDN URLs.
	cdn_list: Vec<String>,

	/// Proxy for downloading files.
	proxy: Option<String>,

	/// Database connection.
	db: Arc<DbConn>,
}
