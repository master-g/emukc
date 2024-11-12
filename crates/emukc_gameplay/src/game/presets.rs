use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		fleet,
		preset::{
			preset_caps, preset_deck,
			preset_slot::{self, SelectMode},
		},
	},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait},
};
use emukc_model::{
	fields::MoveValueToEnd,
	kc2::KcUseItemType,
	profile::{
		preset_deck::{PresetDeck, PresetDeckItem},
		preset_slot::PresetSlot,
	},
};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	fleet::{get_fleets_impl, update_fleet_ships_impl},
	ship::find_ship_impl,
	slot_item::find_slot_item_impl,
	use_item::deduct_use_item_impl,
};

/// A trait for preset related gameplay.
#[async_trait]
pub trait PresetOps {
	/// Get preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_preset_decks(&self, profile_id: i64) -> Result<PresetDeck, GameplayError>;

	/// Find preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn find_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<PresetDeckItem, GameplayError>;

	/// Get preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_preset_slots(&self, profile_id: i64) -> Result<PresetSlot, GameplayError>;

	/// Register preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset`: The preset deck item.
	async fn register_preset_deck(
		&self,
		profile_id: i64,
		preset: &PresetDeckItem,
	) -> Result<preset_deck::Model, GameplayError>;

	/// Register preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	/// - `ship_id`: The ship ID.
	async fn register_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
	) -> Result<preset_slot::Model, GameplayError>;

	/// Delete preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn delete_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Expand preset deck capacity
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn expand_preset_deck_capacity(&self, profile_id: i64) -> Result<(), GameplayError>;

	/// Expand preset slot capacity
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn expand_preset_slot_capacity(&self, profile_id: i64) -> Result<i64, GameplayError>;

	/// Apply preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `deck_id`: The deck ID.
	/// - `preset_no`: The preset number.
	async fn apply_preset_deck(
		&self,
		profile_id: i64,
		deck_id: i64,
		preset_no: i64,
	) -> Result<fleet::Model, GameplayError>;

	/// Delete preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn delete_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Toggle preset slot ex flag
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn toggle_preset_slot_ex_flag(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Toggle preset slot locked
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn toggle_preset_slot_locked(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Update preset slot name
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	/// - `name`: The new name.
	async fn update_preset_slot_name(
		&self,
		profile_id: i64,
		preset_no: i64,
		name: &str,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PresetOps for T {
	async fn get_preset_decks(&self, profile_id: i64) -> Result<PresetDeck, GameplayError> {
		let db = self.db();

		let (caps, decks) = get_preset_decks_impl(db, profile_id).await?;

		Ok(PresetDeck {
			max_num: caps.deck_limit,
			records: decks.into_iter().map(Into::into).collect(),
		})
	}

	async fn find_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<PresetDeckItem, GameplayError> {
		let db = self.db();

		let model = find_preset_deck_impl(db, profile_id, preset_no).await?;

		Ok(model.into())
	}

	async fn get_preset_slots(&self, profile_id: i64) -> Result<PresetSlot, GameplayError> {
		let db = self.db();

		let (caps, slots) = get_preset_slots_impl(db, profile_id).await?;

		Ok(PresetSlot {
			profile_id,
			max_num: caps.slot_limit,
			records: slots.into_iter().map(Into::into).collect(),
		})
	}

	async fn register_preset_deck(
		&self,
		profile_id: i64,
		preset: &PresetDeckItem,
	) -> Result<preset_deck::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = register_preset_deck_impl(&tx, profile_id, preset).await?;

		tx.commit().await?;

		Ok(m)
	}

	async fn register_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
	) -> Result<preset_slot::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = register_preset_slot_impl(&tx, profile_id, preset_no, ship_id).await?;

		tx.commit().await?;

		Ok(m)
	}

	async fn delete_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		delete_preset_deck_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn expand_preset_deck_capacity(&self, profile_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		expand_preset_deck_capacity_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn expand_preset_slot_capacity(&self, profile_id: i64) -> Result<i64, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let new_cap = expand_preset_slot_capacity_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(new_cap)
	}

	async fn apply_preset_deck(
		&self,
		profile_id: i64,
		deck_id: i64,
		preset_no: i64,
	) -> Result<fleet::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = apply_preset_deck_impl(&tx, profile_id, deck_id, preset_no).await?;

		tx.commit().await?;

		Ok(m)
	}

	async fn delete_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		delete_preset_slot_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn toggle_preset_slot_ex_flag(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		toggle_preset_slot_ex_flag_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn toggle_preset_slot_locked(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		toggle_preset_slot_locked_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn update_preset_slot_name(
		&self,
		profile_id: i64,
		preset_no: i64,
		name: &str,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_preset_slot_name_impl(&tx, profile_id, preset_no, name).await?;

		tx.commit().await?;

		Ok(())
	}
}

pub(crate) async fn get_preset_decks_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(preset_caps::Model, Vec<preset_deck::Model>), GameplayError>
where
	C: ConnectionTrait,
{
	let Some(caps) = preset_caps::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::EntryNotFound(format!(
			"preset_caps for profile_id {}",
			profile_id
		)));
	};

	let decks = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.order_by_asc(preset_deck::Column::Index)
		.all(c)
		.await?;

	Ok((caps, decks))
}

