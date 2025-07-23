//! Slot item extension for `Codex`

use crate::{
	kc2::{KcApiSlotItem, KcApiUnsetSlot},
	prelude::ApiMstSlotitem,
};

use super::{Codex, CodexError};

impl Codex {
	/// Convert unused slot items to KC API format.
	///
	/// # Parameters
	///
	/// - `items`: The slot items to convert.
	pub fn convert_unused_slot_items_to_api(
		&self,
		items: &[KcApiSlotItem],
	) -> Result<KcApiUnsetSlot, CodexError> {
		let mut unset_slots: KcApiUnsetSlot = KcApiUnsetSlot::new();
		for item in items {
			let mst = self.find::<ApiMstSlotitem>(&item.api_slotitem_id)?;
			let stype = mst.api_type[2];
			let key = format!("api_slottype{stype}");
			if let Some(slots) = unset_slots.get_mut(&key) {
				slots.push(item.api_id);
			} else {
				unset_slots.insert(key, vec![item.api_id]);
			}
		}
		for (_, slots) in unset_slots.iter_mut() {
			slots.sort_unstable();
		}

		Ok(unset_slots)
	}
}
