use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		self,
		item::slot_item,
		ship::{self, sp_effect_item},
	},
	sea_orm::{entity::prelude::*, ActiveValue, QueryOrder, TransactionTrait, TryIntoModel},
};
use emukc_model::{codex::Codex, kc2::KcApiShip};

use super::{picturebook::add_ship_to_picture_book_impl, slot_item::update_slot_item_impl};
use crate::{err::GameplayError, game::slot_item::add_slot_item_impl, gameplay::HasContext};
use sp::find_ship_sp_effect_items_impl;

mod sp;

/// A trait for material related gameplay.
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
		let tx = db.begin().await?;

		if let Some((ship, sps)) = find_ship_impl(&tx, ship_id).await? {
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
		.ok_or_else(|| GameplayError::ProfileNotFound(profile_id))?;

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
	add_ship_to_picture_book_impl(c, profile_id, ship.api_sortno, None, None).await?;

	Ok((model.try_into_model()?, ship))
}

pub async fn find_ship_impl<C>(
	c: &C,
	ship_id: i64,
) -> Result<Option<(ship::Model, Vec<sp_effect_item::Model>)>, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find().filter(ship::Column::Id.eq(ship_id)).one(c).await?;

	let sp_items = if ship.is_some() {
		find_ship_sp_effect_items_impl(c, ship_id).await?
	} else {
		vec![]
	};

	Ok(ship.map(|s| (s, sp_items)))
}

pub async fn get_ships_impl<C>(
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

	Ok((ships, sp_items))
}

pub(super) async fn init<C>(_c: &C, _profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
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

	Ok(())
}