pub(crate) async fn find_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<preset_deck::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_deck for profile_id {} and index {}",
				profile_id, preset_no
			))
		})?;

	Ok(record)
}

pub(crate) async fn get_preset_slots_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(preset_caps::Model, Vec<preset_slot::Model>), GameplayError>
where
	C: ConnectionTrait,
{
	let Some(caps) = preset_caps::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::EntryNotFound(format!(
			"preset_caps for profile_id {}",
			profile_id
		)));
	};

	let slots = preset_slot::Entity::find()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.order_by_asc(preset_slot::Column::Index)
		.all(c)
		.await?;

	Ok((caps, slots))
}

pub(crate) async fn register_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	preset: &PresetDeckItem,
) -> Result<preset_deck::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset.index))
		.one(c)
		.await?;

	let mut am =
		record.map(emukc_db::sea_orm::IntoActiveModel::into_active_model).unwrap_or_else(|| {
			preset_deck::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(preset.index),
				..Default::default()
			}
		});

	am.name = ActiveValue::Set(preset.name.clone());
	am.ship_1 = ActiveValue::Set(preset.ships[0]);
	am.ship_2 = ActiveValue::Set(preset.ships[1]);
	am.ship_3 = ActiveValue::Set(preset.ships[2]);
	am.ship_4 = ActiveValue::Set(preset.ships[3]);
	am.ship_5 = ActiveValue::Set(preset.ships[4]);
	am.ship_6 = ActiveValue::Set(preset.ships[5]);

	let m = match am.id {
		ActiveValue::NotSet => am.insert(c).await?,
		_ => am.update(c).await?,
	};

	Ok(m)
}

async fn process_equip<C>(
	c: &C,
	slot_item_id: i64,
	mst_id_field: &mut ActiveValue<i64>,
	stars_field: &mut ActiveValue<i64>,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	if slot_item_id > 0 {
		let slot_item = find_slot_item_impl(c, slot_item_id).await?;
		*mst_id_field = ActiveValue::Set(slot_item.mst_id);
		*stars_field = ActiveValue::Set(slot_item.level);
	} else {
		*mst_id_field = ActiveValue::Set(0);
		*stars_field = ActiveValue::Set(0);
	}
	Ok(())
}

pub(crate) async fn register_preset_slot_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
	ship_id: i64,
) -> Result<preset_slot::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_slot::Entity::find()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.filter(preset_slot::Column::Index.eq(preset_no))
		.one(c)
		.await?;

	let mut am =
		record.map(emukc_db::sea_orm::IntoActiveModel::into_active_model).unwrap_or_else(|| {
			preset_slot::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(preset_no),
				mode: ActiveValue::Set(SelectMode::A),
				name: ActiveValue::Set(format!("\u{7b2c}{:02}", preset_no)),
				..Default::default()
			}
		});

	let (ship, _) = find_ship_impl(c, ship_id)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship for ship_id {}", ship_id)))?;

	process_equip(c, ship.slot_1, &mut am.mst_id_1, &mut am.stars_1).await?;
	process_equip(c, ship.slot_2, &mut am.mst_id_2, &mut am.stars_2).await?;
	process_equip(c, ship.slot_3, &mut am.mst_id_3, &mut am.stars_3).await?;
	process_equip(c, ship.slot_4, &mut am.mst_id_4, &mut am.stars_4).await?;
	process_equip(c, ship.slot_5, &mut am.mst_id_5, &mut am.stars_5).await?;
	process_equip(c, ship.slot_ex, &mut am.mst_id_ex, &mut am.stars_ex).await?;

	let m = match am.id {
		ActiveValue::NotSet => am.insert(c).await?,
		_ => am.update(c).await?,
	};

	Ok(m)
}

