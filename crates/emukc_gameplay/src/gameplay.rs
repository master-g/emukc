//! A wrapper around the game's data and logic.

use std::sync::Arc;

use emukc_db::sea_orm::DbConn;
use emukc_model::codex::Codex;

use crate::game::sortie_store::{PracticeStore, SortieStore};

/// Everything a gameplay operation needs, as a concrete type.
///
/// Gameplay operations are inherent `async fn`s on `Ctx`. Owning types (the
/// server `State`, the integration-test context, the battle-sim context) embed a
/// `Ctx` and `Deref` to it, so `owner.some_op(..)` keeps working unchanged.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// Database connection.
    pub db: Arc<DbConn>,

    /// The game's codex.
    pub codex: Arc<Codex>,

    /// Sortie runtime store.
    pub sortie_store: Arc<SortieStore>,

    /// Practice runtime store.
    pub practice_store: Arc<PracticeStore>,
}

impl Ctx {
    /// Build a context around `db` and `codex` with fresh, isolated runtime
    /// stores.
    pub fn new(db: Arc<DbConn>, codex: Arc<Codex>) -> Self {
        Self {
            db,
            codex,
            sortie_store: Arc::new(SortieStore::new()),
            practice_store: Arc::new(PracticeStore::new()),
        }
    }

    /// Get the database connection.
    pub fn db(&self) -> &DbConn {
        &self.db
    }

    /// Get the game's codex.
    pub fn codex(&self) -> &Codex {
        &self.codex
    }

    /// Get the sortie runtime store.
    pub fn sortie_store(&self) -> &SortieStore {
        &self.sortie_store
    }

    /// Get the practice runtime store.
    pub fn practice_store(&self) -> &PracticeStore {
        &self.practice_store
    }
}
