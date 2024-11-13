//! Ship extension for `Codex`

use crate::{
	fields::MoveValueToEnd,
	kc2::{level, KcApiShip, KcApiSlotItem, KcShipType},
	prelude::{
		ApiMstShip, ApiMstSlotitem, Kc3rdShip, Kc3rdShipPicturebookInfo,
		Kc3rdShipRemodelRequirement,
	},
};

use super::{Codex, CodexError};

impl Codex {
	/// Create a new ship instance.
	///
	/// # Arguments
	///
	/// * `mst_id` - The ship manifest ID.
	pub fn new_ship(&self, mst_id: i64) -> Option<(KcApiShip, Vec<KcApiSlotItem>)> {
		let mst = self.manifest.find_ship(mst_id)?;
		let basic = self.ship_extra.get(&mst_id)?;

		let mut slot_items: Vec<KcApiSlotItem> = Vec::new();
		for slot_info in basic.slots.iter() {
			if slot_info.item_id > 0 {
				slot_items.push(KcApiSlotItem {
					api_id: 0,
					api_slotitem_id: slot_info.item_id,
					api_locked: 0,
					api_level: slot_info.stars,
					api_alv: None,
				});
			}
		}

		let api_lv = basic.remodel.as_ref().map_or(1, |remodel| {
			if remodel.level > 0 {
				remodel.level
			} else {
				1
			}
		});

		let exp_now = if api_lv > 1 {
			level::ship_level_required_exp(api_lv)
		} else {
			0
		};

		let next_exp_required = level::exp_to_ship_level(exp_now).1;

		let api_nowhp = mst.api_taik.as_ref().unwrap()[0];

		let ship = KcApiShip {
			api_id: 0,
			api_sortno: mst.api_sortno.unwrap_or(-1),
			api_ship_id: mst_id,
			api_lv,
			api_exp: [exp_now, next_exp_required, 0],
			api_nowhp,
			api_maxhp: api_nowhp,
			api_soku: mst.api_soku,
			api_leng: mst.api_leng.unwrap_or(-1),
			api_slot: [-1; 5],
			api_onslot: mst.api_maxeq.unwrap_or([0; 5]),
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
			api_sp_effect_items: None,
		};

		Some((ship, slot_items))
	}

