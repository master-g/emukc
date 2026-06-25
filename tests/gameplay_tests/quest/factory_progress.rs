//! Integration tests: equipment development advances development-quest progress.
//!
//! Reproduces the batch-craft scenario from plan 2026-06-22-005. `api_multiple_flag=1`
//! crafts up to 3 items in one `create_slotitem` call; each successful item must advance
//! a development quest by exactly 1.
//!
//! Dev quests (codex `quest.json`):
//! - 605 「新装備「開発」指令」 — daily, requires SlotItemConstruction: 1
//! - 607 「装備「開発」集中強化！」 — daily, requires SlotItemConstruction: 3 (the batch case)
//!
//! Asserts on the remaining counter stored in each quest's progress record (3 → 0).

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-fq-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "factory-quest").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    /// Remaining `SlotItemConstruction` count from a quest's stored progress requirements.
    fn slotitem_remaining(reqs: &serde_json::Value) -> i64 {
        let conds: Vec<Kc3rdQuestCondition> = serde_json::from_value(reqs.clone()).unwrap();
        conds
            .iter()
            .find_map(|c| match c {
                Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemConstruction(
                    n,
                )) => Some(*n),
                _ => None,
            })
            .expect("quest has a SlotItemConstruction condition")
    }

    async fn remaining_for(context: &crate::TestContext, pid: i64, quest_id: i64) -> i64 {
        let records = context.get_quest_records(pid).await.unwrap();
        let rec = records
            .iter()
            .find(|r| r.quest_id == quest_id)
            .unwrap_or_else(|| panic!("quest {quest_id} not in progress records"));
        slotitem_remaining(&rec.requirements)
    }

    #[tokio::test]
    async fn single_craft_advances_development_quest_605() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "single").await;
        context.quest_add(pid, 605).await.unwrap();
        context.quest_start(pid, 605).await.unwrap();

        assert_eq!(remaining_for(&context, pid, 605).await, 1, "605 baseline requires 1 craft");

        // One successful craft (slotitem mst_id 1 = 12cm単装砲).
        context.create_slotitem(pid, &[1], &[]).await.unwrap();

        assert_eq!(
            remaining_for(&context, pid, 605).await,
            0,
            "a single successful craft must advance 605 from 1 to 0"
        );
    }

    #[tokio::test]
    async fn batch_craft_of_three_advances_development_quest_607() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "batch3").await;
        context.quest_add(pid, 607).await.unwrap();
        context.quest_start(pid, 607).await.unwrap();

        assert_eq!(remaining_for(&context, pid, 607).await, 3, "607 baseline requires 3 crafts");

        // Batch craft: 3 successful items in one call (api_multiple_flag=1 semantics).
        context.create_slotitem(pid, &[1, 1, 1], &[]).await.unwrap();

        assert_eq!(
            remaining_for(&context, pid, 607).await,
            0,
            "a batch of 3 successful crafts must advance 607 from 3 to 0 (one event per item)"
        );
    }

    #[tokio::test]
    async fn batch_craft_with_one_failure_counts_only_successes() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "batchfail").await;
        context.quest_add(pid, 607).await.unwrap();
        context.quest_start(pid, 607).await.unwrap();

        // Batch of 3 where the middle craft failed (mst_id -1 = failure → no item, no event).
        context.create_slotitem(pid, &[1, -1, 1], &[]).await.unwrap();

        assert_eq!(
            remaining_for(&context, pid, 607).await,
            1,
            "2 successes + 1 failure must advance 607 from 3 to 1 (failures are not counted)"
        );
    }
}
