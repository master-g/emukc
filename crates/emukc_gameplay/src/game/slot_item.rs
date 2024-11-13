use std::collections::BTreeMap;

use async_trait::async_trait;
use emukc_db::{
	entity::profile::item::slot_item,
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait, TryIntoModel},
};
use emukc_model::{prelude::*, profile::slot_item::SlotItem};

use crate::{err::GameplayError, game::material::add_material_impl, gameplay::HasContext};

use super::picturebook::add_slot_item_to_picturebook_impl;

/// A trait for slot item related gameplay.
#[async_trait]
pub trait SlotItemOps {
	/// Add slot item to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The slot item manifest ID.
	/// - `stars`: The stars of the item.
	/// - `alv`: The aircraft level of the item.
	async fn add_slot_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		stars: i64,
		alv: i64,
	) -> Result<KcApiSlotItem, GameplayError>;

	/// Find slot item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `id`: The slot item instance ID.
	async fn find_slot_item(&self, id: i64) -> Result<KcApiSlotItem, GameplayError>;

	/// Get all slot items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_slot_items(&self, profile_id: i64) -> Result<Vec<KcApiSlotItem>, GameplayError>;

	/// Update slot item.
	///
	/// # Parameters
	///
	/// - `id`: The slot item instance ID.
	/// - `stars`: The stars of the item.
	/// - `alv`: The aircraft level of the item.
	/// - `equip_on`: The ship instance ID the item is equipped on.
	async fn update_slot_item(
		&self,
		id: i64,
		stars: Option<i64>,
		alv: Option<i64>,
		equip_on: Option<i64>,
	) -> Result<KcApiSlotItem, GameplayError>;

	/// Get all unset slot items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_unset_slot_items(
		&self,
		profile_id: i64,
	) -> Result<Vec<KcApiSlotItem>, GameplayError>;

	/// Get unset slot items by types.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `type3`: The item types.
	async fn get_unset_slot_items_by_types(
		&self,
		profile_id: i64,
		type3: &[i64],
	) -> Result<BTreeMap<i64, Vec<i64>>, GameplayError>;

	/// Toggle slot item locked status.
	///
	/// for now (5.9.4.0) this can only lock or unlock the item that is not equipped on any ship.
	///
	/// # Parameters
	///
	/// - `item_id`: The slot item instance ID.
	async fn toggle_slot_item_locked(&self, item_id: i64) -> Result<KcApiSlotItem, GameplayError>;

	/// Destroy slot items.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `item_ids`: The slot item instance IDs.
	async fn destroy_items(
		&self,
		profile_id: i64,
		item_ids: &[i64],
	) -> Result<Vec<(MaterialCategory, i64)>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> SlotItemOps for T {
	async fn add_slot_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		stars: i64,
		alv: i64,
	) -> Result<KcApiSlotItem, GameplayError> {
		let codex = self.codex();

		let db = self.db();
		let tx = db.begin().await?;

		let m = add_slot_item_impl(&tx, codex, profile_id, mst_id, stars, alv).await?;

		tx.commit().await?;

		Ok(KcApiSlotItem {
			api_id: m.id,
			api_slotitem_id: mst_id,
			api_locked: 0,
			api_level: stars,
			api_alv: (alv > 0).then_some(alv),
		})
	}

	async fn find_slot_item(&self, id: i64) -> Result<KcApiSlotItem, GameplayError> {
		let db = self.db();
		let m = find_slot_item_impl(db, id).await?;
		let slot_item: SlotItem = m.into();

		Ok(slot_item.into())
	}

	async fn get_slot_items(&self, profile_id: i64) -> Result<Vec<KcApiSlotItem>, GameplayError> {
		let db = self.db();
		let ms = get_slot_items_impl(db, profile_id).await?;

		let slot_items: Vec<SlotItem> = ms.into_iter().map(std::convert::Into::into).collect();
		let slot_items: Vec<KcApiSlotItem> =
			slot_items.into_iter().map(std::convert::Into::into).collect();

		Ok(slot_items)
	}

	async fn update_slot_item(
		&self,
		id: i64,
		stars: Option<i64>,
		alv: Option<i64>,
		equip_on: Option<i64>,
	) -> Result<KcApiSlotItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = update_slot_item_impl(&tx, id, stars, alv, equip_on).await?;

		tx.commit().await?;

		Ok(KcApiSlotItem {
			api_id: m.id,
			api_slotitem_id: m.mst_id,
			api_locked: m.locked as i64,
			api_level: m.level,
			api_alv: (m.aircraft_lv > 0).then_some(m.aircraft_lv),
		})
	}

	async fn get_unset_slot_items(
		&self,
		profile_id: i64,
	) -> Result<Vec<KcApiSlotItem>, GameplayError> {
		let db = self.db();
		let ms = get_unset_slot_items_impl(db, profile_id).await?;

		let slot_items: Vec<SlotItem> = ms.into_iter().map(std::convert::Into::into).collect();
		let slot_items: Vec<KcApiSlotItem> =
			slot_items.into_iter().map(std::convert::Into::into).collect();

		Ok(slot_items)
	}

	async fn get_unset_slot_items_by_types(
		&self,
		profile_id: i64,
		type3: &[i64],
	) -> Result<BTreeMap<i64, Vec<i64>>, GameplayError> {
		let db = self.db();

		let item_ids = get_unset_slot_items_by_types_impl(db, profile_id, type3).await?;

		Ok(item_ids)
	}

	async fn toggle_slot_item_locked(&self, item_id: i64) -> Result<KcApiSlotItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = toggle_slot_item_locked_impl(&tx, item_id).await?;
		let m: SlotItem = m.into();

		tx.commit().await?;

		Ok(m.into())
	}

	async fn destroy_items(
		&self,
		profile_id: i64,
		item_ids: &[i64],
	) -> Result<Vec<(MaterialCategory, i64)>, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let scrapped_materials = destroy_items_impl(&tx, codex, profile_id, item_ids).await?;

		Ok(scrapped_materials)
	}
}

