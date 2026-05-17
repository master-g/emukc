//! Repository trait for practice runtime state.

use super::practice::{PracticeBattleResultSnapshot, PracticeBattleSession};

/// Abstract store for practice runtime state.
///
/// Decouples practice session functions from [`PracticeStore`] so tests can
/// inject isolated in-memory storage instead of sharing a process-global
/// instance.
pub trait PracticeRepository: Send + Sync {
    // ── pending battles ─────────────────────────────────────────────

    fn get_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession>;

    fn insert_pending_battle(&self, profile_id: i64, session: PracticeBattleSession);

    fn take_pending_battle(&self, profile_id: i64) -> Option<PracticeBattleSession>;

    // ── pending results ─────────────────────────────────────────────

    fn get_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot>;

    fn insert_pending_result(&self, profile_id: i64, result: PracticeBattleResultSnapshot);

    fn take_pending_result(&self, profile_id: i64) -> Option<PracticeBattleResultSnapshot>;

    // ── convenience ─────────────────────────────────────────────────

    fn clear_pending_battle(&self, profile_id: i64) {
        self.take_pending_battle(profile_id);
    }

    fn clear_pending_result(&self, profile_id: i64) {
        self.take_pending_result(profile_id);
    }
}