	/// Calculate ship status.
	///
	/// # How it works
	///
	/// 1. calculate ship repair status
	/// 2. correct slot items, change 0 to -1, and move -1 to the end
	/// 3. calculate slot item boost, set ship locked equipment
	/// 4. calculate ship status, leveling, married, and srate
	/// 5. apply kyouka values
	///
	/// # Arguments
	///
	/// * `ship` - The ship instance.
	/// * `slot_items` - The slot items.
	pub fn cal_ship_status(
		&self,
		ship: &mut KcApiShip,
		slot_items: &[KcApiSlotItem],
	) -> Result<(), CodexError> {
		let mst = self.find_ship_mst(ship.api_ship_id)?;
		let basic = self.find_ship_extra(ship.api_ship_id)?;

		// recalculating ship repair status
		self.cal_damage_status(mst, ship)?;

		// collect slot items and fix empty slot
		let mut slots: [i64; 6] = [-1; 6];
		for (i, slot) in ship.api_slot.iter().enumerate() {
			if *slot == 0 {
				// fix empty slot
				ship.api_onslot[i] = -1;
			}
			if *slot > 0 {
				slots[i] = *slot;
			}
		}

		// move -1 to the end
		ship.api_onslot.move_value_to_end(-1);
		// collect extra slot as well
		if ship.api_slot_ex > 0 {
			slots[5] = ship.api_slot_ex;
		}

		// calculate slot item boost
		let mut item_souk = 0; // armor
		let mut item_houg = 0; // firepower
		let mut item_raig = 0; // torpedo
		let mut item_soku = 0; // speed
		let mut item_tyku = 0; // anti-air
		let mut item_tais = 0; // anti-sub
		let mut item_houk = 0; // evasion
		let mut item_saku = 0; // los

		// set ship locked equipment
		ship.api_locked_equip = 0;

		for slotitem_id in slots {
			if slotitem_id <= 0 {
				continue;
			}
			let slotitem =
				slot_items.iter().find(|s| s.api_id == slotitem_id).ok_or(CodexError::NotFound(
					format!("slot item id {} not found in slotitems", slotitem_id),
				))?;

			if slotitem.api_locked != 0 && ship.api_locked_equip == 0 {
				ship.api_locked_equip = 1;
			}

			let item_mst = self.find::<ApiMstSlotitem>(&slotitem.api_slotitem_id)?;

			item_souk += item_mst.api_souk;
			item_houg += item_mst.api_houg;
			item_raig += item_mst.api_raig;
			item_soku += item_mst.api_soku;
			item_tyku += item_mst.api_tyku;
			item_tais += item_mst.api_tais;
			item_houk += item_mst.api_houk;
			item_saku += item_mst.api_saku;
		}

		// calculate ship status
		ship.api_maxhp = mst.api_taik.as_ref().unwrap()[0]; // hp
		ship.api_soku = mst.api_soku; // speed
		ship.api_leng = mst.api_leng.unwrap_or(0); // range
		ship.api_karyoku = mst.api_houg.unwrap_or([0, 0]); // fire power
		ship.api_raisou = mst.api_raig.unwrap_or([0, 0]); // torpedo
		ship.api_taiku = mst.api_tyku.unwrap_or([0, 0]); // anti air
		ship.api_soukou = mst.api_souk.unwrap_or([0, 0]); // armor
		basic.kaih.clone_into(&mut ship.api_kaihi); // evasion
		basic.tais.clone_into(&mut ship.api_taisen); // anti submarine
		basic.saku.clone_into(&mut ship.api_sakuteki); // line of sight
		ship.api_lucky = mst.api_luck.unwrap_or([0, 0]); // luck

		// leveling
		let formula = |lv: i64, base: i64, max: i64| -> i64 { ((max - base) * lv / 99) + base };
		ship.api_kaihi[0] = formula(ship.api_lv, basic.kaih[0], basic.kaih[1]);
		ship.api_sakuteki[0] = formula(ship.api_lv, basic.saku[0], basic.saku[1]);
		ship.api_taisen[0] = formula(ship.api_lv, basic.tais[0], basic.tais[1]);

		// married
		if ship.api_lv >= 100 {
			// some exceptional cases are not addressed here
			// see https://en.kancollewiki.net/Marriage
			let min_hp = mst.api_taik.as_ref().unwrap()[0];
			let max_hp = mst.api_taik.as_ref().unwrap()[1];
			let idx = (min_hp as f64 / 10.0).floor() as usize;
			let bonus = [4, 4, 4, 5, 6, 7, 7, 8, 8, 9];
			let bonus = bonus.get(idx).unwrap_or(&0);
			ship.api_maxhp = max_hp.min(min_hp + bonus);
		}

		// srate
		self.cal_srate(mst, ship)?;

		// apply kyouka
		ship.api_karyoku[0] += ship.api_kyouka[0]; // firepower
		ship.api_raisou[0] += ship.api_kyouka[1]; // torpedo
		ship.api_taiku[0] += ship.api_kyouka[2]; // anti-air
		ship.api_soukou[0] += ship.api_kyouka[3]; // armor
		ship.api_lucky[0] += ship.api_kyouka[4]; // luck
		ship.api_maxhp += ship.api_kyouka[5]; // max hp
		ship.api_taisen[0] += ship.api_kyouka[6]; // anti-sub

		// apply slotitem boost
		ship.api_soku += item_soku;
		ship.api_karyoku[0] += item_houg;
		ship.api_raisou[0] += item_raig;
		ship.api_taiku[0] += item_tyku;
		ship.api_soukou[0] += item_souk;
		ship.api_kaihi[0] += item_houk;
		ship.api_taisen[0] += item_tais;
		ship.api_sakuteki[0] += item_saku;

		// apply special effect items
		ship.api_sp_effect_items.iter().flatten().for_each(|v| {
			ship.api_karyoku[0] += v.api_houg.unwrap_or(0);
			ship.api_raisou[0] += v.api_raig.unwrap_or(0);
			ship.api_kaihi[0] += v.api_kaih.unwrap_or(0);
			ship.api_soukou[0] += v.api_souk.unwrap_or(0);
		});

		Ok(())
	}

