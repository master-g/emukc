//! Leveling and experience related functions.

use std::sync::LazyLock;

static HQ_EXP_TABLE: LazyLock<Vec<i64>> = LazyLock::new(|| {
	let sections = vec![
		(50, 100),
		(10, 200),
		(10, 300),
		(10, 400),
		(10, 500),
		(1, 1000),
		(1, 2000),
		(1, 3000),
		(1, 5000),
		(1, 10000),
		(1, 20000),
		(1, 30000),
		(1, 58500),
		(1, 151500),
		(3, 0),
		(1, 100000),
		(1, 0),
		(1, 100000),
		(1, 0),
		(1, 100000),
		(1, 0),
		(1, 100000),
		(1, 0),
		(1, 100000),
		(1, 0),
		(1, 100000),
		(1, 0),
		(1, 100000),
		(1, 0),
		(4, 0),
	];

	let mut hq_exp_table = Vec::new();
	let mut exp_start = 0;
	let mut diff_now = 0;

	for (count, diff) in sections {
		for _ in 0..count {
			diff_now += diff;
			exp_start += diff_now;
			hq_exp_table.push(exp_start);
		}
	}

	hq_exp_table
});

/// Get the HQ level and the total required exp for the next level.
///
/// # Arguments
///
/// * `exp` - The current exp.
///
/// # Returns
///
/// A tuple of the HQ level and the total required exp for the next level.
pub fn exp_to_hq_level(exp: i64) -> (i64, i64) {
	for (i, &exp_required) in HQ_EXP_TABLE.iter().enumerate() {
		if exp < exp_required {
			return (i as i64 + 1, exp_required);
		}
	}

	(120, 0)
}

static SHIP_EXP_TABLE: LazyLock<Vec<i64>> = LazyLock::new(|| {
	let sections = vec![
		(50, 100),
		(10, 200),
		(10, 300),
		(10, 400),
		(10, 500),
		(1, 1000),
		(1, 2000),
		(1, 3000),
		(1, 5000),
		(1, 10000),
		(1, 20000),
		(1, 30000),
		(1, 58500),
		(1, 0),
		(1, 10000),
		(0, 0),     // 100
		(10, 1000), // 110
		(5, 2000),  // 115
		(5, 3000),  // 120
		(10, 4000), // 130
		(9, 5000),  // 140
		(5, 7000),  // 145
		(5, 8000),  // 150
		(5, 9000),  // 155
		(1, 10000), // 156
		(0, 0),
		(1, 60000), // 157
		(0, 0),
		(1, 80000), // 158
		(0, 0),
		(1, 110000), // 159
		(0, 0),
		(1, 150000), // 160
		(0, 0),
		(1, 200000), // 161
		(0, 0),
		(1, 260000), // 162
		(0, 0),
		(1, 330000), // 163
		(0, 0),
		(1, 410000), // 164
		(0, 0),
		(1, 500000), // 165
		(0, 0),
		(1, 100000), // 166
		(0, 0),
		(1, 113000), // 167
		(0, 0),
		(1, 139000), // 168
		(0, 0),
		(1, 178000), // 169
		(0, 0),
		(1, 230000), // 170
		(0, 0),
		(1, 295000), // 171
		(0, 0),
		(1, 373000), // 172
		(0, 0),
		(1, 457000), // 173
		(0, 0),
		(1, 561000), // 174
		(0, 0),
		(1, 684000), // 175
		(0, 0),
		(1, 150000), // 176
		(0, 0),
		(1, 200000), // 177
		(0, 0),
		(1, 300000), // 178
		(0, 0),
		(1, 500000), // 179
		(0, 0),
		(1, 900000), // 180
	];

	let mut ship_exp_table = Vec::new();
	let mut exp_now = 0;
	let mut diff_now = 0;

	for (count, diff) in sections {
		if diff == 0 {
			diff_now = 0;
			if count != 0 {
				ship_exp_table.push(exp_now);
			}
			continue;
		}
		for _ in 0..count {
			diff_now += diff;
			exp_now += diff_now;
			ship_exp_table.push(exp_now);
		}
	}

	ship_exp_table
});

/// Get the ship level and the required exp for the next level.
///
/// # Arguments
///
/// * `exp` - The current exp.
///
/// # Returns
///
/// A tuple of the ship level and the required exp for the next level.
pub fn exp_to_ship_level(exp: i64) -> (i64, i64) {
	if exp < 100 {
		return (1, 100);
	}
	for (i, &exp_required) in SHIP_EXP_TABLE.iter().enumerate() {
		if exp < exp_required {
			return (i as i64 + 1, exp_required);
		}
	}

	(180, 0)
}

#[cfg(test)]
mod test {

	#[test]
	fn test_ship_exp_table() {
		for (i, &exp) in super::SHIP_EXP_TABLE.iter().enumerate() {
			println!("lv: {}, Exp: {}", i + 2, exp);
		}
	}

	#[test]
	fn test_ship_lv_edges() {
		[1_000_000, 12_000_000, 12_999_999, 13_000_000].iter().for_each(|&exp| {
			println!("Exp: {}, {:?}", exp, super::exp_to_ship_level(exp));
		});
	}

	#[test]
	fn test_exp_to_hq_level() {
		let mut lv = 0;
		let mut exp = 0;
		loop {
			let (new_lv, next) = super::exp_to_hq_level(exp);

			if new_lv != lv {
				lv = new_lv;
				println!("Level: {}, Exp: {}, Next: {}", lv, exp, next);
			}
			exp += 100;

			if new_lv == 120 {
				break;
			}
		}
	}

	#[test]
	fn test_exp_to_ship_level() {
		let mut lv = 0;
		let mut exp = 0;
		loop {
			let (new_lv, next) = super::exp_to_ship_level(exp);

			if new_lv != lv {
				lv = new_lv;
				println!("Level: {}, Exp: {}, Next: {}", lv, exp, next);
			}
			exp += 100;

			if new_lv == 179 {
				break;
			}
		}
	}
}
