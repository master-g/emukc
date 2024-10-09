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

impl Default for FurnitureConfig {
	fn default() -> Self {
		Self {
			id: 0,
			floor: 1,
			wallpaper: 38,
			window: 72,
			wall_hanging: 102,
			shelf: 133,
			desk: 164,
		}
	}
}

impl FurnitureConfig {
	/// Get API values
	pub fn api_values(&self) -> [i64; 6] {
		[self.floor, self.wallpaper, self.window, self.wall_hanging, self.shelf, self.desk]
	}
}
