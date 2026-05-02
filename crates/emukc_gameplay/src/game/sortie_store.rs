//! Instance-scoped store for sortie runtime state.
//!
//! Replaces the former process-global statics (`ACTIVE_SORTIES`,
//! `PENDING_SORTIE_RESULTS`, `PENDING_SORTIE_BATTLES`) with a value that can
//! be owned per-context.  [`HasContext`] exposes a default implementation that
//! falls back to a process-global instance, so existing tuple-based test
//! contexts keep working.  The binary-crate [`State`] overrides it with an
//! instance-scoped store, giving each route-test its own isolated copy.

use std::{collections::HashMap, fmt, sync::LazyLock};

use parking_lot::Mutex;

use super::{
    battle::{repository::SortieRepository, sortie::SortieBattleSession},
    sortie::ActiveSortieState,
    sortie_result::SortieBattleResultSnapshot,
};

/// Runtime state backing a single sortie lifecycle.
pub struct SortieStore {
    active_sorties: Mutex<HashMap<i64, ActiveSortieState>>,
    pending_results: Mutex<HashMap<i64, SortieBattleResultSnapshot>>,
    pending_battles: Mutex<HashMap<i64, SortieBattleSession>>,
}

impl SortieStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            active_sorties: Mutex::new(HashMap::new()),
            pending_results: Mutex::new(HashMap::new()),
            pending_battles: Mutex::new(HashMap::new()),
        }
    }

    // ── active sorties ──────────────────────────────────────────────

    pub(super) fn get_active_sortie(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.active_sorties.lock().get(&profile_id).cloned()
    }

    pub(super) fn remove_active_sortie(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.active_sorties.lock().remove(&profile_id)
    }

    // ── pending results ─────────────────────────────────────────────

    pub(super) fn get_pending_result_sortie(
        &self,
        profile_id: i64,
    ) -> Option<SortieBattleResultSnapshot> {
        self.pending_results.lock().get(&profile_id).cloned()
    }

    pub(super) fn take_pending_result_sortie(
        &self,
        profile_id: i64,
    ) -> Option<SortieBattleResultSnapshot> {
        self.pending_results.lock().remove(&profile_id)
    }

    pub(super) fn insert_pending_result_sortie(
        &self,
        profile_id: i64,
        snapshot: SortieBattleResultSnapshot,
    ) {
        self.pending_results.lock().insert(profile_id, snapshot);
    }

    // ── pending battles ─────────────────────────────────────────────

    pub(super) fn get_pending_battle_sortie(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.pending_battles.lock().get(&profile_id).cloned()
    }

    pub(super) fn insert_pending_battle_sortie(
        &self,
        profile_id: i64,
        session: SortieBattleSession,
    ) {
        self.pending_battles.lock().insert(profile_id, session);
    }

    pub(super) fn take_pending_battle_sortie(
        &self,
        profile_id: i64,
    ) -> Option<SortieBattleSession> {
        self.pending_battles.lock().remove(&profile_id)
    }

    /// Clear all runtime state.
    pub fn clear(&self) {
        self.active_sorties.lock().clear();
        self.pending_results.lock().clear();
        self.pending_battles.lock().clear();
    }
}

impl Default for SortieStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SortieStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SortieStore").finish_non_exhaustive()
    }
}

/// Process-global fallback used by tuple-based [`HasContext`] impls.
pub static GLOBAL_SORTIE_STORE: LazyLock<SortieStore> = LazyLock::new(SortieStore::new);

// ── SortieRepository impl ──────────────────────────────────────────────

impl SortieRepository for SortieStore {
    fn get_active(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.get_active_sortie(profile_id)
    }

    fn insert_active(
        &self,
        profile_id: i64,
        state: ActiveSortieState,
    ) -> Option<ActiveSortieState> {
        self.active_sorties.lock().insert(profile_id, state)
    }

    fn remove_active(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.remove_active_sortie(profile_id)
    }

    fn get_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.get_pending_battle_sortie(profile_id)
    }

    fn insert_pending_battle(&self, profile_id: i64, session: SortieBattleSession) {
        self.insert_pending_battle_sortie(profile_id, session);
    }

    fn take_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.take_pending_battle_sortie(profile_id)
    }

    fn get_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot> {
        self.get_pending_result_sortie(profile_id)
    }

    fn insert_pending_result(&self, profile_id: i64, result: SortieBattleResultSnapshot) {
        self.insert_pending_result_sortie(profile_id, result);
    }

    fn take_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot> {
        self.take_pending_result_sortie(profile_id)
    }
}

// ── TestSortieStore ────────────────────────────────────────────────────

/// An isolated [`SortieRepository`] for tests.
///
/// Unlike the process-global [`SortieStore`], each instance is independent
/// so tests can run in parallel without sharing state.
pub struct TestSortieStore {
    inner: SortieStore,
}

impl TestSortieStore {
    pub fn new() -> Self {
        Self {
            inner: SortieStore::new(),
        }
    }

    pub fn clear(&self) {
        self.inner.clear();
    }
}

impl Default for TestSortieStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SortieRepository for TestSortieStore {
    fn get_active(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.inner.get_active(profile_id)
    }

    fn insert_active(
        &self,
        profile_id: i64,
        state: ActiveSortieState,
    ) -> Option<ActiveSortieState> {
        self.inner.insert_active(profile_id, state)
    }

    fn remove_active(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.inner.remove_active(profile_id)
    }

    fn get_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.inner.get_pending_battle(profile_id)
    }

    fn insert_pending_battle(&self, profile_id: i64, session: SortieBattleSession) {
        self.inner.insert_pending_battle(profile_id, session);
    }

    fn take_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.inner.take_pending_battle(profile_id)
    }

    fn get_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot> {
        self.inner.get_pending_result(profile_id)
    }

    fn insert_pending_result(&self, profile_id: i64, result: SortieBattleResultSnapshot) {
        self.inner.insert_pending_result(profile_id, result);
    }

    fn take_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot> {
        self.inner.take_pending_result(profile_id)
    }
}
