//! A weekly quest lasts until 05:00 on Monday, Japan time (issue #101).
//! The quests under test are made and read at fixed times. Creating the profile still
//! reads the clock; none of its records is one of them.

#[cfg(test)]
mod tests {
    use emukc_internal::time::chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};

    /// 敵艦隊主力を撃滅せよ！, a daily.
    const BD2: i64 = 216;
    /// あ号作戦, the weekly that follows it.
    const BW1: i64 = 214;

    fn jst(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        FixedOffset::east_opt(9 * 3600)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-wk-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "weekly-tester").await.unwrap();
        context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id
    }

    /// Hand the profile a fulfilled quest and claim it, all at the given time. The quest
    /// must not be on the list yet: Bw1 goes before Bd2, which the list offers it after,
    /// and Bd2's own daily is left undone.
    async fn complete(context: &crate::TestContext, pid: i64, quest_id: i64, at: DateTime<Utc>) {
        context.get_quest_records_at(pid, at).await.unwrap();
        context.quest_add_at(pid, quest_id, at).await.unwrap();
        context.quest_start(pid, quest_id).await.unwrap();
        context.quest_clear_and_claim_reward_at(pid, quest_id, None, at).await.unwrap();
    }

    async fn listed(
        context: &crate::TestContext,
        pid: i64,
        quest_id: i64,
        at: DateTime<Utc>,
    ) -> bool {
        let records = context.get_quest_records_at(pid, at).await.unwrap();
        records.iter().any(|record| record.quest_id == quest_id)
    }

    /// A weekly claimed in a week is gone for the rest of it and offered again from
    /// 05:00 on Monday, whichever side of today the week lies: a Sunday of 2026 that is
    /// over and one of 2027 that is to come.
    #[tokio::test]
    async fn a_weekly_claimed_is_offered_again_from_monday_0500() {
        for (tag, sunday, monday_0500) in [
            ("past", jst(2026, 3, 8, 12, 0), jst(2026, 3, 9, 5, 0)),
            ("future", jst(2027, 3, 7, 12, 0), jst(2027, 3, 8, 5, 0)),
        ] {
            let context = crate::TestContext::new().await;
            let pid = new_profile(&context, tag).await;

            // Bw1 is handed over first: once its daily is done the list offers it unasked.
            complete(&context, pid, BW1, sunday).await;
            complete(&context, pid, BD2, sunday).await;
            assert!(!listed(&context, pid, BW1, sunday).await, "{tag}: claimed");

            // 04:59 on Monday is still Sunday's quest day and last week.
            let before = monday_0500 - Duration::minutes(1);
            assert!(!listed(&context, pid, BW1, before).await, "{tag}: Monday 04:59");

            // From 05:00 the daily is new as well, and Bw1 follows it again.
            complete(&context, pid, BD2, monday_0500).await;
            assert!(listed(&context, pid, BW1, monday_0500).await, "{tag}: Monday 05:00");
        }
    }

    /// One claimed on Monday, after 05:00, lasts the whole week: the Sunday after is the
    /// same week, the Monday after that the next.
    #[tokio::test]
    async fn a_weekly_claimed_on_monday_lasts_until_the_next() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "monday").await;
        let monday = jst(2027, 3, 8, 5, 0);

        complete(&context, pid, BW1, monday).await;
        complete(&context, pid, BD2, monday).await;

        let sunday = monday + Duration::days(6);
        complete(&context, pid, BD2, sunday).await;
        assert!(!listed(&context, pid, BW1, sunday).await, "the same week");

        let next_monday = monday + Duration::weeks(1);
        complete(&context, pid, BD2, next_monday).await;
        assert!(listed(&context, pid, BW1, next_monday).await, "the next week");
    }
}
