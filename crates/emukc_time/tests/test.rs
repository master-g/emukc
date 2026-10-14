//! Tests for `emukc_time`

#[cfg(test)]
mod test {
    use chrono::{DateTime, FixedOffset, TimeZone, Utc};
    use emukc_time::KcTime;

    #[test]
    fn test_format_date() {
        assert_eq!(KcTime::format_date(1610000000000, "T"), "2021-01-07T14:13:20".to_string());
    }

    #[test]
    fn test_jst_today_0500() {
        // get current UTC time
        let now: DateTime<Utc> = Utc::now();

        // convert current UTC time to JST (UTC+9)
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap(); // 9小时 * 3600秒/小时 = 32400秒
        let tokyo_now: DateTime<FixedOffset> = now.with_timezone(&tokyo_tz);

        // get today's date (only the date part)
        let today = tokyo_now.date_naive();

        // construct 5 AM JST time of today
        let five_am_jst = today.and_hms_opt(5, 0, 0).unwrap().and_local_timezone(tokyo_tz).unwrap();

        let five_am_utc = five_am_jst.with_timezone(&Utc);

        // convert 5 AM JST time of today to UTC
        println!("Current time in Tokyo: {}", tokyo_now);
        println!("Today at 5 AM JST: {}", five_am_jst);
        println!("Today at 5 AM JST in UTC: {}", five_am_utc);
        assert_eq!(KcTime::jst_today_hour_utc(5), five_am_utc);
    }

    #[test]
    fn test_jst_monday_0500() {
        println!("Current time in UTC: {}", Utc::now());
        println!("Monday at 5 AM JST in UTC: {}", KcTime::jst_monday_0500_utc());
    }

    #[test]
    fn test_jst_nth_0500() {
        for i in 1..=31 {
            if i == KcTime::jst_day_of_month() {
                println!(
                    "{}th day of month at 5 AM JST in UTC: {:?}",
                    i,
                    KcTime::jst_0500_of_nth_day(i)
                );
            }
        }
    }

    #[test]
    fn test_jst_day_one_of_quarter() {
        println!(
            "First day of quarter at 5 AM JST in UTC: {:?}",
            KcTime::jst_0500_day_one_of_quarter()
        );
    }

    #[test]
    fn test_jst_day_one_of_year() {
        println!("First day of year at 5 AM JST in UTC: {:?}", KcTime::jst_0500_day_one_of_year());
    }

    #[test]
    fn test_before_or_after() {
        let now = Utc::now();
        let before_hour = 3;
        let after_hour = 15;
        println!("Current time in UTC: {}", now);
        println!("Before 3 AM JST today: {}", KcTime::jst_today_hour_utc(before_hour));
        println!("After 3 PM JST today: {}", KcTime::jst_today_hour_utc(after_hour));
        println!(
            "Is before 3 AM JST today or after 3 PM JST today? {}",
            KcTime::is_before_or_after_jst_today_hour(&now, before_hour, after_hour)
        );
    }

    #[test]
    fn test_next_day_0500() {
        let now = Utc::now();
        println!("Current time in UTC: {}", now);
        println!("Next day at 5 AM JST in UTC: {:?}", KcTime::jst_next_day_0500(&now));
    }

    #[test]
    fn test_next_monday_0500() {
        let now = Utc::now();
        println!("Current time in UTC: {}", now);
        println!("Next Monday at 5 AM JST in UTC: {:?}", KcTime::jst_next_monday_0500(&now));
    }

    #[test]
    fn test_next_1st_day_of_the_month() {
        let now = Utc::now();
        println!("Current time in UTC: {}", now);
        println!(
            "Next 1st day of the month at 5 AM JST in UTC: {:?}",
            KcTime::jst_next_1st_day_of_the_month(&now)
        );
    }

    #[test]
    fn test_jst_next_370_day_of_the_month() {
        let now = Utc::now();
        println!("Current time in UTC: {}", now);
        println!(
            "Next 3rd, 7th, or 0th day of the month at 5 AM JST in UTC: {:?}",
            KcTime::jst_next_370th_day_of_the_month(&now)
        );
    }

    #[test]
    fn test_jst_next_28_day_of_the_month() {
        let now = Utc::now();
        println!("Current time in UTC: {}", now);
        println!(
            "Next 2nd or 8th day of the month at 5 AM JST in UTC: {:?}",
            KcTime::jst_next_28th_day_of_the_month(&now)
        );
    }

    #[test]
    fn the_next_nth_day_is_the_first_0500_after_the_given_time() {
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let at = |month, day, hour, minute| {
            jst.with_ymd_and_hms(2026, month, day, hour, minute, 0).unwrap().with_timezone(&Utc)
        };
        let next_28 = |ts| KcTime::jst_next_28th_day_of_the_month(&ts);
        let next_370 = |ts| KcTime::jst_next_370th_day_of_the_month(&ts);

        // Before 05:00 on a matching day it is that day's 05:00; this used to overflow.
        assert_eq!(next_28(at(10, 12, 1, 39)), at(10, 12, 5, 0));
        assert_eq!(next_370(at(10, 13, 1, 0)), at(10, 13, 5, 0));
        // Before 05:00 on any other day, and from 05:00 on, it is the next matching day.
        assert_eq!(next_370(at(10, 12, 1, 39)), at(10, 13, 5, 0));
        assert_eq!(next_28(at(10, 12, 5, 0)), at(10, 18, 5, 0));
        assert_eq!(next_28(at(10, 12, 6, 0)), at(10, 18, 5, 0));
        assert_eq!(next_370(at(10, 13, 6, 0)), at(10, 17, 5, 0));
        // Across the end of a month, with no 30th in February.
        assert_eq!(next_28(at(2, 28, 6, 0)), at(3, 2, 5, 0));
        assert_eq!(next_370(at(2, 27, 6, 0)), at(3, 3, 5, 0));
        assert_eq!(next_370(at(10, 30, 6, 0)), at(11, 3, 5, 0));
    }

