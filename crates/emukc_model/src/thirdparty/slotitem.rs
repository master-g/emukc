use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItem {
	pub api_id: i64,
	pub name: String,
	pub info: String,
	pub buildable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemRemodelInfo {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemRemodelBonus {
	firepower: Vec<i64>,
	night_firepower: Vec<i64>,
	hit: Vec<i64>,
	night_hit: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemExtraInfo {
	pub api_id: i64,
	pub info: String,
}

pub type Kc3rdSlotItemExtraInfoMap = std::collections::BTreeMap<i64, Kc3rdSlotItemExtraInfo>;
