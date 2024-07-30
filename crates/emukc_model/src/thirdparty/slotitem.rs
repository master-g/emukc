use serde::{Deserialize, Serialize};

/// Slot item thirdparty information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItem {
	/// `api_id`, slot item id
	pub api_id: i64,

	/// slot item name
	pub name: String,

	/// info in picture book
	pub info: String,

	/// can be constructed
	pub buildable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemRemodelInfo {
	// TODO
}

/// Slot item remodel bonus
/// lookup table for stars
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemRemodelBonus {
	firepower: Vec<i64>,
	night_firepower: Vec<i64>,
	hit: Vec<i64>,
	night_hit: Vec<i64>,
}

/// Slot item extra information
/// This is what we actually using right now
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemExtraInfo {
	/// `api_id`, slot item id
	pub api_id: i64,

	/// info in picture book
	pub info: String,
}

/// Slot item extra information map
pub type Kc3rdSlotItemExtraInfoMap = std::collections::BTreeMap<i64, Kc3rdSlotItemExtraInfo>;
