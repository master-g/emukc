//! Test for `SuffixUtils` js module.

use std::sync::LazyLock;

use regex::Regex;

static DIGIT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());

static RESOURCE_TABLE: LazyLock<Vec<u64>> = LazyLock::new(|| {
	vec![
		6657, 5699, 3371, 8909, 7719, 6229, 5449, 8561, 2987, 5501, 3127, 9319, 4365, 9811, 9927,
		2423, 3439, 1865, 5925, 4409, 5509, 1517, 9695, 9255, 5325, 3691, 5519, 6949, 5607, 9539,
		4133, 7795, 5465, 2659, 6381, 6875, 4019, 9195, 5645, 2887, 1213, 1815, 8671, 3015, 3147,
		2991, 7977, 7045, 1619, 7909, 4451, 6573, 4545, 8251, 5983, 2849, 7249, 7449, 9477, 5963,
		2711, 9019, 7375, 2201, 5631, 4893, 7653, 3719, 8819, 5839, 1853, 9843, 9119, 7023, 5681,
		2345, 9873, 6349, 9315, 3795, 9737, 4633, 4173, 7549, 7171, 6147, 4723, 5039, 2723, 7815,
		6201, 5999, 5339, 4431, 2911, 4435, 3611, 4423, 9517, 3243,
	]
});

/// `SuffixUtils` module.
pub struct SuffixUtils;

impl SuffixUtils {
	/// Create a key from a string.
	///
	/// # Arguments
	///
	/// * `s` - A string slice.
	///
	/// # Returns
	///
	/// A key.
	pub fn create_key(s: &str) -> u64 {
		s.encode_utf16().fold(0, |acc, c| acc + c as u64)
	}

	/// Create a key from a string.
	///
	/// # Arguments
	///
	/// * `s` - A string slice.
	/// * `key` - A key.
	pub fn create(s: &str, key: &str) -> String {
		// extract first consecutive digits from `s`
		let num: u64 = DIGIT_REGEX.find(s).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);

		// calculate index
		let key_sum = Self::create_key(key);
		// avoid division by zero
		let key_len = key.len().max(1) as u64;

		let index = (key_sum + num * key_len) % 100;
		let magic = RESOURCE_TABLE[index as usize];

		let base = num + 7;
		let step1 = 17u64.checked_mul(base).unwrap_or(0);
		let step2 = step1.checked_mul(magic).unwrap_or(0);
		let step3 = step2 % 8973 + 1000;

		step3.to_string()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_create_key() {
		assert_eq!(SuffixUtils::create_key("hello"), 532);
		assert_eq!(SuffixUtils::create_key("world"), 552);
	}

	#[test]
	fn test_create() {
		assert_eq!(SuffixUtils::create("1", "ship_banner"), "2910");
		assert_eq!(SuffixUtils::create("1", "ship_banner_dmg"), "4742");
	}

	#[test]
	fn test_bgm() {
		// SoundManager.bgm.play(bgm_id, loop, )
		// SoundManager.bgm.playBattleBGM = play(bgm_id, loop, fadeOutDuration, category, callback)
		assert_eq!(SuffixUtils::create("014", "bgm_battle"), "3949");
	}

	#[test]
	fn test_ship() {
		assert_eq!(SuffixUtils::create("0001", "ship_album_status"), "5832");
	}
}
