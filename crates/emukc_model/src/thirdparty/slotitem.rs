use serde::{Deserialize, Serialize};

/// Slot item extra information
/// This is what we actually using right now
#[deprecated]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemExtraInfo {
	/// `api_id`, slot item id
	pub api_id: i64,

	/// info in picture book
	pub info: String,
}

/// Slot item extra information map
pub type Kc3rdSlotItemExtraInfoMap = std::collections::BTreeMap<i64, Kc3rdSlotItemExtraInfo>;

// TODO: modify the following struct to match the `KcWiki` slot item info

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Kc3rdSlotItemAswDamageType {
	/// DCP
	DepthCargeProjector,

	/// DCR
	DepthChargeRack,
}

/// Slot item thirdparty information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItem {
	/// `api_id`, slot item id
	pub api_id: i64,

	/// slot item name
	pub name: String,

	/// info in picture book
	pub info: String,

	/// can be crafted
	pub craftable: bool,

	/// initial level
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stars: Option<i64>,

	/// flight cost
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flight_cost: Option<i64>,

	/// flight range
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flight_range: Option<i64>,

	/// can attack installations
	pub can_attack_installations: bool,

	/// asw damage type
	pub asw_damage_type: Option<Kc3rdSlotItemAswDamageType>,
}
