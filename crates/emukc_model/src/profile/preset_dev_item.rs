use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiPresetDevItem, KcApiPresetDevItemElement};

/// Preset dev item element
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetDevItemElement {
	/// index of the item in the preset, starting from 0
	pub index: i64,

	/// name of the item
	pub name: String,

	/// fuel
	pub item1: i64,

	/// ammo
	pub item2: i64,

	/// steel
	pub item3: i64,

	/// bauxite
	pub item4: i64,
}

/// Preset dev item
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetDevItem {
	/// max number of the preset dev item
	pub max_num: i64,

	/// preset dev item elements
	pub records: Vec<PresetDevItemElement>,
}

impl From<PresetDevItemElement> for KcApiPresetDevItemElement {
	fn from(value: PresetDevItemElement) -> Self {
		Self {
			api_preset_no: value.index,
			api_name: value.name,
			api_item1: value.item1,
			api_item2: value.item2,
			api_item3: value.item3,
			api_item4: value.item4,
		}
	}
}

impl From<PresetDevItem> for KcApiPresetDevItem {
	fn from(value: PresetDevItem) -> Self {
		let api_preset_items = if value.records.is_empty() {
			None
		} else {
			Some(value.records.into_iter().map(Into::into).collect())
		};

		Self {
			api_max_num: value.max_num,
			api_preset_items,
		}
	}
}
