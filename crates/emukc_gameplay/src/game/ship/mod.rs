use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		self,
		item::slot_item,
		ship::{self, morale_timer, sp_effect_item},
	},
	sea_orm::{
		ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, TryIntoModel,
		entity::prelude::*,
	},
};
use emukc_model::{
	codex::Codex,
	kc2::{KcApiShip, KcApiSlotItem, KcUseItemType},
};
use emukc_time::chrono::{DateTime, Utc};

use super::{
	picturebook::add_ship_to_picturebook_impl,
	slot_item::{find_slot_items_by_id_impl, update_slot_item_impl},
	use_item::deduct_use_item_impl,
};
use crate::{err::GameplayError, game::slot_item::add_slot_item_impl, gameplay::HasContext};
use sp::find_ship_sp_effect_items_impl;

mod sp;

/// A trait for ship related gameplay.
#[async_trait]
pub trait ShipOps {
	/// Add ship to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The ship manifest ID.
	async fn add_ship(&self, profile_id: i64, mst_id: i64) -> Result<KcApiShip, GameplayError>;

	/// Find a ship by ID.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship ID.
	async fn find_ship(&self, ship_id: i64) -> Result<Option<KcApiShip>, GameplayError>;

	/// Get ships of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_ships(&self, profile_id: i64) -> Result<Vec<KcApiShip>, GameplayError>;

	/// Toggle ship locked status.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship ID.
	async fn toggle_ship_locked(&self, ship_id: i64) -> Result<KcApiShip, GameplayError>;

	/// Open ship ex-slot.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ship_id`: The ship ID.
	async fn open_ship_exslot(
		&self,
		profile_id: i64,
		ship_id: i64,
	) -> Result<KcApiShip, GameplayError>;

	/// Set ex-slot item.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship ID.
	/// - `slot_item_id`: The slot item ID.
	async fn set_exslot_item(&self, ship_id: i64, slot_item_id: i64) -> Result<(), GameplayError>;

	/// Set slot item.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship ID.
	/// - `slot_idx`: The slot index.
	/// - `slot_item_id`: The slot item ID.
	async fn set_slot_item(
		&self,
		ship_id: i64,
		slot_idx: i64,
		slot_item_id: i64,
	) -> Result<(), GameplayError>;

	/// Unset all slots of a ship.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship ID.
	async fn unset_all_slots(&self, ship_id: i64) -> Result<(), GameplayError>;

	/// Update ship.
	///
	/// TODO: this is a temporary implementation.
	///
	/// # Parameters
	///
	/// - `ship`: The ship to update.
	async fn update_ship(&self, ship: &KcApiShip) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> ShipOps for T {
	async fn add_ship(&self, profile_id: i64, mst_id: i64) -> Result<KcApiShip, GameplayError> {
		let codex = self.codex();
		let db = self.db();

		let tx = db.begin().await?;

		let (_, ship) = add_ship_impl(&tx, codex, profile_id, mst_id).await?;

		tx.commit().await?;

		Ok(ship)
	}

	async fn find_ship(&self, ship_id: i64) -> Result<Option<KcApiShip>, GameplayError> {
		let db = self.db();

		if let Some((ship, sps)) = find_ship_impl(db, ship_id).await? {
			let mut m: KcApiShip = ship.into();

			if !sps.is_empty() {
				m.api_sp_effect_items = Some(sps.into_iter().map(Into::into).collect());
			}

			Ok(Some(m))
		} else {
			Ok(None)
		}
	}

	async fn get_ships(&self, profile_id: i64) -> Result<Vec<KcApiShip>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let (ships, sps) = get_ships_impl(&tx, profile_id).await?;

		tx.commit().await?;

		let ships = ships
			.into_iter()
			.zip(sps)
			.map(|(s, sp)| {
				let mut m: KcApiShip = s.into();

				if !sp.is_empty() {
					m.api_sp_effect_items = Some(sp.into_iter().map(Into::into).collect());
				}

				m
			})
			.collect();

		Ok(ships)
	}

	async fn toggle_ship_locked(&self, ship_id: i64) -> Result<KcApiShip, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let ship = toggle_ship_locked_impl(&tx, ship_id).await?;

		tx.commit().await?;