pub(crate) async fn delete_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	preset_deck::Entity::delete_many()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset_no))
		.exec(c)
		.await?;

	Ok(())
}

pub(crate) async fn delete_preset_slot_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	preset_slot::Entity::delete_many()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.filter(preset_slot::Column::Index.eq(preset_no))
		.exec(c)
		.await?;

	Ok(())
}

pub(crate) async fn expand_preset_deck_capacity_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// deduct dock key
	deduct_use_item_impl(c, profile_id, KcUseItemType::DockKey as i64, 1).await?;

	// change deck limit
	let caps = preset_caps::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!("preset_caps for profile_id {}", profile_id))
	})?;

	let new_cap = caps.deck_limit + 1;
	let mut am = caps.into_active_model();
	am.deck_limit = ActiveValue::Set(new_cap);

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn expand_preset_slot_capacity_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<i64, GameplayError>
where
	C: ConnectionTrait,
{
	// deduct dock key
	deduct_use_item_impl(c, profile_id, KcUseItemType::DockKey as i64, 1).await?;

	// change slot limit
	let caps = preset_caps::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!("preset_caps for profile_id {}", profile_id))
	})?;

	let new_cap = caps.slot_limit + 1;
	let mut am = caps.into_active_model();
	am.slot_limit = ActiveValue::Set(new_cap);

	am.update(c).await?;

	Ok(new_cap)
}

pub(crate) async fn apply_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	deck_id: i64,
	preset_no: i64,
) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let preset = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_deck for profile_id {} and index {}",
				profile_id, preset_no
			))
		})?;

	let fleets = get_fleets_impl(c, profile_id).await?;

	let other_fleet_ships: Vec<i64> = fleets
		.iter()
		.filter_map(|f| {
			if f.id == deck_id {
				None
			} else {
				Some(vec![f.ship_1, f.ship_2, f.ship_3, f.ship_4, f.ship_5, f.ship_6])
			}
		})
		.flatten()
		.filter(|&sid| sid != -1)
		.collect();

	let preset_ships =
		[preset.ship_1, preset.ship_2, preset.ship_3, preset.ship_4, preset.ship_5, preset.ship_6];

	let mut new_ship_ids: [i64; 6] = [-1; 6];
	for (i, sid) in preset_ships.iter().enumerate() {
		if *sid == -1 || other_fleet_ships.contains(sid) {
			new_ship_ids[i] = -1;
		} else {
			new_ship_ids[i] = *sid;
		}
	}

	new_ship_ids.move_value_to_end(-1);

	let m = update_fleet_ships_impl(c, profile_id, deck_id, &new_ship_ids).await?;

	Ok(m)
}

pub(crate) async fn toggle_preset_slot_ex_flag_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_slot::Entity::find()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.filter(preset_slot::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_slot for profile_id {} and index {}",
				profile_id, preset_no
			))
		})?;

	let new_mod = match record.mode {
		SelectMode::A => SelectMode::B,
		SelectMode::B => SelectMode::A,
	};

	let mut am = record.into_active_model();
	am.mode = ActiveValue::Set(new_mod);

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn toggle_preset_slot_locked_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_slot::Entity::find()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.filter(preset_slot::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_slot for profile_id {} and index {}",
				profile_id, preset_no
			))
		})?;

	let new_locked = !record.locked;

	let mut am = record.into_active_model();
	am.locked = ActiveValue::Set(new_locked);

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn update_preset_slot_name_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
	name: &str,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_slot::Entity::find()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.filter(preset_slot::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_slot for profile_id {} and index {}",
				profile_id, preset_no
			))
		})?;

	let mut am = record.into_active_model();
	am.name = ActiveValue::Set(name.to_string());

	am.update(c).await?;

	Ok(())
}

pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let caps_am: preset_caps::ActiveModel = preset_caps::ActiveModel {
		id: ActiveValue::set(profile_id),
		deck_limit: ActiveValue::set(3),
		slot_limit: ActiveValue::set(4),
	};

	caps_am.insert(c).await?;
	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	preset_caps::Entity::delete_by_id(profile_id).exec(c).await?;
	preset_deck::Entity::delete_many()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
