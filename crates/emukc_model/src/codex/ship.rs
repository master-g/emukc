//! Ship extension for `Codex`

use crate::kc2::{KcApiShip, KcApiSlotItem};

use super::Codex;

impl Codex {
	/// Create a new ship instance.
	///
	/// # Arguments
	///
	/// * `mst_id` - The ship manifest ID.
	pub fn new_ship(&self, mst_id: i64) -> Option<(KcApiShip, Vec<KcApiSlotItem>)> {
		let mst = self.manifest.find_ship(mst_id)?;
		let basic = self.ship_basic.get(&mst_id)?;

		let mut api_onslot = [0; 5];
		for (i, slot) in basic.slots.iter().enumerate() {
			api_onslot[i] = *slot;
		}

		let mut slot_items: Vec<KcApiSlotItem> = Vec::new();
		for equip in basic.equip.iter() {
			let slot_item = KcApiSlotItem {
				api_id: 0,
				api_slotitem_id: equip.api_id,
				api_locked: 0,
				api_level: equip.star,
				api_alv: None,
			};
			slot_items.push(slot_item);
		}

		let api_nowhp = mst.api_taik.as_ref().unwrap()[0];

		let ship = KcApiShip {
			api_id: 0,
			api_sortno: mst.api_sortno.unwrap_or(-1),
			api_ship_id: mst_id,
			api_lv: 1,
			api_exp: [0, 100, 0],
			api_nowhp,
			api_maxhp: api_nowhp,
			api_soku: mst.api_soku,
			api_leng: mst.api_leng.unwrap_or(-1),
			api_slot: [-1; 5],
			api_onslot,
			api_slot_ex: 0,
			api_kyouka: [0; 7],
			api_backs: mst.api_backs.unwrap_or(-1),
			api_fuel: mst.api_fuel_max.unwrap_or(-1),
			api_bull: mst.api_bull_max.unwrap_or(-1),
			api_slotnum: mst.api_slot_num,
			api_ndock_time: 0,
			api_ndock_item: [0, 0],
			api_srate: 0,
			api_cond: 40,
			api_karyoku: mst.api_houg.unwrap_or([0, 0]),
			api_raisou: mst.api_raig.unwrap_or([0, 0]),
			api_taiku: mst.api_tyku.unwrap_or([0, 0]),
			api_soukou: mst.api_souk.unwrap_or([0, 0]),
			api_kaihi: basic.kaih,
			api_taisen: basic.tais,
			api_sakuteki: basic.saku.to_owned(),
			api_lucky: basic.luck.to_owned(),
			api_locked: 0,
			api_locked_equip: 0,
			api_sally_area: 0,
		};

		Some((ship, slot_items))
	}
}
