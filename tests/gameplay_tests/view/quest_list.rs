//! Tests for `Ctx::quest_list_view`.

#[cfg(test)]
mod tests {
    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-ql-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "quest-list-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn quest_list_tab_filters_by_label_type_and_activation() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "tab").await;
        context.quest_add(pid, 605).await.unwrap();
        context.quest_start(pid, 605).await.unwrap();

        let all = context.quest_list_view(pid, 0).await.unwrap();
        assert!(all.items.iter().any(|item| item.label_type == 1), "tab 0 keeps oneshot quests");
        assert!(all.items.iter().any(|item| item.label_type == 2), "tab 0 keeps daily quests");
        assert_eq!(all.exec_count, 1);

        let daily = context.quest_list_view(pid, 2).await.unwrap();
        assert!(!daily.items.is_empty());
        assert!(daily.items.iter().all(|item| item.label_type == 2));
        assert!(daily.items.len() < all.items.len());
        assert!(daily.items.iter().any(|item| item.no == 605));

        let oneshot = context.quest_list_view(pid, 1).await.unwrap();
        assert!(!oneshot.items.is_empty());
        assert!(oneshot.items.iter().all(|item| item.label_type == 1));
        assert!(oneshot.items.iter().all(|item| item.no != 605));

        // Tab 9 drops every non-activated quest. The label-type filter still runs
        // after it and no quest carries label type 9, so the tab stays empty;
        // pinned here so that changing it has to be deliberate.
        let activated = context.quest_list_view(pid, 9).await.unwrap();
        assert!(activated.items.iter().all(|item| item.state != 1));
        assert!(activated.items.is_empty());
    }

    #[tokio::test]
    async fn quest_list_state_follows_status_and_progress() {
        use emukc_internal::db::entity::profile::quest::progress;

        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "state").await;

        // Idle status maps to state 1.
        let records = context.get_quest_records(pid).await.unwrap();
        let idle_id = records
            .iter()
            .find(|record| record.status == progress::Status::Idle)
            .expect("a fresh profile knows idle quests")
            .quest_id;
        let idle = context.quest_list_view(pid, 0).await.unwrap();
        assert_eq!(idle.items.iter().find(|item| item.no == idle_id).unwrap().state, 1);

        // Activated but unfulfilled maps to state 2.
        context.quest_add(pid, 605).await.unwrap();
        context.quest_start(pid, 605).await.unwrap();
        let running = context.quest_list_view(pid, 0).await.unwrap();
        let running_item = running.items.iter().find(|item| item.no == 605).unwrap();
        assert_eq!(running_item.state, 2);
        assert_eq!(running.completed_kind, 0);

        // Fulfilled maps to state 3 with a cleared progress flag; one craft
        // satisfies 605.
        context.create_slotitem(pid, &[1], &[]).await.unwrap();
        let done = context.quest_list_view(pid, 0).await.unwrap();
        let done_item = done.items.iter().find(|item| item.no == 605).unwrap();
        assert_eq!(done_item.state, 3);
        assert_eq!(done_item.progress_flag, 0);
        assert_eq!(done.completed_kind, 1);
    }
}
