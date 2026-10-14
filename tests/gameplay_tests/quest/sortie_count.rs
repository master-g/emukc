//! Integration tests: Bw1's "sortie 36 times" counts sorties, not battles (#115).
//!
//! - 214 Bw1 あ号作戦 — four `Sortie` conditions; the first (`times: 36`) is
//!   counted when a sortie starts, the other three per battle.
//! - 216 Bd2 / 210 Bd3 — same shape as Bw1's first, but counted per battle.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        gameplay::scenario::{Scenario, apply_scenario},
        prelude::*,
    };

    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-sc-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "sortie-count").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        apply_scenario(context, pid, &Scenario::fresh_1_1()).await.unwrap();
        pid
    }

    async fn taken(context: &crate::TestContext, pid: i64, quest_id: i64) {
        context.quest_add(pid, quest_id).await.unwrap();
        context.quest_start(pid, quest_id).await.unwrap();
    }

    /// What each `Sortie` condition of the quest still asks for, in order.
    async fn remaining(context: &crate::TestContext, pid: i64, quest_id: i64) -> Vec<i64> {
        let records = context.get_quest_records(pid).await.unwrap();
        let record = records.iter().find(|r| r.quest_id == quest_id).expect("quest has a record");
        let conds: Vec<Kc3rdQuestCondition> =
            serde_json::from_value(record.requirements.clone()).unwrap();
        conds
            .iter()
            .filter_map(|c| match c {
                Kc3rdQuestCondition::Sortie(sortie) => Some(sortie.times),
                _ => None,
            })
            .collect()
    }

    /// Sail 1-1 to its end, fighting every battle cell; returns the battles fought.
    async fn sail_1_1(context: &crate::TestContext, pid: i64) -> i64 {
        let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let (mut event_id, mut has_next) = (start.event_id, start.has_next);
        let mut battles = 0;
        loop {
            if matches!(event_id, 4 | 5) {
                context.sortie_battle(pid, 1).await.unwrap();
                context.sortie_battle_result(pid).await.unwrap();
                battles += 1;
            }
            if !has_next {
                return battles;
            }
            let next = context.next_sortie(pid, None).await.unwrap();
            (event_id, has_next) = (next.event_id, next.has_next);
        }
    }

    #[tokio::test]
    async fn a_sortie_of_several_battles_counts_once_for_bw1() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "several").await;
        taken(&context, pid, 214).await;

        let battles = sail_1_1(&context, pid).await;

        assert!(battles >= 2, "1-1 is fought twice on the way to its boss, got {battles}");
        let remaining = remaining(&context, pid, 214).await;
        assert_eq!(remaining[0], 35);
        // "Reach the boss 24 times" still counts the boss battle, as before.
        assert_eq!(remaining[1], 23);
    }

    #[tokio::test]
    async fn a_sortie_that_retreats_before_any_battle_counts_for_bw1() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "retreat").await;
        taken(&context, pid, 214).await;

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        context.sortie_goback_port(pid).await.unwrap();

        assert_eq!(remaining(&context, pid, 214).await, vec![35, 24, 12, 6]);
    }

    #[tokio::test]
    async fn bw1_boss_and_rank_requirements_still_count_battles() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "boss").await;
        taken(&context, pid, 214).await;

        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        // Starting a sortie touches the first requirement only.
        assert_eq!(remaining(&context, pid, 214).await, vec![35, 24, 12, 6]);

        context.sortie_battle(pid, 1).await.unwrap();
        context.sortie_battle_result(pid).await.unwrap();

        // A battle leaves the sortie count alone.
        assert_eq!(remaining(&context, pid, 214).await[0], 35);
    }

    #[tokio::test]
    async fn bd2_and_bd3_still_count_battles() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "daily").await;
        taken(&context, pid, 210).await;

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        assert_eq!(remaining(&context, pid, 210).await, vec![10], "a start alone is no battle");
        context.sortie_goback_port(pid).await.unwrap();

        let battles = sail_1_1(&context, pid).await;
        assert_eq!(remaining(&context, pid, 210).await, vec![10 - battles]);

        taken(&context, pid, 216).await;
        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        assert_eq!(remaining(&context, pid, 216).await, vec![1]);
        context.sortie_battle(pid, 1).await.unwrap();
        context.sortie_battle_result(pid).await.unwrap();
        assert_eq!(remaining(&context, pid, 216).await, vec![0]);
    }
}
