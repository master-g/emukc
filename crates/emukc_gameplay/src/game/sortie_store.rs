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
    battle::sortie::SortieBattleSession, sortie::ActiveSortieState,
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

    pub(super) fn insert_active_sortie(&self, profile_id: i64, state: ActiveSortieState) {
        self.active_sorties.lock().insert(profile_id, state);
    }

    pub(super) fn modify_active_sortie(
        &self,
        profile_id: i64,
        f: impl FnOnce(&mut ActiveSortieState),
    ) {
        self.active_sorties.lock().entry(profile_id).and_modify(f);
    }

    pub(super) fn remove_active_sortie(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.active_sorties.lock().remove(&profile_id)
    }

    // ── pending results ─────────────────────────────────────────────

    pub(super) fn take_pending_result(
        &self,
        profile_id: i64,
    ) -> Option<SortieBattleResultSnapshot> {
        self.pending_results.lock().remove(&profile_id)
    }

    pub(super) fn insert_pending_result(
        &self,
        profile_id: i64,
        snapshot: SortieBattleResultSnapshot,
    ) {
        self.pending_results.lock().insert(profile_id, snapshot);
    }

    pub(super) fn with_pending_result_mut(
        &self,
        profile_id: i64,
        f: impl FnOnce(&mut SortieBattleResultSnapshot),
    ) {
        if let Some(snapshot) = self.pending_results.lock().get_mut(&profile_id) {
            f(snapshot);
        }
    }

    // ── pending battles ─────────────────────────────────────────────

    pub(super) fn get_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.pending_battles.lock().get(&profile_id).cloned()
    }

    pub(super) fn insert_pending_battle(&self, profile_id: i64, session: SortieBattleSession) {
        self.pending_battles.lock().insert(profile_id, session);
    }

    pub(super) fn take_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.pending_battles.lock().remove(&profile_id)
    }

    pub(super) fn with_pending_battle_mut(
        &self,
        profile_id: i64,
        f: impl FnOnce(&mut SortieBattleSession),
    ) {
        if let Some(session) = self.pending_battles.lock().get_mut(&profile_id) {
            f(session);
        }
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
