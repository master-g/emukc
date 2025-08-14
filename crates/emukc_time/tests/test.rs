//! Tests for `emukc_time`

#[cfg(test)]
mod test {
	use chrono::{DateTime, FixedOffset, Utc};
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
	fn test_jst_next_first_day_of_quarter() {
		let now = Utc::now();
		println!("Current time in UTC: {}", now);
		println!(
			"Next first day of the quarter at 5 AM JST in UTC: {:?}",
			KcTime::jst_next_quarter_day_one_0500(&now)
		);
	}

	#[test]
	fn test_jst_next_day_one_of_year() {
		let now = Utc::now();
		println!("Current time in UTC: {}", now);
		println!(
			"Next first day of the year at 5 AM JST in UTC: {:?}",
			KcTime::jst_next_year_day_one_0500(&now)
		);
	}
}
