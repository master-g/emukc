//! Repository trait for sortie runtime state.

use crate::game::{sortie::ActiveSortieState, sortie_result::SortieBattleResultSnapshot};

use super::sortie::SortieBattleSession;

/// Abstract store for sortie runtime state.
///
/// Decouples battle session functions from [`SortieStore`] so tests can inject
/// isolated in-memory storage instead of sharing a process-global instance.
pub trait SortieRepository: Send + Sync {
    // ── active sorties ──────────────────────────────────────────────

    fn get_active(&self, profile_id: i64) -> Option<ActiveSortieState>;

    #[must_use]
    fn insert_active(&self, profile_id: i64, state: ActiveSortieState)
    -> Option<ActiveSortieState>;

    fn remove_active(&self, profile_id: i64) -> Option<ActiveSortieState>;

    // ── pending battles ─────────────────────────────────────────────

    fn get_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession>;

    fn insert_pending_battle(&self, profile_id: i64, session: SortieBattleSession);

    fn take_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession>;

    // ── pending results ─────────────────────────────────────────────

    fn get_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot>;

    fn insert_pending_result(&self, profile_id: i64, result: SortieBattleResultSnapshot);

    fn take_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot>;
}
