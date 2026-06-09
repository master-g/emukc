//! Full-sortie golden-transcript tests (U6).
//!
//! Drives the public gameplay traits over an in-memory DB with the thread-local
//! seed, renders the transcript (U2), and freezes it as a regression. This pins
//! routing + unlock + battle together (KTD-2). Battle-math isolation is left to
//! `emukc_battle`'s existing `SeededRng` golden vectors, not a second altitude.
//!
//! Regenerate intentionally: run the test, copy the `left` value from the
//! failure into `GOLDEN`, and note the behavior change in the commit message
//! (mirrors the `roll_scratch_damage_golden_vector` convention).

#[cfg(test)]
mod tests {
    use emukc_internal::crypto::rng;
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-golden", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "golden-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    /// Apply `fresh_1_1`, seed, drive the first 1-1 battle, render the transcript.
    async fn render_fresh_1_1_seed1(context: &crate::TestContext, pid: i64) -> String {
        apply_scenario(context, pid, &Scenario::fresh_1_1()).await.unwrap();
        rng::seed(1);
        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        context.sortie_battle(pid, 1).await.unwrap();

        let session = context.sortie_store().get_pending_battle(pid).unwrap();
        let simulation = BattleSimulation {
            friendly: session.friendly,
            enemy: session.enemy,
            packet: session.packet,
            outcome: session.outcome,
        };
        let transcript = render_day_battle(&simulation);

        context.sortie_battle_result(pid).await.unwrap();

        // Restore entropy so the seeded stream does not leak to other tests
        // sharing this OS thread (the thread-local seed is process-global).
        rng::reseed_from_entropy();
        transcript
    }

    const GOLDEN: &str = concat!(
        "== Day Battle ==\n",
        "formation: friend=1 enemy=1 engagement=1\n",
        "\n",
        "[shelling 1]\n",
        "  F1 -> E1: dmg 22 [hit]\n",
        "\n",
        "result: rank S, mvp F1, midnight no\n",
        "\n",
        "friendly:\n",
        "  F1 ship951: 35 -> 35 (max 35)\n",
        "  F2 ship951: 35 -> 35 (max 35)\n",
        "\n",
        "enemy:\n",
        "  E1 ship1502: 22 -> 0 (max 22) SUNK\n",
    );

    /// Covers AE1. A fixed scenario + seed produces a transcript identical to the
    /// checked-in golden.
    #[tokio::test]
    async fn full_sortie_transcript_matches_golden() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        let transcript = render_fresh_1_1_seed1(&context, pid).await;
        assert_eq!(transcript, GOLDEN, "full-sortie transcript drifted:\n{transcript}");
    }

    /// Covers AE3. A deliberately altered expected golden must NOT match the
    /// rendered transcript, proving the AE1 guard bites on real drift.
    #[tokio::test]
    async fn golden_guard_catches_drift() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        let transcript = render_fresh_1_1_seed1(&context, pid).await;
        let altered = GOLDEN.replace("rank S", "rank A");
        assert_ne!(transcript, altered, "drift guard failed to distinguish an altered golden");
    }
}
