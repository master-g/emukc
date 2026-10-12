//! The two daily quests tied to the day of the month: 敵空母を３隻撃沈せよ！ (Bd4)
//! on days ending in 3, 7 or 0, and 敵輸送船団を叩け！ (Bd6) on days ending in 2 or 8.
//! Every test runs against fixed times, never the clock.

#[cfg(test)]
mod tests {
    use emukc_internal::db::entity::profile::quest::progress;
    use emukc_internal::time::chrono::{DateTime, FixedOffset, TimeZone, Utc};

    /// 敵艦隊を撃破せよ！, the daily both quests follow.
    const BD1: i64 = 201;
    /// 敵艦隊主力を撃滅せよ！, an ordinary daily that follows Bd1 as well.
    const BD2: i64 = 216;
    const BD4: i64 = 211;
    const BD6: i64 = 212;

    /// A time in 2026, in Japan.
    fn jst(month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        FixedOffset::east_opt(9 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, month, day, hour, minute, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-ld-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "limited-day-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    /// Hand the profile a fulfilled quest and claim it at the given time. The
    /// list is read first, as the client does, so that yesterday's records are
    /// reset before the quest is handed out again.
    async fn complete(context: &crate::TestContext, pid: i64, quest_id: i64, at: DateTime<Utc>) {
        context.get_quest_records_at(pid, at).await.unwrap();
        context.quest_add(pid, quest_id).await.unwrap();
        context.quest_start(pid, quest_id).await.unwrap();
        context.quest_clear_and_claim_reward_at(pid, quest_id, None, at).await.unwrap();
    }

    /// The quest as the list shows it at the given time, if it is there.
    async fn listed(
        context: &crate::TestContext,
        pid: i64,
        quest_id: i64,
        at: DateTime<Utc>,
    ) -> Option<progress::Model> {
        let records = context.get_quest_records_at(pid, at).await.unwrap();
        records.into_iter().find(|record| record.quest_id == quest_id)
    }

    #[tokio::test]
    async fn each_quest_is_offered_on_its_days_only() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "days").await;

        // The 2nd: Bd6, not Bd4.
        let day = jst(3, 2, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD6, day).await.is_some());
        assert!(listed(&context, pid, BD4, day).await.is_none());

        // The 3rd: Bd4, not Bd6.
        let day = jst(3, 3, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD4, day).await.is_some());
        assert!(listed(&context, pid, BD6, day).await.is_none());

        // The 4th: neither, while an ordinary daily is there as on any day.
        let day = jst(3, 4, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD4, day).await.is_none());
        assert!(listed(&context, pid, BD6, day).await.is_none());
        assert!(listed(&context, pid, BD2, day).await.is_some());
    }

    #[tokio::test]
    async fn the_end_of_the_month_follows_the_last_digit() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "month-end").await;

        let day = jst(3, 30, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD4, day).await.is_some());

        // The 31st ends in 1.
        let day = jst(3, 31, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD4, day).await.is_none());
        assert!(listed(&context, pid, BD6, day).await.is_none());
    }

    #[tokio::test]
    async fn february_stops_at_the_28th() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "february").await;

        let day = jst(2, 27, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD4, day).await.is_some());
        let day = jst(2, 28, 12, 0);
        complete(&context, pid, BD1, day).await;
        assert!(listed(&context, pid, BD6, day).await.is_some());
        // The 28th lasts until 05:00 on March 1st.
        assert!(listed(&context, pid, BD6, jst(3, 1, 4, 59)).await.is_some());
        assert!(listed(&context, pid, BD6, jst(3, 1, 5, 0)).await.is_none());
    }

    #[tokio::test]
    async fn a_quest_taken_late_lasts_until_0500() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "late").await;

        let taken = jst(3, 3, 23, 0);
        complete(&context, pid, BD1, taken).await;
        assert!(listed(&context, pid, BD4, taken).await.is_some());
        assert!(listed(&context, pid, BD2, taken).await.is_some());
        context.quest_start(pid, BD4).await.unwrap();

        // Before 05:00 it is still the 3rd.
        let still = listed(&context, pid, BD4, jst(3, 4, 4, 59)).await.unwrap();
        assert_eq!(still.status, progress::Status::Activated);
        assert!(listed(&context, pid, BD2, jst(3, 4, 4, 59)).await.is_some());

        // At 05:00 the day turns and both dailies are gone with Bd1.
        assert!(listed(&context, pid, BD4, jst(3, 4, 5, 0)).await.is_none());
        assert!(listed(&context, pid, BD2, jst(3, 4, 5, 0)).await.is_none());

        // The next 3/7/0 day offers it afresh, not as it was left.
        let next = jst(3, 7, 12, 0);
        complete(&context, pid, BD1, next).await;
        let again = listed(&context, pid, BD4, next).await.unwrap();
        assert_eq!(again.status, progress::Status::Idle);
        assert_eq!(again.progress, progress::Progress::Empty);
    }

    #[tokio::test]
    async fn a_completed_quest_returns_on_its_next_day() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "done").await;

        let done = jst(3, 3, 12, 0);
        complete(&context, pid, BD4, done).await;
        complete(&context, pid, BD1, done).await;
        assert!(listed(&context, pid, BD4, done).await.is_none(), "done for the day");

        // Not on the days between, even with Bd1 done again.
        let between = jst(3, 5, 12, 0);
        complete(&context, pid, BD1, between).await;
        assert!(listed(&context, pid, BD4, between).await.is_none());

        let next = jst(3, 7, 12, 0);
        complete(&context, pid, BD1, next).await;
        assert!(listed(&context, pid, BD4, next).await.is_some());
    }

    #[tokio::test]
    async fn the_prerequisite_still_gates_the_quest_on_its_day() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "prerequisite").await;

        assert!(listed(&context, pid, BD4, jst(3, 3, 12, 0)).await.is_none());
        assert!(listed(&context, pid, BD6, jst(3, 2, 12, 0)).await.is_none());
    }
}