	/// Calculate ship powerup potentials.
	///
	/// # Arguments
	///
	/// * `ship` - The ship instance.
	/// * `mst` - The ship manifest.
	/// * `basic` - The ship basic information.
	pub fn cal_powerup_potentials(
		&self,
		ship: &KcApiShip,
		mst: Option<&ApiMstShip>,
		basic: Option<&Kc3rdShip>,
	) -> Result<[i64; 7], CodexError> {
		let mst = mst.map_or_else(|| self.find_ship_mst(ship.api_ship_id), Ok)?;

		let basic = basic.map_or_else(|| self.find_ship_extra(ship.api_ship_id), Ok)?;

		let mst_houg = mst.api_houg.unwrap_or([0, 0]);
		let mst_raig = mst.api_raig.unwrap_or([0, 0]);
		let mst_tyku = mst.api_tyku.unwrap_or([0, 0]);
		let mst_souk = mst.api_souk.unwrap_or([0, 0]);
		let mst_luck = mst.api_luck.unwrap_or([0, 0]);
		let mst_tais = basic.tais;

		let mut potentials = [0; 7];
		potentials[0] = mst_houg[1] - mst_houg[0] - ship.api_kyouka[0]; // firepower
		potentials[1] = mst_raig[1] - mst_raig[0] - ship.api_kyouka[1]; // torpedo
		potentials[2] = mst_tyku[1] - mst_tyku[0] - ship.api_kyouka[2]; // anti-air
		potentials[3] = mst_souk[1] - mst_souk[0] - ship.api_kyouka[3]; // armor
		potentials[4] = mst_luck[1] - mst_luck[0] - ship.api_kyouka[4]; // luck
		potentials[5] = 2 - ship.api_kyouka[5]; // max hp
		potentials[6] = if mst_tais[1] > 0 {
			9 + mst_tais[1] - mst_tais[0] - ship.api_kyouka[6] // anti-sub
		} else {
			0
		};

		for p in potentials.iter_mut() {
			if *p < 0 {
				*p = 0;
			}
		}

		Ok(potentials)
	}

	/// Set the ship status after repair.
	///
	/// # Arguments
	///
	/// * `ship` - The ship instance.
	pub fn finish_ship_repair(&self, ship: &mut KcApiShip) {
		ship.api_nowhp = ship.api_maxhp;
		ship.api_ndock_time = 0;
		ship.api_ndock_item = [0, 0];
		if ship.api_cond < 40 {
			ship.api_cond = 40;
		}
	}

	/// Get the ship and its after ships.
	///
	/// # Arguments
	///
	/// * `ship_mst_id` - The ship manifest ID.
	pub fn ship_and_after(&self, ship_mst_id: i64) -> Result<Vec<i64>, CodexError> {
		let mut result = Vec::new();
		let mut stack = vec![ship_mst_id];

		while let Some(id) = stack.pop() {
			let ship_mst = self.find_ship_mst(id)?;

			result.push(id);

			if let Some(after_ship_id_str) = &ship_mst.api_aftershipid {
				if let Ok(after_ship_id) = after_ship_id_str.as_str().parse::<i64>() {
					if after_ship_id == 0 {
						continue;
					}
					if !stack.contains(&after_ship_id) && !result.contains(&after_ship_id) {
						stack.push(after_ship_id);
					}
				} else {
					error!("{:?} has non-integer after_ship_id_str", ship_mst);
				}
			}
		}

		Ok(result)
	}

