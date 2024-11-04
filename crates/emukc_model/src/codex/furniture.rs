//! Furniture extension for the codex module.
//!
//! These magic can be found in the `main.js` file.

use super::Codex;

impl Codex {
	/// Check if a furniture needs a craftman.
	///
	/// # Arguments
	///
	/// * `price` - The price of the furniture.
	pub fn furniture_needs_craftman(price: i64) -> bool {
		(2_000..20_000).contains(&price)
	}

	/// Check if a furniture is high grade.
	///
	/// # Arguments
	///
	/// * `price` - The price of the furniture.
	pub fn furniture_is_high_grade(price: i64) -> bool {
		price >= 100_000
	}

	/// Calculate the discount price of a furniture.
	///
	/// # Arguments
	///
	/// * `original_price` - The original price of the furniture.
	pub fn furniture_discount_price(original_price: i64) -> i64 {
		if Self::furniture_is_high_grade(original_price) {
			original_price
		} else {
			let discount: f64 = 0.1 * (original_price as f64 - 100_000.0);
			discount.max(0f64).floor() as i64
		}
	}
}
