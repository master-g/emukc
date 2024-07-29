use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipBasic {
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
	pub slots: Vec<i64>,
	pub equip: Vec<Kc3rdShipSlotItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipSlotItem {
	pub api_id: i64,
	pub star: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipExtraInfo {
	pub api_id: i64,
	pub info: String,
	pub droppable: bool,
	pub buildable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipClassNameInfo {
	pub api_id: i64,
	pub name: String,
}

pub type Kc3rdShipBasicMap = std::collections::BTreeMap<i64, Kc3rdShipBasic>;

pub type Kc3rdShipExtraInfoMap = std::collections::BTreeMap<i64, Kc3rdShipExtraInfo>;

pub type Kc3rdShipClassNameMap = std::collections::BTreeMap<i64, Kc3rdShipClassNameInfo>;
