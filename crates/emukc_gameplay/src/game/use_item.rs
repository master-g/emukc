use crate::err::GameplayError;
use crate::gameplay::HasContext;
use async_trait::async_trait;
use emukc_db::{
	entity::profile::item::use_item::{self, ActiveModel},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::kc2::KcApiUserItem;

/// A trait for use item related gameplay.
#[async_trait]
pub trait UseItemOps {
	/// Add use item to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The use item manifest ID.
	/// - `amount`: The amount of the use item.
	async fn add_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> UseItemOps for T {
	async fn add_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let am = add_use_item_impl(&tx, profile_id, mst_id, amount).await?;

		tx.commit().await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count.unwrap(),
		})
	}
}

/// Add use item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `count`: The count of the item.
#[allow(unused)]
pub async fn add_use_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	amount: i64,
) -> Result<use_item::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let record = use_item::Entity::find()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.filter(use_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;

	let am = match record {
		Some(rec) => ActiveModel {
			id: ActiveValue::Unchanged(rec.id),
			count: ActiveValue::Set(rec.count + amount),
			..rec.into()
		},
		None => use_item::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			mst_id: ActiveValue::Set(mst_id),
			count: ActiveValue::Set(amount),
		},
	};

	let model = am.save(c).await?;

	Ok(model)
}
