//! Time utilities for the game.

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, TimeZone, Timelike, Utc};

/// A utility struct for time operations in `EmuKC`.
pub struct KcTime;

impl KcTime {
    /// Returns a string representation of a date, in kantai collection style.
    ///
    /// # Arguments
    ///
    /// * `ms` - The timestamp in milliseconds.
    /// * `symbol` - The symbol to separate the date and time.
    ///
    /// # Returns
    ///
    /// A string representation of the date.
    pub fn format_date(ms: i64, symbol: &str) -> String {
        let pad = |n: u32| format!("{n:02}");
        let date = Local.timestamp_millis_opt(ms).unwrap();
        format!(
            "{}-{}-{}{}{:02}:{:02}:{:02}",
            date.year(),
            pad(date.month()),
            pad(date.day()),
            symbol,
            date.hour(),
            date.minute(),
            date.second()
        )
    }

    /// Get today's JST (UTC+9) at the specified hour, and convert it to UTC.
    ///
    /// # Arguments
    ///
    /// * `hour` - The hour of the day.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing today's specified hour JST.
    pub fn jst_today_hour_utc(hour: u32) -> DateTime<Utc> {
        let now = Utc::now();
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_now = now.with_timezone(&tokyo_tz);
        let today = tokyo_now.date_naive();
        let n_hour_jst =
            today.and_hms_opt(hour, 0, 0).unwrap().and_local_timezone(tokyo_tz).unwrap();

        n_hour_jst.with_timezone(&Utc)
    }

    /// Get this week's Monday's JST (UTC+9) at 5 AM, and convert it to UTC.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing this week's Monday's 5 AM JST.
    #[must_use]
    pub fn jst_monday_0500_utc() -> DateTime<Utc> {
        let now = Utc::now();
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_now = now.with_timezone(&tokyo_tz);
        let today = tokyo_now.date_naive();
        let weekday = today.weekday().num_days_from_monday();

        let this_monday = today - chrono::Duration::days(weekday as i64);

        let monday_0500 =
            tokyo_tz.from_local_datetime(&this_monday.and_hms_opt(5, 0, 0).unwrap()).unwrap();

        monday_0500.with_timezone(&Utc)
    }

    /// Get the day of the month in JST (UTC+9) today.
    ///
    /// # Returns
    ///
    /// The day of the month in JST.
    pub fn jst_day_of_month() -> u32 {
        let now = Utc::now();
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_now = now.with_timezone(&tokyo_tz);
        let today = tokyo_now.date_naive();
        today.day()
    }

    /// Get the date of the nth day of the month in JST (UTC+9) at 5 AM, and convert it to UTC.
    ///
    /// # Arguments
    ///
    /// * `n` - The nth day of the month.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the nth day of the month at 5 AM JST.
    pub fn jst_0500_of_nth_day(n: u32) -> DateTime<Utc> {
        let now = Utc::now();
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_now = now.with_timezone(&tokyo_tz);
        let today = tokyo_now.date_naive();

        let nth_day = today.with_day(n).unwrap();
        let nth_day_0500_jst =
            tokyo_tz.from_local_datetime(&nth_day.and_hms_opt(5, 0, 0).unwrap()).unwrap();

        nth_day_0500_jst.with_timezone(&Utc)
    }

    /// Get the date of the first day of the quarter in JST (UTC+9) at 5 AM, and convert it to UTC.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the first day of the quarter at 5 AM JST.
    pub fn jst_0500_day_one_of_quarter() -> DateTime<Utc> {
        let now = Utc::now();
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_now = now.with_timezone(&tokyo_tz);
        let today = tokyo_now.date_naive();

        let quarter = (today.month() - 1) / 3;
        let first_day_of_quarter = today.with_day(1).unwrap().with_month(quarter * 3 + 1).unwrap();

        let first_day_0500_jst = tokyo_tz
            .from_local_datetime(&first_day_of_quarter.and_hms_opt(5, 0, 0).unwrap())
            .unwrap();

        first_day_0500_jst.with_timezone(&Utc)
    }

    /// Get the date of the first day of the year in JST (UTC+9) at 5 AM, and convert it to UTC.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the first day of the year at 5 AM JST.
    pub fn jst_0500_day_one_of_year() -> DateTime<Utc> {
        let now = Utc::now();
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_now = now.with_timezone(&tokyo_tz);
        let today = tokyo_now.date_naive();

        let first_day_of_year = today.with_day(1).unwrap().with_month(1).unwrap();

        let first_day_0500_jst =
            tokyo_tz.from_local_datetime(&first_day_of_year.and_hms_opt(5, 0, 0).unwrap()).unwrap();

        first_day_0500_jst.with_timezone(&Utc)
    }

    /// Check if the given time is before or after the specified hour in JST (UTC+9) today.
    ///
    /// # Arguments
    ///
    /// * `t` - The time to check.
    /// * `before_hour` - The hour to check before.
    /// * `after_hour` - The hour to check after.
    pub fn is_before_or_after_jst_today_hour(
        t: &DateTime<Utc>,
        before_hour: u32,
        after_hour: u32,
    ) -> bool {
        let before = Self::jst_today_hour_utc(before_hour);
        let after = Self::jst_today_hour_utc(after_hour);

        t < &before || t >= &after
    }