		Ok(ship.into())
	}

	async fn open_ship_exslot(
		&self,
		profile_id: i64,
		ship_id: i64,
	) -> Result<KcApiShip, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let ship = open_ship_exslot_impl(&tx, profile_id, ship_id).await?;

		tx.commit().await?;

		Ok(ship.into())
	}

	async fn set_exslot_item(&self, ship_id: i64, slot_item_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		set_exslot_item_impl(&tx, ship_id, slot_item_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn set_slot_item(
		&self,
		ship_id: i64,
		slot_idx: i64,
		slot_item_id: i64,
	) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		set_slot_item_impl(&tx, codex, ship_id, slot_idx, slot_item_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn unset_all_slots(&self, ship_id: i64) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		unset_all_slots_impl(&tx, codex, ship_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn update_ship(&self, ship: &KcApiShip) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		update_ship_impl(&tx, codex, ship).await?;

		tx.commit().await?;

		Ok(())
	}
}

/// Add ship to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `ship`: The ship to add.
/// - `slots`: The slot items of the ship.
#[allow(unused)]
pub async fn add_ship_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	mst_id: i64,
) -> Result<(ship::Model, KcApiShip), GameplayError>
where
	C: ConnectionTrait,
{
	// create ship and slot items
	let (mut ship, mut slot_items) = codex.new_ship(mst_id).ok_or_else(|| {
		error!("Failed to create ship: {}", mst_id);
		GameplayError::ShipCreationFailed(mst_id)
	})?;

	// check capacity
	let profile = profile::Entity::find_by_id(profile_id)
		.one(c)
		.await?
		.ok_or(GameplayError::ProfileNotFound(profile_id))?;

	let num_ships_owned =
		ship::Entity::find().filter(ship::Column::ProfileId.eq(profile_id)).count(c).await?;

	if num_ships_owned >= profile.max_ship_capacity as u64 {
		return Err(GameplayError::CapacityExceeded(profile.max_ship_capacity));
	}

	let num_slot_items_owned = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.count(c)
		.await?;

	if num_slot_items_owned >= profile.max_equipment_capacity as u64 {
		return Err(GameplayError::CapacityExceeded(profile.max_equipment_capacity));
	};

	// add slot items
	let mut item_ids = [-1; 5];
	for (i, slot_item) in slot_items.iter_mut().enumerate() {
		let m = add_slot_item_impl(
			c,
			codex,
			profile_id,
			slot_item.api_slotitem_id,
			slot_item.api_level,
			slot_item.api_alv.unwrap_or_default(),
		)
		.await?;
		item_ids[i] = m.id;
		slot_item.api_id = item_ids[i];
	}

	ship.api_slot = item_ids;

	// recalculate stats
	codex.cal_ship_status(&mut ship, &slot_items)?;

	// add ship
	let mut am = ship::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		sort_num: ActiveValue::Set(ship.api_sortno),
		mst_id: ActiveValue::Set(ship.api_ship_id),
		level: ActiveValue::Set(ship.api_lv),
		exp_now: ActiveValue::Set(ship.api_exp[0]),
		exp_next: ActiveValue::Set(ship.api_exp[1]),
		exp_progress: ActiveValue::Set(ship.api_exp[2]),
		married: ActiveValue::Set(false),
		locked: ActiveValue::Set(ship.api_locked.eq(&1)),
		backs: ActiveValue::Set(ship.api_backs),
		hp_now: ActiveValue::Set(ship.api_nowhp),
		hp_max: ActiveValue::Set(ship.api_maxhp),
		speed: ActiveValue::Set(ship.api_soku),
		range: ActiveValue::Set(ship.api_leng),
		slot_1: ActiveValue::Set(ship.api_slot[0]),
		slot_2: ActiveValue::Set(ship.api_slot[1]),
		slot_3: ActiveValue::Set(ship.api_slot[2]),
		slot_4: ActiveValue::Set(ship.api_slot[3]),
		slot_5: ActiveValue::Set(ship.api_slot[4]),
		slot_ex: ActiveValue::Set(ship.api_slot_ex),
		onslot_1: ActiveValue::Set(ship.api_onslot[0]),
		onslot_2: ActiveValue::Set(ship.api_onslot[1]),
		onslot_3: ActiveValue::Set(ship.api_onslot[2]),
		onslot_4: ActiveValue::Set(ship.api_onslot[3]),
		onslot_5: ActiveValue::Set(ship.api_onslot[4]),
		mod_firepower: ActiveValue::Set(ship.api_kyouka[0]),
		mod_torpedo: ActiveValue::Set(ship.api_kyouka[1]),
		mod_aa: ActiveValue::Set(ship.api_kyouka[2]),
		mod_armor: ActiveValue::Set(ship.api_kyouka[3]),
		mod_luck: ActiveValue::Set(ship.api_kyouka[4]),
		mod_hp: ActiveValue::Set(ship.api_kyouka[5]),
		mod_asw: ActiveValue::Set(ship.api_kyouka[6]),
		fuel: ActiveValue::Set(ship.api_fuel),
		ammo: ActiveValue::Set(ship.api_bull),
		slot_num: ActiveValue::Set(ship.api_slotnum),
		ndock_time: ActiveValue::Set(ship.api_ndock_time),
		ndock_fuel: ActiveValue::Set(ship.api_ndock_item[0]),
		ndock_steel: ActiveValue::Set(ship.api_ndock_item[1]),
		srate: ActiveValue::Set(ship.api_srate),
		condition: ActiveValue::Set(ship.api_cond),
		firepower_now: ActiveValue::Set(ship.api_karyoku[0]),
		firepower_max: ActiveValue::Set(ship.api_karyoku[1]),
		torpedo_now: ActiveValue::Set(ship.api_raisou[0]),
		torpedo_max: ActiveValue::Set(ship.api_raisou[1]),
		aa_now: ActiveValue::Set(ship.api_taiku[0]),
		aa_max: ActiveValue::Set(ship.api_taiku[1]),
		armor_now: ActiveValue::Set(ship.api_soukou[0]),
		armor_max: ActiveValue::Set(ship.api_soukou[1]),
		evasion_now: ActiveValue::Set(ship.api_kaihi[0]),
		evasion_max: ActiveValue::Set(ship.api_kaihi[1]),
		asw_now: ActiveValue::Set(ship.api_taisen[0]),
		asw_max: ActiveValue::Set(ship.api_taisen[1]),
		los_now: ActiveValue::Set(ship.api_sakuteki[0]),
		los_max: ActiveValue::Set(ship.api_sakuteki[1]),
		luck_now: ActiveValue::Set(ship.api_lucky[0]),
		luck_max: ActiveValue::Set(ship.api_lucky[1]),
		has_locked_euqip: ActiveValue::Set(ship.api_locked_equip.eq(&1)),
		sally_area: ActiveValue::Set(ship.api_sally_area),
	};

	let model = am.save(c).await?;

	ship.api_id = model.id.clone().unwrap();

	// equip slot items
	for item in slot_items {
		update_slot_item_impl(c, item.api_id, None, None, Some(ship.api_id)).await?;
	}

	// add ship to picture book
	add_ship_to_picturebook_impl(c, profile_id, ship.api_sortno, None, None).await?;

	Ok((model.try_into_model()?, ship))
}

