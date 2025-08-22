//! Application state

use std::{fs::create_dir, sync::Arc};

use anyhow::bail;
use emukc_internal::{
	db::sea_orm::DbConn,
	prelude::{Codex, HasContext, Kache, prepare},
};

use crate::cfg::AppConfig;

const DB_NAME: &str = "emukc.db";

/// Application state
#[derive(Debug, Clone)]
pub struct State {
	/// Database connection
	pub db: Arc<DbConn>,

	/// kache
	pub kache: Arc<Kache>,

	/// Codex instance
	pub codex: Arc<Codex>,
}

impl State {
	/// Create a new application state
	///
	/// # Parameters
	///
	/// - `cfg` - Application configuration
	/// - `load_cache_source` - Whether to load cache source
	pub async fn new(cfg: &AppConfig, load_cache_source: bool) -> anyhow::Result<Self> {
		// ensure workspace root
		if !cfg.workspace_root.exists() {
			create_dir(&cfg.workspace_root)?;
		} else if cfg.workspace_root.is_file() {
			bail!(cfg.workspace_root.to_string_lossy().to_string());
		}

		// prepare database
		let db_path = cfg.workspace_root.join(DB_NAME);
		let db = Arc::new(prepare(&db_path, false).await?);

		let kache = Kache::builder()
			.with_cache_root(cfg.cache_root.clone())
			.with_mods_root(cfg.mods_root.clone())
			.with_gadgets_cdns(cfg.gadgets_cdn.clone())
			.with_content_cdns(cfg.game_cdn.clone())
			.with_proxy(cfg.proxy.to_owned())
			.build()?;

		// kache system
		let kache = Arc::new(kache);

		// codex
		let codex_root = cfg.codex_root()?;
		let codex = Arc::new(Codex::load(&codex_root, load_cache_source)?);

		Ok(Self {
			db,
			kache,
			codex,
		})
	}
}

pub type StateArc = Arc<State>;

impl HasContext for State {
	fn db(&self) -> &DbConn {
		self.db.as_ref()
	}

	fn codex(&self) -> &Codex {
		self.codex.as_ref()
	}
}
