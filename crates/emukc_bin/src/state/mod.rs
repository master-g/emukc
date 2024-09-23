//! Application state

use std::{fs::create_dir, sync::Arc};

use emukc_internal::{
	cache::kache,
	db::sea_orm::{self, DbConn},
	model::codex::CodexError,
	prelude::{prepare, Codex, CodexArc, DbBootstrapError, Kache},
};
use thiserror::Error;

use crate::cfg::AppConfig;

const DB_NAME: &str = "emukc.db";

/// Application state
#[derive(Debug, Clone)]
pub struct State {
	/// Database connection
	pub db: Arc<DbConn>,

	/// kache
	pub kache: Arc<kache::Kache>,

	/// Codex instance
	pub codex: CodexArc,
}

#[derive(Error, Debug)]
pub enum StateError {
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	#[error("Database error: {0}")]
	Db(#[from] sea_orm::error::DbErr),

	#[error("Invalid workspace root: {0}")]
	InvalidWorkspaceRoot(String),

	#[error("Bootstrap error: {0}")]
	DbBootstrap(#[from] DbBootstrapError),

	#[error("Kache error: {0}")]
	Kache(#[from] kache::Error),

	#[error("Codex error: {0}")]
	Codex(#[from] CodexError),
}

impl State {
	/// Create a new application state
	///
	/// # Parameters
	///
	/// - `cfg` - Application configuration
	pub async fn new(cfg: &AppConfig) -> Result<Self, StateError> {
		// ensure workspace root
		if !cfg.workspace_root.exists() {
			create_dir(&cfg.workspace_root)?;
		} else if cfg.workspace_root.is_file() {
			return Err(StateError::InvalidWorkspaceRoot(
				cfg.workspace_root.to_string_lossy().to_string(),
			));
		}

		// prepare database
		let db_path = cfg.workspace_root.join(DB_NAME);
		let db = Arc::new(prepare(&db_path, false).await?);

		let kache_builder = Kache::builder()
			.with_cache_root(cfg.cache_root.clone())
			.with_db(db.clone())
			.with_gadgets_cdns(cfg.gadgets_cdn.clone())
			.with_content_cdns(cfg.game_cdn.clone());
		let kache_builder = if let Some(proxy) = &cfg.proxy {
			kache_builder.with_proxy(proxy.clone())
		} else {
			kache_builder
		};

		// kache system
		let kache = Arc::new(kache_builder.build()?);

		// codex
		let codex_root = cfg.codex_root()?;
		let codex = CodexArc::new(Codex::load(&codex_root)?);

		Ok(Self {
			db,
			kache,
			codex,
		})
	}
}

pub type StateArc = Arc<State>;
