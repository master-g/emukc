use async_trait::async_trait;
use emukc_db::{
	entity::profile::item::slot_item,
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait, TryIntoModel},
};
use emukc_model::{
	codex::Codex, kc2::KcApiSlotItem, prelude::ApiMstSlotitem, profile::slot_item::SlotItem,
};

use crate::{err::GameplayError, prelude::HasContext};

use super::picturebook::add_slot_item_to_picture_book_impl;

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
	/// - `instance_id`: The slot item instance ID.
	async fn find_slot_item(
		&self,
		profile_id: i64,
		instance_id: i64,
	) -> Result<KcApiSlotItem, GameplayError>;

	/// Get all slot items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_slot_items(&self, profile_id: i64) -> Result<Vec<KcApiSlotItem>, GameplayError>;

	/// Get all unused slot items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_unuse_slot_items(
		&self,
		profile_id: i64,
	) -> Result<Vec<KcApiSlotItem>, GameplayError>;
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
			api_alv: if alv > 0 {
				Some(alv)
			} else {
				None
			},
		})
	}

	async fn find_slot_item(
		&self,
		profile_id: i64,
		instance_id: i64,
	) -> Result<KcApiSlotItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = find_slot_item_impl(&tx, profile_id, instance_id).await?;
		let slot_item: SlotItem = m.into();

		Ok(slot_item.into())
	}

	async fn get_slot_items(&self, profile_id: i64) -> Result<Vec<KcApiSlotItem>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let ms = get_slot_items_impl(&tx, profile_id).await?;
		let slot_items: Vec<SlotItem> = ms.into_iter().map(std::convert::Into::into).collect();
		let slot_items: Vec<KcApiSlotItem> =
			slot_items.into_iter().map(std::convert::Into::into).collect();

		Ok(slot_items)
	}

	async fn get_unuse_slot_items(
		&self,
		profile_id: i64,
	) -> Result<Vec<KcApiSlotItem>, GameplayError> {
		todo!()
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
	let am = slot_item::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		mst_id: ActiveValue::Set(mst_id),
		locked: ActiveValue::Set(false),
		level: ActiveValue::Set(stars),
		aircraft_lv: ActiveValue::Set(alv),
	};

	let model = am.save(c).await?;

	// add slot item to picture book
	let mst = codex.find::<ApiMstSlotitem>(&mst_id)?;
	add_slot_item_to_picture_book_impl(c, profile_id, mst.api_sortno).await?;

	Ok(model.try_into_model()?)
}

pub async fn find_slot_item_impl<C>(
	c: &C,
	profile_id: i64,
	instance_id: i64,
) -> Result<slot_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.filter(slot_item::Column::Id.eq(instance_id))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"slot item {} not found for profile {}",
				instance_id, profile_id
			))
		})?;

	Ok(record)
}

pub async fn get_slot_items_impl<C>(
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
