use serde::{Deserialize, Serialize};

/// User owned furniture
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Furniture {
	/// Profile ID
	pub id: i64,

	/// Furniture ID
	pub furniture_id: i64,
}

/// Furniture config
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FurnitureConfig {
	/// initial furniture id list
	pub initial: Vec<i64>,
}

impl Default for FurnitureConfig {
	fn default() -> Self {
		Self {
			initial: vec![1, 38, 72, 102, 133, 164],
		}
	}
}
