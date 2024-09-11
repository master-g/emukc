//! Time utilities for the game.

use chrono::{DateTime, Datelike, FixedOffset, Local, TimeZone, Timelike, Utc};

/**
 * Returns a string representation of a date, in kantai collection style.
 *
 * # Arguments
 * * `ms` - The timestamp in milliseconds.
 * * `symbol` - The symbol to separate the date and time.
 *
 * # Returns
 * A string representation of the date.
 */
#[must_use]
pub fn format_date(ms: i64, symbol: &str) -> String {
	let pad = |n: u32| format!("{:02}", n);
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

/// Get today's JST (UTC+9) at 5 AM, and convert it to UTC.
///
/// # Returns
///
/// A `DateTime<Utc>` representing today's 5 AM JST.
#[must_use]
pub fn jst_today_0500_utc() -> DateTime<Utc> {
	let now = Utc::now();
	let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
	let tokyo_now = now.with_timezone(&tokyo_tz);
	let today = tokyo_now.date_naive();
	let five_am_jst = today.and_hms_opt(5, 0, 0).unwrap().and_local_timezone(tokyo_tz).unwrap();

	five_am_jst.with_timezone(&Utc)
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

	let first_day_0500_jst =
		tokyo_tz.from_local_datetime(&first_day_of_quarter.and_hms_opt(5, 0, 0).unwrap()).unwrap();

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

// Re-export chrono.
pub use chrono;

pub mod prelude {
	//! The `emukc_time` crate prelude.
	#[doc(hidden)]
	pub use crate::{
		format_date, jst_0500_day_one_of_quarter, jst_0500_day_one_of_year, jst_0500_of_nth_day,
		jst_day_of_month, jst_monday_0500_utc, jst_today_0500_utc,
	};
}