	/// Get the ship and its before and after ships.
	///
	/// # Arguments
	///
	/// * `ship_mst_id` - The ship manifest ID.
	pub fn ships_before_and_after(&self, ship_mst_id: i64) -> Result<Vec<i64>, CodexError> {
		let mut first_ship_id = ship_mst_id;
		loop {
			let key = first_ship_id.to_string();
			if let Some(before) = self
				.manifest
				.api_mst_ship
				.iter()
				.find(|m| m.api_aftershipid.as_ref().unwrap_or(&"0".to_owned()) == &key)
			{
				first_ship_id = before.api_id;
			} else {
				break;
			}
		}

		self.ship_and_after(first_ship_id)
	}

	/// Get ship picturebook info
	///
	/// # Arguments
	///
	/// * `mst_id` - The ship manifest ID.
	pub fn find_ship_picturebook(
		&self,
		mst_id: i64,
	) -> Result<&Kc3rdShipPicturebookInfo, CodexError> {
		let extra = self.ship_picturebook.get(&mst_id).ok_or(CodexError::NotFound(format!(
			"ship picturebook info id {} not found in thirdparty",
			mst_id
		)))?;

		Ok(extra)
	}

	/// Check if the ship can equip the slot item.
	///
	/// # Arguments
	///
	/// * `ship_mst_id` - The ship manifest ID.
	/// * `slotitem_mst_id` - The slot item manifest ID.
	pub fn can_equip_slotitem(&self, ship_mst_id: i64, slotitem_mst_id: i64) -> bool {
		let Ok(ship_mst) = self.find_ship_mst(ship_mst_id) else {
			error!("ship mst id {} not found", ship_mst_id);
			return false;
		};

		let Ok(slotitem_mst) = self.find::<ApiMstSlotitem>(&slotitem_mst_id) else {
			error!("slotitem mst id {} not found", slotitem_mst_id);
			return false;
		};

		let slotitem_type = slotitem_mst.api_type[2];

		let mst_equip_slot =
			self.manifest.api_mst_equip_ship.iter().find(|m| m.api_ship_id == ship_mst_id);
		let Some(mst_equip_slot) = mst_equip_slot else {
			error!("equip ship mst id {} not found in api_mst_equip_ship", ship_mst_id);
			return false;
		};

		if mst_equip_slot.api_equip_type.contains(&slotitem_type) {
			return true;
		}

		match self.manifest.api_mst_stype.iter().find(|m| m.api_id == ship_mst.api_stype) {
			Some(mst_stype) => {
				let key = slotitem_type.to_string();
				let v = mst_stype.api_equip_type.get(&key);
				v.is_some() && v.unwrap_or(&0) > &0
			}
			None => false,
		}
	}

	/// Get the ship type.
	///
	/// # Arguments
	///
	/// * `mst_id` - The ship manifest ID.
	pub fn get_ship_type(&self, mst_id: i64) -> i64 {
		self.find_ship_mst(mst_id).map(|mst| mst.api_stype).unwrap_or(0)
	}

	/// see <https://en.kancollewiki.net/Modernization>
	fn cal_srate(&self, mst: &ApiMstShip, ship: &mut KcApiShip) -> Result<(), CodexError> {
		let mst_houg = mst.api_houg.unwrap_or([0, 0]);
		let mst_raig = mst.api_raig.unwrap_or([0, 0]);
		let mst_tyku = mst.api_tyku.unwrap_or([0, 0]);
		let mst_souk = mst.api_souk.unwrap_or([0, 0]);

		let total_potentials = mst_houg[1] - mst_houg[0] // firepower
			 + mst_raig[1]
			- mst_raig[0]
			+ mst_tyku[1]
			- mst_tyku[0]
			+ mst_souk[1]
			- mst_souk[0];

		let mut total_kyouka = 0;
		for i in 0..4 {
			total_kyouka += ship.api_kyouka[i];
		}

		// srate is ranged in [0, 4]
		let srate = total_kyouka as f64 / total_potentials as f64 * 5.0;
		ship.api_srate = srate.floor() as i64;

		Ok(())
	}

