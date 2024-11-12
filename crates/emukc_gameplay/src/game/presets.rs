use std::collections::BTreeMap;

use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		fleet,
		item::slot_item,
		preset::{
			preset_caps, preset_deck,
			preset_slot::{self, SelectMode},
		},
		ship,
	},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait},
};
use emukc_model::{
	codex::Codex,
	fields::MoveValueToEnd,
	kc2::{KcApiShip, KcApiSlotItem, KcUseItemType},
	prelude::Kc3rdShip,
	profile::{
		preset_deck::{PresetDeck, PresetDeckItem},
		preset_slot::PresetSlot,
	},
};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	fleet::{get_fleets_impl, update_fleet_ships_impl},
	ship::find_ship_impl,
	slot_item::{find_slot_item_impl, get_unset_slot_items_impl},
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

	/// Apply preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	/// - `ship_id`: The ship ID.
	/// - `mode`: The mode.
	async fn apply_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
		mode: i64,
	) -> Result<i64, GameplayError>;

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

	async fn apply_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
		mode: i64,
	) -> Result<i64, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let bauxite =
			apply_preset_slot_impl(&tx, codex, profile_id, preset_no, ship_id, mode).await?;

		tx.commit().await?;

		Ok(bauxite)
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
				locked: ActiveValue::Set(false),
				ex_flag: ActiveValue::Set(false),
				mst_id_1: ActiveValue::Set(0),
				stars_1: ActiveValue::Set(0),
				mst_id_2: ActiveValue::Set(0),
				stars_2: ActiveValue::Set(0),
				mst_id_3: ActiveValue::Set(0),
				stars_3: ActiveValue::Set(0),
				mst_id_4: ActiveValue::Set(0),
				stars_4: ActiveValue::Set(0),
				mst_id_5: ActiveValue::Set(0),
				stars_5: ActiveValue::Set(0),
				mst_id_ex: ActiveValue::Set(0),
				stars_ex: ActiveValue::Set(0),
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

