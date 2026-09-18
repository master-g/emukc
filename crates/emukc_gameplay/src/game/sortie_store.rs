//! Instance-scoped store for sortie runtime state.
//!
//! Replaces the former process-global statics (`ACTIVE_SORTIES`,
//! `PENDING_SORTIE_RESULTS`, `PENDING_SORTIE_BATTLES`) with a value that is
//! owned per-context: every [`Ctx`](crate::gameplay::Ctx) builds its own, so
//! each server instance and each test gets an isolated copy.

use std::{collections::HashMap, fmt, future::Future, sync::Arc};

use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;

use super::{
    battle::{
        practice::{PracticeBattleResultSnapshot, PracticeBattleSession},
        sortie::SortieBattleSession,
    },
    sortie::ActiveSortieState,
    sortie_result::SortieBattleResultSnapshot,
};

/// Runtime state backing a single sortie lifecycle.
pub struct SortieStore {
    active_sorties: Mutex<HashMap<i64, ActiveSortieState>>,
    pending_results: Mutex<HashMap<i64, SortieBattleResultSnapshot>>,
    pending_battles: Mutex<HashMap<i64, SortieBattleSession>>,
    profile_locks: Mutex<HashMap<i64, Arc<AsyncMutex<()>>>>,
}

impl SortieStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            active_sorties: Mutex::new(HashMap::new()),
            pending_results: Mutex::new(HashMap::new()),
            pending_battles: Mutex::new(HashMap::new()),
            profile_locks: Mutex::new(HashMap::new()),
        }
    }

    // ── active sorties ──────────────────────────────────────────────

    /// Read the active sortie state of a profile.
    pub fn get_active(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.active_sorties.lock().get(&profile_id).cloned()
    }

    /// Store the active sortie state of a profile, returning the replaced one.
    #[must_use]
    pub fn insert_active(
        &self,
        profile_id: i64,
        state: ActiveSortieState,
    ) -> Option<ActiveSortieState> {
        self.active_sorties.lock().insert(profile_id, state)
    }

    /// Remove and return the active sortie state of a profile.
    pub fn remove_active(&self, profile_id: i64) -> Option<ActiveSortieState> {
        self.active_sorties.lock().remove(&profile_id)
    }

    // ── pending results ─────────────────────────────────────────────

    /// Read the pending battle result of a profile.
    pub fn get_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot> {
        self.pending_results.lock().get(&profile_id).cloned()
    }

    /// Store the pending battle result of a profile.
    pub fn insert_pending_result(&self, profile_id: i64, result: SortieBattleResultSnapshot) {
        self.pending_results.lock().insert(profile_id, result);
    }

    /// Remove and return the pending battle result of a profile.
    pub fn take_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot> {
        self.pending_results.lock().remove(&profile_id)
    }

    // ── pending battles ─────────────────────────────────────────────

    /// Read the pending battle session of a profile.
    pub fn get_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.pending_battles.lock().get(&profile_id).cloned()
    }

    /// Store the pending battle session of a profile.
    pub fn insert_pending_battle(&self, profile_id: i64, session: SortieBattleSession) {
        self.pending_battles.lock().insert(profile_id, session);
    }

    /// Remove and return the pending battle session of a profile.
    pub fn take_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession> {
        self.pending_battles.lock().remove(&profile_id)
    }

    /// Clear all runtime state.
    pub fn clear(&self) {
        self.active_sorties.lock().clear();
        self.pending_results.lock().clear();
        self.pending_battles.lock().clear();
    }

    /// Acquire a per-profile serialization lock and run the given future.
    ///
    /// This ensures that concurrent operations targeting the same profile
    /// (e.g. `next_sortie` and `sortie_battle_impl`) cannot interleave
    /// their read-modify-write cycles on the shared in-memory store.
    pub async fn with_profile_lock<F, T>(&self, profile_id: i64, f: F) -> T
    where
        F: Future<Output = T> + Send,
    {
        let lock = self
            .profile_locks
            .lock()
            .entry(profile_id)
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone();
        let _guard = lock.lock().await;
        f.await
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

/// Process-global store, now only used by this crate's own sortie tests.
///
/// It used to back the tuple `(DbConn, Codex)` context; `Ctx` owns its stores
/// instead, so outside `cfg(test)` nothing reaches for it.
#[cfg(test)]
pub static GLOBAL_SORTIE_STORE: std::sync::LazyLock<SortieStore> =
    std::sync::LazyLock::new(SortieStore::new);

// ── PracticeStore ────────────────────────────────────────────────────

/// Runtime state backing practice battle sessions.
pub struct PracticeStore {
    pending_battles: Mutex<HashMap<i64, PracticeBattleSession>>,
    pending_results: Mutex<HashMap<i64, PracticeBattleResultSnapshot>>,
}

impl PracticeStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            pending_battles: Mutex::new(HashMap::new()),
            pending_results: Mutex::new(HashMap::new()),
        }
    }

    // ── pending battles ─────────────────────────────────────────────

    /// Read the pending battle session of a profile.
    pub fn get_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession> {
        self.pending_battles.lock().get(&profile_id).cloned()
    }

    /// Store the pending battle session of a profile.
    pub fn insert_pending_battle(&self, profile_id: i64, session: PracticeBattleSession) {
        self.pending_battles.lock().insert(profile_id, session);
    }

    /// Remove and return the pending battle session of a profile.
    pub fn take_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession> {
        self.pending_battles.lock().remove(&profile_id)
    }

    /// Drop the pending battle session of a profile, if any.
    pub fn clear_pending_battle(&self, profile_id: i64) {
        self.take_pending_battle(profile_id);
    }

    // ── pending results ─────────────────────────────────────────────

    /// Read the pending battle result of a profile.
    pub fn get_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot> {
        self.pending_results.lock().get(&profile_id).cloned()
    }

    /// Store the pending battle result of a profile.
    pub fn insert_pending_result(&self, profile_id: i64, result: PracticeBattleResultSnapshot) {
        self.pending_results.lock().insert(profile_id, result);
    }

    /// Remove and return the pending battle result of a profile.
    pub fn take_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot> {
        self.pending_results.lock().remove(&profile_id)
    }

    /// Clear all runtime state.
    pub fn clear(&self) {
        self.pending_battles.lock().clear();
        self.pending_results.lock().clear();
    }
}

impl Default for PracticeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for PracticeStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PracticeStore").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_battle::BattleOutcome;
    use emukc_model::kc2::KcSortieResultRank;

    fn minimal_session(profile_id: i64) -> PracticeBattleSession {
        PracticeBattleSession {
            profile_id,
            deck_id: 1,
            enemy_id: 1,
            friendly: vec![],
            enemy: vec![],
            formation: [1, 1, 1],
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 0,
                can_midnight: false,
            },
            air_state: None,
        }
    }

    #[test]
    fn test_practice_store_instances_are_isolated() {
        let a = PracticeStore::new();
        let b = PracticeStore::new();
        a.insert_pending_battle(1, minimal_session(1));
        assert!(a.get_pending_battle(1).is_some());
        assert!(b.get_pending_battle(1).is_none());
    }
}
