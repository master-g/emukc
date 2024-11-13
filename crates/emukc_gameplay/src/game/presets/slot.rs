use std::collections::BTreeMap;

use emukc_db::{
	entity::profile::{
		item::slot_item,
		preset::{
			preset_caps,
			preset_slot::{self, SelectMode},
		},
		ship,
	},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, QueryOrder},
};
use emukc_model::{
	codex::Codex,
	fields::MoveValueToEnd,
	kc2::{KcApiShip, KcApiSlotItem, KcUseItemType},
	prelude::Kc3rdShip,
};

use crate::{err::GameplayError, game::slot_item::find_slot_item_impl};

use super::{deduct_use_item_impl, get_unset_slot_items_impl};

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

	let ship = ship::Entity::find_by_id(ship_id)
		.one(c)
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
	let ship = ship::Entity::find_by_id(ship_id)
		.one(c)
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

	let mut has_locked_item = false;

	api_new_ship.api_slot = new_equip_ids;

	if ship_has_exslot && new_ex_id > 0 {
		api_new_ship.api_slot_ex = new_ex_id;

		if let Some(ex_m) = unset_id_lookup.remove(&new_ex_id) {
			if ex_m.locked {
				has_locked_item = true;
			}
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
			if m.locked {
				has_locked_item = true;
			}
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
	new_ship_am.has_locked_euqip = ActiveValue::Set(has_locked_item);

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