pub(crate) async fn apply_preset_slot_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	preset_no: i64,
	ship_id: i64,
	mode: i64,
) -> Result<i64, GameplayError>
where
	C: ConnectionTrait,
{
	let mode = match mode {
		1 => SelectMode::A,
		2 => SelectMode::B,
		_ => return Err(GameplayError::WrongType(format!("mode {}", mode))),
	};

	// find preset record
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

	// find target ship
	let (ship, _) = find_ship_impl(c, ship_id)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship for ship_id {}", ship_id)))?;

	let extra = codex.find::<Kc3rdShip>(&ship.mst_id)?;

	// undress target ship first
	let on_ship_items =
		[ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex];
	for item_id in on_ship_items.iter().filter(|&&v| v > 0) {
		let item = find_slot_item_impl(c, *item_id).await?;
		let mut am = item.into_active_model();
		am.equip_on = ActiveValue::Set(0);
		am.update(c).await?;
	}

	// find available slot items
	let unused_slot_items = get_unset_slot_items_impl(c, profile_id).await?;
	let slot_item_mst_in_preset: Vec<i64> = [
		record.mst_id_1,
		record.mst_id_2,
		record.mst_id_3,
		record.mst_id_4,
		record.mst_id_5,
		record.mst_id_ex,
	]
	.into_iter()
	.filter(|v| *v > 0)
	.collect();

	let mut unset_mst_lv_lookup: BTreeMap<i64, Vec<&slot_item::Model>> = unused_slot_items
		.iter()
		.filter(|m| slot_item_mst_in_preset.contains(&m.mst_id))
		.fold(BTreeMap::new(), |mut acc, item| {
			acc.entry(item.mst_id).or_default().push(item);
			acc
		});
	// lower level items first
	unset_mst_lv_lookup.iter_mut().for_each(|(_, v)| {
		v.sort_by(|a, b| a.level.cmp(&b.level));
	});

	let mut new_equip_ids = [-1i64; 5];
	for i in 0..(new_equip_ids.len()).min(extra.slots.len()) {
		let (target_mst_id, target_stars) = match i {
			0 => (record.mst_id_1, record.stars_1),
			1 => (record.mst_id_2, record.stars_2),
			2 => (record.mst_id_3, record.stars_3),
			3 => (record.mst_id_4, record.stars_4),
			4 => (record.mst_id_5, record.stars_5),
			_ => unreachable!(),
		};

		let Some(available) = unset_mst_lv_lookup.get(&target_mst_id) else {
			continue;
		};

		if available.is_empty() {
			continue;
		}

		let id = match mode {
			SelectMode::A => {
				// pick the best
				available.iter().last().map(|m| m.id).unwrap_or(-1)
			}
			SelectMode::B => {
				// pick the first one that meets the requirement
				let mut id = -1;
				for item in available.iter() {
					if item.level >= target_stars {
						id = item.id;
						break;
					}
				}
				if id == -1 {
					id = available.last().map(|m| m.id).unwrap_or(-1);
				}
				id
			}
		};

		new_equip_ids[i] = id;
	}
	new_equip_ids.move_value_to_end(-1);

	let ship_has_exslot = ship.slot_ex != 0;
	let (ex_mst_id, ex_level) = (record.mst_id_ex, record.stars_ex);
	let new_ex_id = if let Some(ex_available) = unset_mst_lv_lookup.get(&ex_mst_id) {
		if ex_mst_id > 0 && ship_has_exslot {
			match mode {
				SelectMode::A => {
					// pick the best
					ex_available.iter().last().map(|m| m.id).unwrap_or(-1)
				}
				SelectMode::B => {
					// pick the first one that meets the requirement
					let mut id = -1;
					for item in ex_available.iter() {
						if item.level >= ex_level {
							id = item.id;
							break;
						}
					}
					if id == -1 {
						id = ex_available.last().map(|m| m.id).unwrap_or(-1);
					}
					id
				}
			}
		} else {
			-1
		}
	} else {
		-1
	};

	// apply

	let mut unset_id_lookup: BTreeMap<i64, slot_item::Model> =
		unused_slot_items.into_iter().map(|m| (m.id, m)).collect();

	let mut api_new_ship: KcApiShip = ship.into();
	let mut api_slot_items: Vec<KcApiSlotItem> = Vec::new();

	api_new_ship.api_slot = new_equip_ids;

	if ship_has_exslot && new_ex_id > 0 {
		api_new_ship.api_slot_ex = new_ex_id;

		if let Some(ex_m) = unset_id_lookup.remove(&new_ex_id) {
			api_slot_items.push(ex_m.clone().into());

			let mut am = ex_m.into_active_model();
			am.equip_on = ActiveValue::Set(ship_id);
			am.update(c).await?;
		};
	}

	for item_id in new_equip_ids.iter() {
		if *item_id == -1 {
			continue;
		}

		if let Some(m) = unset_id_lookup.remove(item_id) {
			api_slot_items.push(m.clone().into());

			let mut am = m.into_active_model();
			am.equip_on = ActiveValue::Set(ship_id);
			am.update(c).await?;
		};
	}

	codex.cal_ship_status(&mut api_new_ship, &api_slot_items)?;

	let mut new_ship_am: ship::ActiveModel = api_new_ship.into();
	new_ship_am.id = ActiveValue::Unchanged(ship_id);
	new_ship_am.profile_id = ActiveValue::Unchanged(profile_id);

	new_ship_am.slot_1 = ActiveValue::Set(new_equip_ids[0]);
	new_ship_am.slot_2 = ActiveValue::Set(new_equip_ids[1]);
	new_ship_am.slot_3 = ActiveValue::Set(new_equip_ids[2]);
	new_ship_am.slot_4 = ActiveValue::Set(new_equip_ids[3]);
	new_ship_am.slot_5 = ActiveValue::Set(new_equip_ids[4]);
	if ship_has_exslot && ex_mst_id > 0 {
		new_ship_am.slot_ex = ActiveValue::Set(new_ex_id);
	}

	new_ship_am.update(c).await?;

	Ok(0)
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
