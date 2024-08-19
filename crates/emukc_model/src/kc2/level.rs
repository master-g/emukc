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
		(1, 10000),
		(10, 1000),
		(5, 2000),
		(5, 3000),
		(10, 4000),
		(10, 5000),
		(5, 7000),
		(5, 8000),
		(5, 9000),
		(1, 10000),
		(1, 2000),
		(1, 3000),
		(1, 4000),
		(1, 5000),
		(1, 6000),
		(1, 7000),
		(1, 8000),
		(1, 9000),
		(1, 10000),
		(1, 13000),
		(1, 26000),
		(1, 39000),
		(1, 52000),
		(1, 65000),
		(1, 78000),
		(1, 84000),
		(1, 104000),
		(1, 123000),
		(1, 50000),
		(1, 100000),
		(1, 200000),
		(1, 400000),
	];

	let mut ship_exp_table = Vec::new();
	let mut exp_start = 0;
	let mut diff_now = 0;

	for (count, diff) in sections {
		for _ in 0..count {
			diff_now += diff;
			exp_start += diff_now;
			ship_exp_table.push(exp_start);
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
		return (1, 100 - exp);
	}
	for (i, &exp_required) in SHIP_EXP_TABLE.iter().enumerate() {
		if exp < exp_required {
			return (i as i64 + 1, exp_required);
		}
	}

	(175, 0)
}

#[cfg(test)]
mod test {

	#[test]
	fn test_ship_exp_table() {
		for (i, &exp) in super::SHIP_EXP_TABLE.iter().enumerate() {
			println!("Level: {}, Exp: {}", i + 1, exp);
		}
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

			if new_lv == 175 {
				break;
			}
		}
	}
}