    #[test]
    fn the_next_day_0500_depends_only_on_the_given_time() {
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let at = |month, day, hour, minute| {
            jst.with_ymd_and_hms(2026, month, day, hour, minute, 0).unwrap().with_timezone(&Utc)
        };
        let next = |ts| KcTime::jst_next_day_0500(&ts);

        assert_eq!(next(at(3, 3, 23, 0)), at(3, 4, 5, 0));
        assert_eq!(next(at(3, 4, 4, 59)), at(3, 4, 5, 0));
        assert_eq!(next(at(3, 4, 5, 0)), at(3, 5, 5, 0));
        assert_eq!(next(at(2, 28, 12, 0)), at(3, 1, 5, 0));
    }

    /// 2027-03-08 is a Monday. The week runs from 05:00 on Monday to 04:59 on the next,
    /// whatever today is.
    #[test]
    fn the_next_monday_0500_depends_only_on_the_given_time() {
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let at = |year, month, day, hour, minute| {
            jst.with_ymd_and_hms(year, month, day, hour, minute, 0).unwrap().with_timezone(&Utc)
        };
        let next = |ts| KcTime::jst_next_monday_0500(&ts);

        assert_eq!(next(at(2027, 3, 3, 12, 0)), at(2027, 3, 8, 5, 0));
        assert_eq!(next(at(2027, 3, 7, 23, 59)), at(2027, 3, 8, 5, 0));
        assert_eq!(next(at(2027, 3, 8, 4, 59)), at(2027, 3, 8, 5, 0));
        assert_eq!(next(at(2027, 3, 8, 5, 0)), at(2027, 3, 15, 5, 0));
        assert_eq!(next(at(2027, 3, 14, 12, 0)), at(2027, 3, 15, 5, 0));
        // A week long over, and one across the turn of a year.
        assert_eq!(next(at(2020, 1, 1, 12, 0)), at(2020, 1, 6, 5, 0));
        assert_eq!(next(at(2026, 12, 31, 12, 0)), at(2027, 1, 4, 5, 0));
    }

    #[test]
    fn the_quest_day_turns_at_0500_jst() {
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let at = |month, day, hour, minute| {
            jst.with_ymd_and_hms(2026, month, day, hour, minute, 0).unwrap().with_timezone(&Utc)
        };
        let day = |month, day| chrono::NaiveDate::from_ymd_opt(2026, month, day).unwrap();

        assert_eq!(KcTime::jst_quest_day(&at(3, 3, 23, 0)), day(3, 3));
        assert_eq!(KcTime::jst_quest_day(&at(3, 4, 4, 59)), day(3, 3));
        assert_eq!(KcTime::jst_quest_day(&at(3, 4, 5, 0)), day(3, 4));
        assert_eq!(KcTime::jst_quest_day(&at(3, 1, 2, 0)), day(2, 28));
    }

    fn jst(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        FixedOffset::east_opt(9 * 3600)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn quarterly_quests_reset_on_the_first_of_march_june_september_and_december() {
        let next = |start| KcTime::jst_next_quarter_day_one_0500(&start);

        assert_eq!(next(jst(2026, 2, 10, 12, 0)), jst(2026, 3, 1, 5, 0));
        assert_eq!(next(jst(2026, 3, 1, 4, 59)), jst(2026, 3, 1, 5, 0));
        assert_eq!(next(jst(2026, 3, 1, 5, 0)), jst(2026, 6, 1, 5, 0));
        assert_eq!(next(jst(2026, 8, 31, 23, 0)), jst(2026, 9, 1, 5, 0));
        assert_eq!(next(jst(2026, 12, 1, 5, 0)), jst(2027, 3, 1, 5, 0));
        assert_eq!(next(jst(2027, 1, 1, 3, 0)), jst(2027, 3, 1, 5, 0));
    }

    #[test]
    fn yearly_quests_reset_on_the_first_of_their_own_month() {
        let next = |start, month| KcTime::jst_next_year_day_one_0500(&start, month);

        assert_eq!(next(jst(2026, 6, 1, 12, 0), 5), jst(2027, 5, 1, 5, 0));
        assert_eq!(next(jst(2026, 4, 30, 12, 0), 5), jst(2026, 5, 1, 5, 0));
        assert_eq!(next(jst(2026, 5, 1, 4, 59), 5), jst(2026, 5, 1, 5, 0));
        assert_eq!(next(jst(2026, 5, 1, 5, 0), 5), jst(2027, 5, 1, 5, 0));
        assert_eq!(next(jst(2026, 6, 1, 12, 0), 1), jst(2027, 1, 1, 5, 0));
        assert_eq!(next(jst(2027, 1, 1, 3, 0), 1), jst(2027, 1, 1, 5, 0));
    }
}
