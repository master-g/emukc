//! Gameplay integration tests

use std::{
    ops::Deref,
    sync::{Arc, LazyLock},
};

use emukc_internal::prelude::*;

static CODEX: LazyLock<Arc<Codex>> =
    LazyLock::new(|| {
        Arc::new(Codex::load_without_cache_source(".data/codex").expect(
            "Codex load failed; run `cargo run -- bootstrap` first to populate .data/codex/",
        ))
    });

/// Per-test context with an isolated [`SortieStore`] and [`PracticeStore`].
///
/// Avoids profile-id collisions that occur when tests share one store.
pub struct TestContext {
    ctx: Ctx,
}

impl TestContext {
    /// Create a new test context with an in-memory database and isolated stores.
    pub async fn new() -> Self {
        let db = new_mem_db().await.expect("in-memory DB creation failed");
        Self {
            ctx: Ctx::new(Arc::new(db), CODEX.clone()),
        }
    }
}

impl Deref for TestContext {
    type Target = Ctx;

    fn deref(&self) -> &Ctx {
        &self.ctx
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

#[path = "gameplay_tests/ship_onslot_max.rs"]
mod ship_onslot_max;

#[path = "gameplay_tests/ship/hangar_expand.rs"]
mod hangar_expand;

#[path = "gameplay_tests/api_alignment_e2e.rs"]
mod api_alignment_e2e;

#[path = "gameplay_tests/scenario.rs"]
mod scenario;

#[path = "gameplay_tests/battle_golden.rs"]
mod battle_golden;
