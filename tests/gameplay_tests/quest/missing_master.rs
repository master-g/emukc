//! A quest in progress that a rebuilt codex no longer has.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use emukc_internal::prelude::*;

    /// 中部海域「基地航空隊」展開！
    const QUEST_ID: i64 = 647;

    #[tokio::test]
    async fn a_quest_missing_from_the_codex_leaves_quests_and_actions_working() {
        let context = crate::TestContext::new().await;
        let account = context.sign_up("missing-master", "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "admiral").await.unwrap();
        let pid = profile.profile.id;
        context.quest_add(pid, QUEST_ID).await.unwrap();
        context.quest_start(pid, QUEST_ID).await.unwrap();

        let mut codex = context.codex().clone();
        assert!(codex.quest.remove(&QUEST_ID).is_some());
        let rebuilt = Ctx::new(context.db.clone(), Arc::new(codex));

        let records = rebuilt.get_quest_records(pid).await.unwrap();
        assert!(records.iter().any(|record| record.quest_id == QUEST_ID));
        let ship = rebuilt.add_ship(pid, 1).await.unwrap();
        rebuilt.destroy_ship(pid, ship.api_id, false).await.unwrap();
    }
}
