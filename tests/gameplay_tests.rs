//! Gameplay integration tests

use std::sync::LazyLock;

use emukc_internal::prelude::*;

static CODEX: LazyLock<Codex> = LazyLock::new(|| {
    Codex::load_without_cache_source(".data/codex")
        .expect("Codex load failed; run `cargo run -- bootstrap` first to populate .data/codex/")
});

/// Per-test context with an isolated [`SortieStore`] and [`PracticeStore`].
///
/// Avoids profile-id collisions that occur when tests share the
/// process-global `GLOBAL_SORTIE_STORE` / `GLOBAL_PRACTICE_STORE`.
pub struct TestContext {
    db: emukc_internal::db::sea_orm::DbConn,
    codex: &'static Codex,
    sortie_store: SortieStore,
    practice_store: PracticeStore,
}

impl TestContext {
    /// Create a new test context with an in-memory database and isolated stores.
    pub async fn new() -> Self {
        let db = new_mem_db().await.expect("in-memory DB creation failed");
        Self {
            db,
            codex: &CODEX,
            sortie_store: SortieStore::new(),
            practice_store: PracticeStore::new(),
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

    fn practice_store(&self) -> &PracticeStore {
        &self.practice_store
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

#[path = "gameplay_tests/sortie_ammo_reaches_battle.rs"]
mod sortie_ammo_reaches_battle;
