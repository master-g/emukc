use serde::{Deserialize, Serialize};

/// Picture book for ships
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PictureBookShip {
	/// Profile ID
	pub id: i64,

	/// Ship sort number
	pub sort_num: i64,

	/// Ship damaged
	pub damaged: bool,

	/// Ship married
	pub married: bool,
}

/// Picture book for slot items
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PictureBookSlotItem {
	/// Profile ID
	pub id: i64,

	/// Slot item sort number
	pub sort_num: i64,
}
