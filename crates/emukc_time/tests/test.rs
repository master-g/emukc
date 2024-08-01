#[cfg(test)]
mod test {
	use chrono::{DateTime, FixedOffset, Utc};
	use emukc_time::*;

	#[test]
	fn test_format_date() {
		assert_eq!(format_date(1610000000000, "T"), "2021-01-07T14:13:20".to_string());
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
		assert_eq!(jst_today_0500_utc(), five_am_utc);
	}

	#[test]
	fn test_jst_monday_0500() {
		println!("Current time in UTC: {}", Utc::now());
		println!("Monday at 5 AM JST in UTC: {}", jst_monday_0500_utc());
	}

	#[test]
	fn test_jst_nth_0500() {
		for i in 1..=31 {
			if i == jst_day_of_month() {
				println!("{}th day of month at 5 AM JST in UTC: {:?}", i, jst_0500_of_nth_day(i));
			}
		}
	}

	#[test]
	fn test_jst_day_one_of_quarter() {
		println!("First day of quarter at 5 AM JST in UTC: {:?}", jst_0500_day_one_of_quarter());
	}

	#[test]
	fn test_jst_day_one_of_year() {
		println!("First day of year at 5 AM JST in UTC: {:?}", jst_0500_day_one_of_year());
	}
}
