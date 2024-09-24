//! Application state

use std::{fs::create_dir, sync::Arc};

use anyhow::bail;
use emukc_internal::{
	cache::kache,
	db::sea_orm::DbConn,
	prelude::{prepare, Codex, CodexArc, Kache},
};

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

impl State {
	/// Create a new application state
	///
	/// # Parameters
	///
	/// - `cfg` - Application configuration
	pub async fn new(cfg: &AppConfig) -> anyhow::Result<Self> {
		// ensure workspace root
		if !cfg.workspace_root.exists() {
			create_dir(&cfg.workspace_root)?;
		} else if cfg.workspace_root.is_file() {
			bail!(cfg.workspace_root.to_string_lossy().to_string());
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
