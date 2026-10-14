//! Quarterly quests reset on 1 March, June, September and December, a yearly
//! quest on the 1st of its own month (`label_type` 100 + month), at 05:00
//! Japan time (issue #112). Records are made and read at fixed times.

#[cfg(test)]
mod tests {
    use emukc_internal::time::chrono::{DateTime, FixedOffset, TimeZone, Utc};

    /// 南西諸島方面「海上警備行動」発令！, a quarterly.
    const QUARTERLY: i64 = 284;
    /// 日英米合同水上艦隊、抜錨せよ！, a yearly whose `label_type` is 105.
    const YEARLY_MAY: i64 = 973;
    /// 「吹雪改三」抜錨します！見てくださいっ！, a yearly whose `label_type` is 101.
    const YEARLY_JANUARY: i64 = 1045;

    fn jst(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        FixedOffset::east_opt(9 * 3600)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Whether the record of `quest_id` started at `start` is still there at `at`,
    /// on a profile of its own.
    async fn kept(quest_id: i64, start: DateTime<Utc>, at: DateTime<Utc>) -> bool {
        let context = crate::TestContext::new().await;
        let account = context.sign_up("test-periodic", "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "periodic").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        context.quest_add_at(pid, quest_id, start).await.unwrap();

        let records = context.get_quest_records_at(pid, at).await.unwrap();
        records.iter().any(|record| record.quest_id == quest_id && record.start_since == start)
    }

    #[tokio::test]
    async fn a_quarterly_resets_on_the_first_of_march() {
        let start = jst(2026, 2, 10, 12, 0);
        assert!(kept(QUARTERLY, start, jst(2026, 3, 1, 4, 59)).await);
        assert!(!kept(QUARTERLY, start, jst(2026, 3, 1, 5, 0)).await);
    }

    #[tokio::test]
    async fn a_quarterly_started_on_the_reset_day_turns_at_0500() {
        assert!(!kept(QUARTERLY, jst(2026, 3, 1, 4, 59), jst(2026, 3, 1, 5, 0)).await);

        let start = jst(2026, 3, 1, 5, 0);
        assert!(kept(QUARTERLY, start, jst(2026, 6, 1, 4, 59)).await);
        assert!(!kept(QUARTERLY, start, jst(2026, 6, 1, 5, 0)).await);
    }

    #[tokio::test]
    async fn a_may_yearly_resets_on_the_first_of_may() {
        let start = jst(2026, 6, 1, 12, 0);
        assert!(kept(YEARLY_MAY, start, jst(2027, 1, 1, 5, 0)).await, "not in January");
        assert!(kept(YEARLY_MAY, start, jst(2027, 5, 1, 4, 59)).await);
        assert!(!kept(YEARLY_MAY, start, jst(2027, 5, 1, 5, 0)).await);

        assert!(!kept(YEARLY_MAY, jst(2026, 4, 30, 12, 0), jst(2026, 5, 1, 5, 0)).await);
    }

    #[tokio::test]
    async fn a_january_yearly_still_resets_on_the_first_of_january() {
        let start = jst(2026, 6, 1, 12, 0);
        assert!(kept(YEARLY_JANUARY, start, jst(2027, 1, 1, 4, 59)).await);
        assert!(!kept(YEARLY_JANUARY, start, jst(2027, 1, 1, 5, 0)).await);
    }
}
