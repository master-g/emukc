use async_trait::async_trait;
use emukc_db::{
	entity::profile::item::slot_item,
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait, TryIntoModel},
};
use emukc_model::{codex::Codex, kc2::KcApiSlotItem};

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

		let am = add_slot_item_impl(&tx, codex, profile_id, mst_id, stars, alv).await?;

		tx.commit().await?;

		Ok(KcApiSlotItem {
			api_id: am.id,
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
	let mst = codex.find_slotitem_mst(mst_id)?;
	add_slot_item_to_picture_book_impl(c, profile_id, mst.api_sortno).await?;

	Ok(model.try_into_model()?)
}
