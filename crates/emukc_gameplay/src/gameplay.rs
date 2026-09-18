//! A wrapper around the game's data and logic.

use std::sync::Arc;

use async_trait::async_trait;
use emukc_db::sea_orm::DbConn;
use emukc_model::codex::Codex;

use crate::game::{
    GameOps,
    sortie_store::{PracticeStore, SortieStore},
};

/// A trait for types that have a database connection and a codex.
pub trait HasContext: Send + Sync {
    /// Get the database connection.
    fn db(&self) -> &DbConn;

    /// Get the game's codex.
    fn codex(&self) -> &Codex;

    /// Get the sortie runtime store.
    fn sortie_store(&self) -> &SortieStore;

    /// Get the practice runtime store.
    fn practice_store(&self) -> &PracticeStore;
}

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
}

impl HasContext for Ctx {
    fn db(&self) -> &DbConn {
        &self.db
    }

    fn codex(&self) -> &Codex {
        &self.codex
    }

    fn sortie_store(&self) -> &SortieStore {
        &self.sortie_store
    }

    fn practice_store(&self) -> &PracticeStore {
        &self.practice_store
    }
}

/// Gameplay trait for the game's data and logic.
#[async_trait]
pub trait Gameplay: GameOps {}

/// Blanket implementation of `Gameplay` for types that implement `HasContext`.
#[async_trait]
impl<T: HasContext + ?Sized> Gameplay for T {}
