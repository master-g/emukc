//! Application state

use std::{fs::create_dir, ops::Deref, sync::Arc};

use anyhow::bail;
use emukc_internal::prelude::{Codex, Ctx, Kache, prepare};

use crate::cfg::AppConfig;

mod payment_store;

pub use payment_store::{PaymentSession, PaymentStore};

const DB_NAME: &str = "emukc.db";

/// Application state
#[derive(Debug, Clone)]
pub struct State {
    /// Gameplay context: database, codex and the instance-scoped runtime stores
    pub ctx: Ctx,

    /// kache
    pub kache: Arc<Kache>,

    /// Payment session store (instance-scoped)
    pub payment_store: Arc<PaymentStore>,
}

impl Deref for State {
    type Target = Ctx;

    fn deref(&self) -> &Ctx {
        &self.ctx
    }
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
        let codex = Codex::load(&codex_root, load_cache_source)?;
        let codex = Arc::new(codex);

        Ok(Self {
            ctx: Ctx::new(db, codex),
            kache,
            payment_store: Arc::new(PaymentStore::new()),
        })
    }
}

pub type StateArc = Arc<State>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn codex_load_uses_generated_runtime_map_catalog() {
        let codex_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex");
        let codex = Codex::load_without_cache_source(codex_root).unwrap();

        let map_74 = codex.maps.map_definition(74).unwrap();
        let variant = map_74.variant("").unwrap();
        assert!(!variant.routing_rules.is_empty());
        assert!(variant.routing_rules.values().flatten().any(|rule| rule.to_cell_no > 0));
    }

    #[test]
    fn generated_runtime_map_catalog_keeps_world_1_1_at_four_cells() {
        let codex_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex");
        let codex = Codex::load_without_cache_source(codex_root).unwrap();

        let map_11 = codex.maps.map_definition(11).unwrap();
        let variant = map_11.variant("").unwrap();
        let cell_nos = variant.cells.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();
        assert_eq!(cell_nos, vec![0, 1, 2, 3]);
    }
}
