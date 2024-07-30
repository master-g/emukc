use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipBasic {
	/// `api_id`, ship id
	pub api_id: i64,

	/// kaihi, evasion
	pub kaih: Vec<i64>,

	/// taisen, anti-submarine
	pub tais: Vec<i64>,

	/// sakuteki, line of sight
	pub saku: Vec<i64>,

	/// luck
	pub luck: Vec<i64>,

	/// cnum, construction number
	pub cnum: i64,

	/// `slots.len()`: how many slots the ship has
	/// `slots[n]`: how many planes the n-th slot can hold
	pub slots: Vec<i64>,

	/// initial equipment
	pub equip: Vec<Kc3rdShipSlotItem>,
}

/// Ship initial equipment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipSlotItem {
	/// `api_id`, slot item id
	pub api_id: i64,

	/// improvement level
	pub star: i64,
}

/// Ship extra information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipExtraInfo {
	/// `api_id`, ship id
	pub api_id: i64,

	/// ship info in picture book
	pub info: String,

	/// can obtain from sortie
	pub droppable: bool,

	/// can obtain from construction
	pub buildable: bool,
}

/// Ship class name information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipClassNameInfo {
	/// `api_id`, ship class id
	pub api_id: i64,

	/// ship class name
	pub name: String,
}

/// Ship basic information map
pub type Kc3rdShipBasicMap = std::collections::BTreeMap<i64, Kc3rdShipBasic>;

/// Ship extra information map
pub type Kc3rdShipExtraInfoMap = std::collections::BTreeMap<i64, Kc3rdShipExtraInfo>;

/// Ship class name information map
pub type Kc3rdShipClassNameMap = std::collections::BTreeMap<i64, Kc3rdShipClassNameInfo>;
