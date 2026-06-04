//! A wrapper around the game's data and logic.

use async_trait::async_trait;
use emukc_db::sea_orm::DbConn;
use emukc_model::codex::Codex;

use crate::{
    game::{
        GameOps,
        sortie_store::{GLOBAL_PRACTICE_STORE, GLOBAL_SORTIE_STORE, PracticeStore, SortieStore},
    },
    user::{AccountOps, ProfileOps},
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

/// Gameplay trait for the game's data and logic.
#[async_trait]
pub trait Gameplay: AccountOps + ProfileOps + GameOps {}

/// Blanket implementation of `Gameplay` for types that implement `HasContext`.
#[async_trait]
impl<T: HasContext + ?Sized> Gameplay for T {}

/// Blanket implementation of `HasContext` for a tuple of `DbConn` and `Codex`.
impl HasContext for (DbConn, Codex) {
    fn db(&self) -> &DbConn {
        &self.0
    }

    fn codex(&self) -> &Codex {
        &self.1
    }

    fn sortie_store(&self) -> &SortieStore {
        &GLOBAL_SORTIE_STORE
    }

    fn practice_store(&self) -> &PracticeStore {
        &GLOBAL_PRACTICE_STORE
    }
}
