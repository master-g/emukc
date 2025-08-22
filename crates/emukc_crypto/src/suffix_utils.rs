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

	fn c(id: u64, typ: &str) -> String {
		let key_value = Self::create_key(typ);
		let resource_index = ((key_value + id * typ.len() as u64) % 100) as usize;
		(17 * (id + 7) * RESOURCE_TABLE[resource_index] % 8973 + 1000).to_string()
	}

	fn pad(id: u64, eors: &str) -> String {
		if eors == "ship" || eors == "slot" {
			format!("{:04}", id)
		} else {
			format!("{:03}", id)
		}
	}

	/// Format a KC2 resource path.
	///
	/// # Arguments
	///
	/// * `id` - The resource ID.
	/// * `eors` - The EORS type (e.g., "ship", /// "slot", "useitem").
	/// * `typ` - The resource type (e.g., "`ship_banner`", "`ship_banner_dmg`").
	/// * `ext` - The file extension (e.g., "png", "json").
	/// * `filename` - An optional filename suffix.
	///
	/// # Returns
	///
	/// A formatted resource path string.
	pub fn format_kc2_resource(
		id: u64,
		eors: &str,
		typ: &str,
		ext: &str,
		filename: Option<&str>,
	) -> String {
		let mut suffix = String::new();
		let mut processed_type = typ.to_string();

		// handle special cases for resource types
		if typ.contains("_d") && !typ.contains("_dmg") {
			suffix = "_d".to_string();
			processed_type = typ.replace("_d", "");
		}

		// handle optional suffixes
		let unique_key = match filename {
			Some(name) => format!("_{}", name),
			None => String::new(),
		};
		let padded = Self::pad(id, eors);
		let magic = Self::c(id, &format!("{}_{}", eors, processed_type));

		format!("kcs2/resources/{eors}/{processed_type}/{padded}{suffix}_{magic}{unique_key}.{ext}",)
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

	#[test]
	fn test_kc2_resource() {
		let path = SuffixUtils::format_kc2_resource(1, "ship", "banner", "png", None);
		assert_eq!(path, "kcs2/resources/ship/banner/0001_2910.png");

		let path = SuffixUtils::format_kc2_resource(1, "ship", "banner2_dmg", "png", None);
		assert_eq!(path, "kcs2/resources/ship/banner2_dmg/0001_7408.png");
	}
}
