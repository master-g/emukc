//! Gameplay integration tests

use emukc_internal::prelude::*;

/// Per-test context with an isolated [`SortieStore`].
///
/// Avoids profile-id collisions that occur when tests share the
/// process-global `GLOBAL_SORTIE_STORE`.
pub struct TestContext {
    db: emukc_internal::db::sea_orm::DbConn,
    codex: Codex,
    sortie_store: SortieStore,
}

impl TestContext {
    /// Create a new test context with an in-memory database and isolated sortie store.
    pub async fn new() -> Self {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        Self {
            db,
            codex,
            sortie_store: SortieStore::new(),
        }
    }
}

impl HasContext for TestContext {
    fn db(&self) -> &emukc_internal::db::sea_orm::DbConn {
        &self.db
    }

    fn codex(&self) -> &Codex {
        &self.codex
    }

    fn sortie_store(&self) -> &SortieStore {
        &self.sortie_store
    }
}

#[path = "gameplay_tests/map/mod.rs"]
mod map;

#[path = "gameplay_tests/quest/mod.rs"]
mod quest;

#[path = "gameplay_tests/useitem_material_sync.rs"]
mod useitem_material_sync;

#[path = "gameplay_tests/remodel_hp_restore.rs"]
mod remodel_hp_restore;

#[path = "gameplay_tests/level_cap_exp.rs"]
mod level_cap_exp;

#[path = "gameplay_tests/remodel_preserve_fields.rs"]
mod remodel_preserve_fields;