/// Add slot item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `sortno`: The item's sort number.
/// - `stars`: The stars of the item.
/// - `alv`: The aircraft level of the item.
#[allow(unused)]
pub async fn add_slot_item_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	mst_id: i64,
	stars: i64,
	alv: i64,
) -> Result<slot_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let mst = codex.find::<ApiMstSlotitem>(&mst_id)?;
	let am = slot_item::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		mst_id: ActiveValue::Set(mst_id),
		type3: ActiveValue::Set(mst.api_type[2]),
		locked: ActiveValue::Set(false),
		level: ActiveValue::Set(stars),
		aircraft_lv: ActiveValue::Set(alv),
		equip_on: ActiveValue::Set(0),
	};

	let model = am.save(c).await?;

	// add slot item to picture book
	add_slot_item_to_picturebook_impl(c, profile_id, mst.api_sortno).await?;

	Ok(model.try_into_model()?)
}

pub async fn find_slot_item_impl<C>(c: &C, id: i64) -> Result<slot_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = slot_item::Entity::find()
		.filter(slot_item::Column::Id.eq(id))
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("slot item {} not found", id)))?;

	Ok(record)
}

pub async fn find_slot_items_by_id_impl<C>(
	c: &C,
	ids: &[i64],
) -> Result<Vec<slot_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = slot_item::Entity::find()
		.filter(slot_item::Column::Id.is_in(ids.to_owned()))
		.all(c)
		.await?;

	Ok(records)
}

pub(crate) async fn update_slot_item_impl<C>(
	c: &C,
	id: i64,
	stars: Option<i64>,
	alv: Option<i64>,
	equip_on: Option<i64>,
) -> Result<slot_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let model = slot_item::Entity::find()
		.filter(slot_item::Column::Id.eq(id))
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("slot item {} not found", id)))?;

	let mut am: slot_item::ActiveModel = model.into();

	if let Some(stars) = stars {
		am.level = ActiveValue::Set(stars);
	}

	if let Some(alv) = alv {
		am.aircraft_lv = ActiveValue::Set(alv);
	}

	if let Some(equip_on) = equip_on {
		am.equip_on = ActiveValue::Set(equip_on);
	}

	let m = am.save(c).await?;

	Ok(m.try_into_model()?)
}

pub(crate) async fn get_slot_items_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<slot_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.all(c)
		.await?;

	Ok(records)
}

pub(crate) async fn get_unset_slot_items_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<slot_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.filter(slot_item::Column::EquipOn.eq(0))
		.all(c)
		.await?;

	Ok(records)
}

pub(crate) async fn get_unset_slot_items_by_types_impl<C>(
	c: &C,
	profile_id: i64,
	type3: &[i64],
) -> Result<BTreeMap<i64, Vec<i64>>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.filter(slot_item::Column::EquipOn.eq(0))
		.filter(slot_item::Column::Type3.is_in(type3.to_owned()))
		.all(c)
		.await?;

	let mut map: BTreeMap<i64, Vec<i64>> = BTreeMap::new();

	records.iter().for_each(|record| {
		map.entry(record.type3).or_default().push(record.id);
	});

	Ok(map)
}

pub(crate) async fn toggle_slot_item_locked_impl<C>(
	c: &C,
	item_id: i64,
) -> Result<slot_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = slot_item::Entity::find()
		.filter(slot_item::Column::Id.eq(item_id))
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("slot item {} not found", item_id)))?;

	let locked = record.locked;
	let mut am: slot_item::ActiveModel = record.into();

	am.locked = ActiveValue::Set(!locked);

	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn destroy_items_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	item_ids: &[i64],
) -> Result<Vec<(MaterialCategory, i64)>, GameplayError>
where
	C: ConnectionTrait,
{
	let mut scrap_materials = vec![
		(MaterialCategory::Fuel, 0),
		(MaterialCategory::Ammo, 0),
		(MaterialCategory::Steel, 0),
		(MaterialCategory::Bauxite, 0),
	];

	let items = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.filter(slot_item::Column::Id.is_in(item_ids.to_owned()))
		.all(c)
		.await?;

	for item in items {
		let mst = codex.find::<ApiMstSlotitem>(&item.mst_id)?;

		mst.api_broken.iter().enumerate().for_each(|(i, v)| {
			scrap_materials[i].1 += v;
		});

		item.delete(c).await?;
	}

	add_material_impl(c, codex, profile_id, &scrap_materials).await?;

	Ok(scrap_materials)
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
	slot_item::Entity::delete_many()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_bool_to_i64() {
		assert_eq!(true as i64, 1);
		assert_eq!(false as i64, 0);
	}
}
