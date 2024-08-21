use serde::{Deserialize, Serialize};

/// Initial furniture values
pub static FURNITURE_INIT_VALUES: [i64; 6] = [1, 38, 72, 102, 133, 164];

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
	/// Profile ID
	pub id: i64,

	/// Floor
	pub floor: i64,

	/// Wallpaper
	pub wallpaper: i64,

	/// Window
	pub window: i64,

	/// Wall hanging
	pub wall_hanging: i64,

	/// Shelf
	pub shelf: i64,

	/// Desk
	pub desk: i64,
}
