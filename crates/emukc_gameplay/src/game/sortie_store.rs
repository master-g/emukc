//! Instance-scoped store for sortie runtime state.
//!
//! Replaces the former process-global statics (`ACTIVE_SORTIES`,
//! `PENDING_SORTIE_RESULTS`, `PENDING_SORTIE_BATTLES`) with a value that can
//! be owned per-context.  [`HasContext`] exposes a default implementation that
//! falls back to a process-global instance, so existing tuple-based test
//! contexts keep working.  The binary-crate [`State`] overrides it with an
//! instance-scoped store, giving each route-test its own isolated copy.

use std::{collections::HashMap, fmt, future::Future, sync::Arc, sync::LazyLock};

use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;

use super::{
    battle::{
        practice::{PracticeBattleResultSnapshot, PracticeBattleSession},
        practice_repository::PracticeRepository,
        repository::SortieRepository,
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

/// Process-global fallback used by tuple-based [`HasContext`] impls.
pub static GLOBAL_PRACTICE_STORE: LazyLock<PracticeStore> = LazyLock::new(PracticeStore::new);

impl PracticeRepository for PracticeStore {
    fn get_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession> {
        self.pending_battles.lock().get(&profile_id).cloned()
    }

    fn insert_pending_battle(&self, profile_id: i64, session: PracticeBattleSession) {
        self.pending_battles.lock().insert(profile_id, session);
    }

    fn take_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession> {
        self.pending_battles.lock().remove(&profile_id)
    }

    fn get_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot> {
        self.pending_results.lock().get(&profile_id).cloned()
    }

    fn insert_pending_result(&self, profile_id: i64, result: PracticeBattleResultSnapshot) {
        self.pending_results.lock().insert(profile_id, result);
    }

    fn take_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot> {
        self.pending_results.lock().remove(&profile_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::battle::practice_repository::PracticeRepository;
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

    fn minimal_result(profile_id: i64) -> PracticeBattleResultSnapshot {
        PracticeBattleResultSnapshot {
            deck_id: 1,
            enemy_id: 1,
            friendly_ship_ids: vec![],
            friendly_fleet_snapshot: vec![],
            enemy_ship_ids: vec![],
            win_rank: KcSortieResultRank::S,
            get_exp: 100,
            member_lv: 120,
            member_exp: 0,
            get_base_exp: 80,
            mvp: 1,
            get_ship_exp: vec![],
            get_exp_lvup: vec![],
            did_night_battle: false,
            enemy_level: 100,
            enemy_rank: "元帥".to_string(),
            enemy_deck_name: "test".to_string(),
        }
    }

    #[test]
    fn test_practice_store_insert_get_take_cycle() {
        let store = PracticeStore::new();
        assert!(store.get_pending_battle(1).is_none());

        store.insert_pending_battle(1, minimal_session(1));
        let got = store.get_pending_battle(1).unwrap();
        assert_eq!(got.profile_id, 1);

        let taken = store.take_pending_battle(1).unwrap();
        assert_eq!(taken.profile_id, 1);
        assert!(store.get_pending_battle(1).is_none());

        store.insert_pending_result(1, minimal_result(1));
        let taken_result = store.take_pending_result(1).unwrap();
        assert_eq!(taken_result.get_exp, 100);
        assert!(store.take_pending_result(1).is_none());
    }

    #[test]
    fn test_practice_store_empty_take_returns_none() {
        let store = PracticeStore::new();
        assert!(store.take_pending_battle(42).is_none());
        assert!(store.take_pending_result(42).is_none());
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