pub(crate) async fn find_ship_impl<C>(
	c: &C,
	ship_id: i64,
) -> Result<Option<(ship::Model, Vec<sp_effect_item::Model>)>, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find_by_id(ship_id).one(c).await?;

	let sp_items = if ship.is_some() {
		find_ship_sp_effect_items_impl(c, ship_id).await?
	} else {
		vec![]
	};

	Ok(ship.map(|s| (s, sp_items)))
}

pub(crate) async fn get_ships_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(Vec<ship::Model>, Vec<Vec<sp_effect_item::Model>>), GameplayError>
where
	C: ConnectionTrait,
{
	let ships = ship::Entity::find()
		.filter(ship::Column::ProfileId.eq(profile_id))
		.order_by_asc(ship::Column::Id)
		.all(c)
		.await?;

	let sp_items = ships.load_many(sp_effect_item::Entity, c).await?;

	let ships = {
		// morale
		let timer =
			morale_timer::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
				GameplayError::EntryNotFound(format!(
					"morale timer for profile {profile_id} not found"
				))
			})?;
		let last_time_checked = timer.last_time_regen.unwrap_or(DateTime::UNIX_EPOCH);
		let num_of_3_minutes_passed = (Utc::now() - last_time_checked).num_minutes() / 3;

		debug!("{} mins passed", num_of_3_minutes_passed);

		if num_of_3_minutes_passed > 1 {
			let morale_gain = num_of_3_minutes_passed * 3;
			let mut new_ships = vec![];
			for ship in &ships {
				let new_cond = if ship.condition < 49 {
					(ship.condition + morale_gain).max(49)
				} else {
					ship.condition
				};
				let mut am = ship.into_active_model();
				am.condition = ActiveValue::Set(new_cond);
				let m = am.update(c).await?;
				new_ships.push(m);
			}

			{
				let mut am = timer.into_active_model();
				am.last_time_regen = ActiveValue::Set(Some(Utc::now()));
				am.update(c).await?;
			}

			new_ships
		} else {
			ships
		}
	};

	Ok((ships, sp_items))
}

