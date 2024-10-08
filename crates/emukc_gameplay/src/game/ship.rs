use async_trait::async_trait;
use emukc_db::{
	entity::profile::{self, item::slot_item, ship},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::kc2::{KcApiShip, KcApiSlotItem};

use crate::{err::GameplayError, game::slot_item::add_slot_item_impl, prelude::HasContext};

/// A trait for material related gameplay.
#[async_trait]
pub trait ShipOps {
	/// Add ship to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The ship manifest ID.
	async fn add_ship(&self, profile_id: i64, mst_id: i64) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> ShipOps for T {
	async fn add_ship(&self, profile_id: i64, mst_id: i64) -> Result<(), GameplayError> {
		let codex = self.codex();

		let db = self.db();
		let tx = db.begin().await?;

		let Some((ship, slot_items)) = codex.new_ship(mst_id) else {
			error!("Failed to create ship: {}", mst_id);
			return Err(GameplayError::ShipCreationFailed(mst_id));
		};

		add_ship_impl(&tx, profile_id, &ship, &slot_items).await?;

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
	profile_id: i64,
	ship: &KcApiShip,
	slots: &[KcApiSlotItem],
) -> Result<ship::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let Some(profile) = profile::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::ProfileNotFound(profile_id));
	};

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
	for (i, slot_item) in slots.iter().enumerate() {
		let m = add_slot_item_impl(c, profile_id, slot_item.api_slotitem_id, slot_item.api_level)
			.await?;
		item_ids[i] = m.id.unwrap();
	}

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
		slot_1: ActiveValue::Set(item_ids[0]),
		slot_2: ActiveValue::Set(item_ids[1]),
		slot_3: ActiveValue::Set(item_ids[2]),
		slot_4: ActiveValue::Set(item_ids[3]),
		slot_5: ActiveValue::Set(item_ids[4]),
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

	Ok(model)
}
