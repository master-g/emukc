use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiPresetSlot, KcApiPresetSlotElement, KcApiPresetSlotItemElement};

/// Preset slot item slot info
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetSlotItemSlot {
	/// Slot item mst id
	pub mst_id: i64,

	/// Slot item stars
	pub stars: i64,
}

/// Preset slot item select mode
#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum PresetSlotItemSelectMode {
	/// mode A, max level slot item will be select first
	A = 1,

	/// mode B, same level slot item will be select first, and `api_slot_ex_flag` will be took into account
	B = 2,
}

/// Preset slot item
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetSlotItemElement {
	/// Profile id
	pub profile_id: i64,

	/// preset index
	pub index: i64,

	/// preset name
	pub name: String,

	/// select mode, 1 = A, 2 = B
	pub select_mode: PresetSlotItemSelectMode,

	/// preset locked
	pub locked: bool,

	/// ex slot flag
	pub ex_flag: bool,

	/// slot item mst id and stars
	pub slots: Vec<PresetSlotItemSlot>,

	/// ex slot
	pub ex: Option<PresetSlotItemSlot>,
}

/// Preset slot
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetSlot {
	/// Profile id
	pub profile_id: i64,

	/// Max number of records
	pub max_num: i64,

	/// records
	pub records: Vec<PresetSlotItemElement>,
}

impl From<PresetSlot> for KcApiPresetSlot {
	fn from(value: PresetSlot) -> Self {
		Self {
			api_max_num: value.max_num,
			api_preset_items: value.records.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<PresetSlotItemElement> for KcApiPresetSlotElement {
	fn from(value: PresetSlotItemElement) -> Self {
		Self {
			api_preset_no: value.index,
			api_name: value.name,
			api_selected_mode: value.select_mode as i64,
			api_lock_flag: if value.locked {
				1
			} else {
				0
			},
			api_slot_ex_flag: if value.ex_flag {
				1
			} else {
				0
			},
			api_slot_item: value.slots.into_iter().map(Into::into).collect(),
			api_slot_item_ex: value.ex.map(Into::into),
		}
	}
}

impl From<PresetSlotItemSlot> for KcApiPresetSlotItemElement {
	fn from(value: PresetSlotItemSlot) -> Self {
		Self {
			api_id: value.mst_id,
			api_level: value.stars,
		}
	}
}