pub(crate) async fn toggle_ship_locked_impl<C>(
	c: &C,
	ship_id: i64,
) -> Result<ship::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship with id {ship_id} not found")))?;

	let locked = !ship.locked;
	let mut am = ship.into_active_model();
	am.locked = ActiveValue::Set(locked);
	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn open_ship_exslot_impl<C>(
	c: &C,
	profile_id: i64,
	ship_id: i64,
) -> Result<ship::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship with id {ship_id} not found")))?;

	deduct_use_item_impl(c, profile_id, KcUseItemType::ReinforceExpansion as i64, 1).await?;

	let mut am = ship.into_active_model();
	am.slot_ex = ActiveValue::Set(-1);
	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn set_exslot_item_impl<C>(
	c: &C,
	ship_id: i64,
	slot_item_id: i64,
) -> Result<ship::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship with id {ship_id} not found")))?;

	{
		let slot_item_model =
			slot_item::Entity::find_by_id(slot_item_id).one(c).await?.ok_or_else(|| {
				GameplayError::EntryNotFound(format!("slot item with id {slot_item_id} not found"))
			})?;
		let mut am = slot_item_model.into_active_model();

		am.equip_on = ActiveValue::Set(ship_id);

		am.update(c).await?;
	}

	let mut am = ship.into_active_model();
	am.slot_ex = ActiveValue::Set(slot_item_id);
	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn set_slot_item_impl<C>(
	c: &C,
	codex: &Codex,
	ship_id: i64,
	slot_idx: i64,
	slot_item_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// find target ship
	let mut ship = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship with id {ship_id} not found")))?;

	// find slot item to set
	if slot_item_id > 0 {
		let slot_item_model =
			slot_item::Entity::find_by_id(slot_item_id).one(c).await?.ok_or_else(|| {
				GameplayError::EntryNotFound(format!("slot item with id {slot_item_id} not found"))
			})?;

		let mut am = slot_item_model.into_active_model();
		am.equip_on = ActiveValue::Set(ship_id);
		am.update(c).await?;
	}

	// handle unset slot item
	let unset_slot_item_id = match slot_idx {
		0 => {
			let tmp = ship.slot_1;
			ship.slot_1 = slot_item_id;
			tmp
		}
		1 => {
			let tmp = ship.slot_2;
			ship.slot_2 = slot_item_id;
			tmp
		}
		2 => {
			let tmp = ship.slot_3;
			ship.slot_3 = slot_item_id;
			tmp
		}
		3 => {
			let tmp = ship.slot_4;
			ship.slot_4 = slot_item_id;
			tmp
		}
		4 => {
			let tmp = ship.slot_5;
			ship.slot_5 = slot_item_id;
			tmp
		}
		_ => unreachable!(),
	};

	if unset_slot_item_id > 0 {
		let slot_item_model =
			slot_item::Entity::find_by_id(unset_slot_item_id).one(c).await?.ok_or_else(|| {
				GameplayError::EntryNotFound(format!(
					"slot item with id {unset_slot_item_id} not found"
				))
			})?;

		let mut am = slot_item_model.into_active_model();
		am.equip_on = ActiveValue::Set(0);
		am.update(c).await?;
	}

	// recalculate stats
	let am = recalculate_ship_status_with_model(c, codex, &ship).await?;

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn unset_all_slots_impl<C>(
	c: &C,
	codex: &Codex,
	ship_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let mut ship = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship with id {ship_id} not found")))?;

	for m in find_slot_items_by_id_impl(
		c,
		&[ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex],
	)
	.await?
	{
		let mut am = m.into_active_model();
		am.equip_on = ActiveValue::Set(0);
		am.update(c).await?;
	}

	ship.slot_1 = -1;
	ship.slot_2 = -1;
	ship.slot_3 = -1;
	ship.slot_4 = -1;
	ship.slot_5 = -1;
	if ship.slot_ex != 0 {
		ship.slot_ex = -1;
	}

	let am = recalculate_ship_status_with_model(c, codex, &ship).await?;

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn update_ship_impl<C>(
	c: &C,
	codex: &Codex,
	s: &KcApiShip,
) -> Result<ship::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let mut m = ship::Entity::find_by_id(s.api_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!("ship with id {} not found", s.api_id))
	})?;

	m.level = s.api_lv;
	m.exp_now = s.api_exp[0];
	m.exp_next = s.api_exp[1];
	m.exp_progress = s.api_exp[2];
	m.married = s.api_lv > 99;
	m.locked = s.api_locked == 1;
	m.backs = s.api_backs;
	m.hp_now = s.api_nowhp;
	m.hp_max = s.api_maxhp;
	m.speed = s.api_soku;
	m.range = s.api_leng;
	m.slot_1 = s.api_slot[0];
	m.slot_2 = s.api_slot[1];
	m.slot_3 = s.api_slot[2];
	m.slot_4 = s.api_slot[3];
	m.slot_5 = s.api_slot[4];
	m.slot_ex = s.api_slot_ex;
	m.onslot_1 = s.api_onslot[0];
	m.onslot_2 = s.api_onslot[1];
	m.onslot_3 = s.api_onslot[2];
	m.onslot_4 = s.api_onslot[3];
	m.onslot_5 = s.api_onslot[4];
	m.mod_firepower = s.api_kyouka[0];
	m.mod_torpedo = s.api_kyouka[1];
	m.mod_aa = s.api_kyouka[2];
	m.mod_armor = s.api_kyouka[3];
	m.mod_luck = s.api_kyouka[4];
	m.mod_hp = s.api_kyouka[5];
	m.mod_asw = s.api_kyouka[6];
	m.fuel = s.api_fuel;
	m.ammo = s.api_bull;
	m.slot_num = s.api_slotnum;
	m.ndock_time = s.api_ndock_time;
	m.ndock_fuel = s.api_ndock_item[0];
	m.ndock_steel = s.api_ndock_item[1];
	m.srate = s.api_srate;
	m.condition = s.api_cond;
	m.firepower_now = s.api_karyoku[0];
	m.firepower_max = s.api_karyoku[1];
	m.torpedo_now = s.api_raisou[0];
	m.torpedo_max = s.api_raisou[1];
	m.aa_now = s.api_taiku[0];
	m.aa_max = s.api_taiku[1];
	m.armor_now = s.api_soukou[0];
	m.armor_max = s.api_soukou[1];
	m.evasion_now = s.api_kaihi[0];
	m.evasion_max = s.api_kaihi[1];
	m.asw_now = s.api_taisen[0];
	m.asw_max = s.api_taisen[1];
	m.los_now = s.api_sakuteki[0];
	m.los_max = s.api_sakuteki[1];
	m.luck_now = s.api_lucky[0];
	m.luck_max = s.api_lucky[1];
	m.has_locked_euqip = s.api_locked_equip == 1;
	m.sally_area = s.api_sally_area;

	let am = recalculate_ship_status_with_model(c, codex, &m).await?;
	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn recalculate_ship_status_with_model<C>(
	c: &C,
	codex: &Codex,
	ship: &ship::Model,
) -> Result<ship::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let mut am: ship::ActiveModel = (*ship).into();

	// find slot items
	let slot_item_ids: Vec<i64> =
		[ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
			.into_iter()
			.filter(|x| *x > 0)
			.collect();

	let slot_items = find_slot_items_by_id_impl(c, &slot_item_ids).await?;

	// update ship has_locked_euqip
	am.has_locked_euqip = ActiveValue::Set(slot_items.iter().any(|x| x.locked));

	// recalculate stats
	let mut api_ship: KcApiShip = (*ship).into();
	let api_slot_items: Vec<KcApiSlotItem> = slot_items.iter().map(|x| x.clone().into()).collect();

	codex.cal_ship_status(&mut api_ship, &api_slot_items)?;

	// modify ship model
	am.sort_num = ActiveValue::Set(api_ship.api_sortno);
	am.mst_id = ActiveValue::Set(api_ship.api_ship_id);
	am.level = ActiveValue::Set(api_ship.api_lv);
	am.exp_now = ActiveValue::Set(api_ship.api_exp[0]);
	am.exp_next = ActiveValue::Set(api_ship.api_exp[1]);
	am.exp_progress = ActiveValue::Set(api_ship.api_exp[2]);
	am.hp_max = ActiveValue::Set(api_ship.api_maxhp);
	am.hp_now = ActiveValue::Set(api_ship.api_nowhp);
	am.married = ActiveValue::Set(api_ship.api_lv > 99);
	am.backs = ActiveValue::Set(api_ship.api_backs);
	am.hp_now = ActiveValue::Set(api_ship.api_nowhp);
	am.hp_max = ActiveValue::Set(api_ship.api_maxhp);
	am.speed = ActiveValue::Set(api_ship.api_soku);
	am.range = ActiveValue::Set(api_ship.api_leng);
	am.fuel = ActiveValue::Set(api_ship.api_fuel);
	am.ammo = ActiveValue::Set(api_ship.api_bull);
	am.slot_num = ActiveValue::Set(api_ship.api_slotnum);
	am.slot_1 = ActiveValue::Set(api_ship.api_slot[0]);
	am.slot_2 = ActiveValue::Set(api_ship.api_slot[1]);
	am.slot_3 = ActiveValue::Set(api_ship.api_slot[2]);
	am.slot_4 = ActiveValue::Set(api_ship.api_slot[3]);
	am.slot_5 = ActiveValue::Set(api_ship.api_slot[4]);
	am.slot_ex = ActiveValue::Set(api_ship.api_slot_ex);
	am.onslot_1 = ActiveValue::Set(api_ship.api_onslot[0]);
	am.onslot_2 = ActiveValue::Set(api_ship.api_onslot[1]);
	am.onslot_3 = ActiveValue::Set(api_ship.api_onslot[2]);
	am.onslot_4 = ActiveValue::Set(api_ship.api_onslot[3]);
	am.onslot_5 = ActiveValue::Set(api_ship.api_onslot[4]);
	am.mod_firepower = ActiveValue::Set(api_ship.api_kyouka[0]);
	am.mod_torpedo = ActiveValue::Set(api_ship.api_kyouka[1]);
	am.mod_aa = ActiveValue::Set(api_ship.api_kyouka[2]);
	am.mod_armor = ActiveValue::Set(api_ship.api_kyouka[3]);
	am.mod_luck = ActiveValue::Set(api_ship.api_kyouka[4]);
	am.mod_hp = ActiveValue::Set(api_ship.api_kyouka[5]);
	am.mod_asw = ActiveValue::Set(api_ship.api_kyouka[6]);
	am.ndock_time = ActiveValue::Set(api_ship.api_ndock_time);
	am.ndock_fuel = ActiveValue::Set(api_ship.api_ndock_item[0]);
	am.ndock_steel = ActiveValue::Set(api_ship.api_ndock_item[1]);
	am.srate = ActiveValue::Set(api_ship.api_srate);
	am.condition = ActiveValue::Set(api_ship.api_cond);
	am.firepower_now = ActiveValue::Set(api_ship.api_karyoku[0]);
	am.firepower_max = ActiveValue::Set(api_ship.api_karyoku[1]);
	am.torpedo_now = ActiveValue::Set(api_ship.api_raisou[0]);
	am.torpedo_max = ActiveValue::Set(api_ship.api_raisou[1]);
	am.aa_now = ActiveValue::Set(api_ship.api_taiku[0]);
	am.aa_max = ActiveValue::Set(api_ship.api_taiku[1]);
	am.armor_now = ActiveValue::Set(api_ship.api_soukou[0]);
	am.armor_max = ActiveValue::Set(api_ship.api_soukou[1]);
	am.evasion_now = ActiveValue::Set(api_ship.api_kaihi[0]);
	am.evasion_max = ActiveValue::Set(api_ship.api_kaihi[1]);
	am.asw_now = ActiveValue::Set(api_ship.api_taisen[0]);
	am.asw_max = ActiveValue::Set(api_ship.api_taisen[1]);
	am.los_now = ActiveValue::Set(api_ship.api_sakuteki[0]);
	am.los_max = ActiveValue::Set(api_ship.api_sakuteki[1]);
	am.luck_now = ActiveValue::Set(api_ship.api_lucky[0]);
	am.luck_max = ActiveValue::Set(api_ship.api_lucky[1]);
	am.sally_area = ActiveValue::Set(api_ship.api_sally_area);

	Ok(am)
}

pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	{
		morale_timer::ActiveModel {
			id: ActiveValue::Set(profile_id),
			last_time_regen: ActiveValue::Set(None),
		}
		.insert(c)
		.await?;
	}
	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	ship::Entity::delete_many().filter(ship::Column::ProfileId.eq(profile_id)).exec(c).await?;
	sp_effect_item::Entity::delete_many()
		.filter(sp_effect_item::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	morale_timer::Entity::delete_by_id(profile_id).exec(c).await?;

	Ok(())
}