	fn cal_damage_status(&self, mst: &ApiMstShip, ship: &mut KcApiShip) -> Result<(), CodexError> {
		if ship.api_nowhp > ship.api_maxhp {
			ship.api_nowhp = ship.api_maxhp;
			return Ok(());
		}

		let ship_type = KcShipType::n(mst.api_stype)
			.ok_or(CodexError::NotFound(format!("invalid ship type {}", mst.api_stype)))?;
		let multiplier = match ship_type {
			KcShipType::BB | KcShipType::BBV | KcShipType::CVB | KcShipType::AR => 2.0,
			KcShipType::CA
			| KcShipType::CAV
			| KcShipType::FBB
			| KcShipType::CVL
			| KcShipType::AS => 1.5,
			KcShipType::SS | KcShipType::DE => 0.5,
			_ => 1.0,
		};

		let hp_lost = (ship.api_maxhp - ship.api_nowhp) as f64;
		let mut repair_seconds = if ship.api_lv < 12 {
			hp_lost * (ship.api_lv as f64) * 10.0 * multiplier
		} else {
			hp_lost
				* ((ship.api_lv as f64 * 5.0
					+ (ship.api_lv as f64 - 11.0).sqrt().floor() * 10.0
					+ 50.0) * multiplier)
		};
		repair_seconds += 30.0;

		ship.api_ndock_time = repair_seconds.floor() as i64;

		// materials
		let fuel_cost = ((mst.api_fuel_max.unwrap_or(0) as f64) * hp_lost * 0.032).floor() as i64;
		let steel_cost = ((mst.api_fuel_max.unwrap_or(0) as f64) * hp_lost * 0.06).floor() as i64;

		ship.api_ndock_item = [fuel_cost.max(1), steel_cost.max(1)];

		Ok(())
	}

	fn find_ship_mst(&self, ship_id: i64) -> Result<&ApiMstShip, CodexError> {
		self.manifest
			.find_ship(ship_id)
			.ok_or(CodexError::NotFound(format!("ship manifest ID: {}", ship_id)))
	}

	/// Find the ship extra information.
	///
	/// # Arguments
	///
	/// * `ship_id` - The ship ID.
	pub fn find_ship_extra(&self, ship_id: i64) -> Result<&Kc3rdShip, CodexError> {
		self.ship_extra
			.get(&ship_id)
			.ok_or(CodexError::NotFound(format!("ship extra ID: {}", ship_id)))
	}

	/// Find the ship after the given ship ID.
	///
	/// # Arguments
	///
	/// * `ship_id` - The ship ID.
	pub fn find_ship_after(
		&self,
		ship_id: i64,
	) -> Result<(&ApiMstShip, &Kc3rdShip, &Kc3rdShipRemodelRequirement), CodexError> {
		let ship_mst = self.find_ship_mst(ship_id)?;
		let extra = self.find_ship_extra(ship_id)?;
		if let Some(back_to) = extra.remodel_back_to {
			if back_to > 0 {
				let after_mst = self.find_ship_mst(back_to)?;
				let after_extra = self.find_ship_extra(back_to)?;

				let consumption = extra.remodel_back_requirement.as_ref().ok_or_else(|| {
					CodexError::NotFound(format!(
						"ship remodel back requirement for ID: {} not found",
						ship_id
					))
				})?;

				return Ok((after_mst, after_extra, consumption));
			}
		}

		let after_ship_id = if let Some(after_ship_id) = &ship_mst.api_aftershipid {
			after_ship_id.parse::<i64>().map_err(|_| {
				CodexError::NotFound(format!(
					"ship after ID: {} is not a valid integer",
					after_ship_id
				))
			})?
		} else {
			0
		};

		if after_ship_id > 0 {
			let after_mst = self.find_ship_mst(after_ship_id)?;
			let after_extra = self.find_ship_extra(after_ship_id)?;
			let consumption = after_extra.remodel.as_ref().ok_or_else(|| {
				CodexError::NotFound(format!(
					"ship remodel requirement for ID: {} not found",
					after_ship_id
				))
			})?;

			Ok((after_mst, after_extra, consumption))
		} else {
			Err(CodexError::NotFound(format!("ship after ID: {} not found", after_ship_id)))
		}
	}
}
