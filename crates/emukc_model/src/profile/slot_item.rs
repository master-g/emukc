use serde::{Deserialize, Serialize};

use crate::kc2::KcApiSlotItem;

/// Slot item
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlotItem {
	/// Profile ID
	pub id: i64,

	/// Item instance ID
	pub instance_id: i64,

	/// Item manifest ID
	pub mst_id: i64,

	/// Item locked
	pub locked: bool,

	/// modify level
	pub level: i64,

	/// aircraft level
	pub aircraft_lv: i64,
}

impl From<SlotItem> for KcApiSlotItem {
	fn from(value: SlotItem) -> Self {
		Self {
			api_id: value.instance_id,
			api_slotitem_id: value.mst_id,
			api_locked: if value.locked {
				1
			} else {
				0
			},
			api_level: value.level,
			api_alv: if value.aircraft_lv > 0 {
				Some(value.aircraft_lv)
			} else {
				None
			},
		}
	}
}
