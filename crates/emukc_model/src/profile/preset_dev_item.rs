use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiPresetDevItem, KcApiPresetDevItemElement};

/// Preset dev item element
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetDevItemElement {
	pub index: i64,
	pub name: String,
	pub item1: i64,
	pub item2: i64,
	pub item3: i64,
	pub item4: i64,
}

/// Preset dev item
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetDevItem {
	pub max_num: i64,
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
			Some(value.records.into_iter().map(|r| r.into()).collect())
		};

		Self {
			api_max_num: value.max_num,
			api_preset_items,
		}
	}
}