    /// Get the next day's 5 AM JST (UTC+9) from the given time.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next day's 5 AM JST.
    pub fn jst_next_day_0500(ts: &DateTime<Utc>) -> DateTime<Utc> {
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let next_day = Self::jst_quest_day(ts) + chrono::Duration::days(1);

        tokyo_tz
            .from_local_datetime(&next_day.and_hms_opt(5, 0, 0).unwrap())
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Get the day the given time belongs to for quests: the JST (UTC+9) date,
    /// with the day turning at 5 AM instead of midnight.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to get the day of.
    ///
    /// # Returns
    ///
    /// The date of the quest day.
    pub fn jst_quest_day(ts: &DateTime<Utc>) -> NaiveDate {
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        (ts.with_timezone(&tokyo_tz) - chrono::Duration::hours(5)).date_naive()
    }

    /// Get the next Monday's 5 AM JST (UTC+9) from the given time.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next Monday's 5 AM JST.
    pub fn jst_next_monday_0500(ts: &DateTime<Utc>) -> DateTime<Utc> {
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        // The week turns with the quest day: Monday 04:59 still belongs to Sunday.
        let quest_day = Self::jst_quest_day(ts);
        let days_left = 7 - i64::from(quest_day.weekday().num_days_from_monday());
        let next_monday = quest_day + chrono::Duration::days(days_left);

        tokyo_tz
            .from_local_datetime(&next_monday.and_hms_opt(5, 0, 0).unwrap())
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Get the next 5 AM JST (UTC+9) of the nth day of the month from the given time.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next 5 AM JST of the nth day of the month.
    pub fn jst_next_1st_day_of_the_month(ts: &DateTime<Utc>) -> DateTime<Utc> {
        let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
        let tokyo_then = ts.with_timezone(&tokyo_tz);

        let current_year = tokyo_then.year();
        let current_month = tokyo_then.month();

        let first_day_current_month_jst =
            tokyo_tz.with_ymd_and_hms(current_year, current_month, 1, 5, 0, 0).unwrap();

        if tokyo_then < first_day_current_month_jst {
            first_day_current_month_jst.with_timezone(&Utc)
        } else {
            let (next_year, next_month) = if current_month == 12 {
                (current_year + 1, 1)
            } else {
                (current_year, current_month + 1)
            };

            let first_day_next_month_jst =
                tokyo_tz.with_ymd_and_hms(next_year, next_month, 1, 5, 0, 0).unwrap();

            first_day_next_month_jst.with_timezone(&Utc)
        }
    }

    /// Get the next 5 AM JST (UTC+9) of the 3rd, 7th, or 10th day of the month from the given time.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next 5 AM JST of the 3rd, 7th, or 10th day of the month.
    pub fn jst_next_370th_day_of_the_month(ts: &DateTime<Utc>) -> DateTime<Utc> {
        jst_next_nth_day_of_the_month(NthDay::Day3_7_0, ts)
    }

    /// Get the next 5 AM JST (UTC+9) of the 2nd or 8th day of the month from the given time.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next 5 AM JST of the 2nd or 8th day of the month.
    pub fn jst_next_28th_day_of_the_month(ts: &DateTime<Utc>) -> DateTime<Utc> {
        jst_next_nth_day_of_the_month(NthDay::Day2_8, ts)
    }

    /// Get the first 5 AM JST (UTC+9) on 1 March, June, September or December
    /// strictly after the given time: when quarterly quests reset.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next quarterly reset.
    pub fn jst_next_quarter_day_one_0500(ts: &DateTime<Utc>) -> DateTime<Utc> {
        jst_next_day_one_0500_in(&[3, 6, 9, 12], ts)
    }

    /// Get the first 5 AM JST (UTC+9) on the 1st of `month` strictly after the
    /// given time: when a yearly quest of that month resets.
    ///
    /// # Arguments
    ///
    /// * `ts` - The time to calculate from.
    /// * `month` - The reset month, 1 to 12.
    ///
    /// # Returns
    ///
    /// A `DateTime<Utc>` representing the next yearly reset.
    ///
    /// # Panics
    ///
    /// If `month` is not 1 to 12.
    pub fn jst_next_day_one_0500_of_month(ts: &DateTime<Utc>, month: u32) -> DateTime<Utc> {
        jst_next_day_one_0500_in(&[month], ts)
    }
}

/// The first 1st-of-the-month 05:00 JST in one of `months` (ascending)
/// strictly after `ts`.
fn jst_next_day_one_0500_in(months: &[u32], ts: &DateTime<Utc>) -> DateTime<Utc> {
    let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
    let year = ts.with_timezone(&tokyo_tz).year();

    [year, year + 1]
        .into_iter()
        .flat_map(|year| months.iter().map(move |&month| (year, month)))
        .map(|(year, month)| tokyo_tz.with_ymd_and_hms(year, month, 1, 5, 0, 0).unwrap())
        .find(|reset| reset.with_timezone(&Utc) > *ts)
        .expect("every year has one of the months")
        .with_timezone(&Utc)
}

enum NthDay {
    Day3_7_0,
    Day2_8,
}

fn jst_next_nth_day_of_the_month(nth_day: NthDay, ts: &DateTime<Utc>) -> DateTime<Utc> {
    let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
    let tokyo_then = ts.with_timezone(&tokyo_tz);
    let mut tokyo_time = tokyo_then;

    loop {
        let day = tokyo_time.day();

        let day_found = match nth_day {
            NthDay::Day3_7_0 => matches!(day % 10, 0 | 3 | 7),
            NthDay::Day2_8 => matches!(day % 10, 2 | 8),
        };

        if day_found {
            let target_time = tokyo_tz
                .with_ymd_and_hms(tokyo_time.year(), tokyo_time.month(), day, 5, 0, 0)
                .unwrap();

            if tokyo_then < target_time {
                return target_time.with_timezone(&Utc);
            }
        }

        tokyo_time += chrono::Duration::days(1);
    }
}

// Re-export chrono.
pub use chrono;

pub mod prelude {
    //! The `emukc_time` crate prelude.
    #[doc(hidden)]
    pub use crate::KcTime;
}
