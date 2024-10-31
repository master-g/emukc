use crate::err::GameplayError;
use crate::gameplay::HasContext;
use async_trait::async_trait;
use emukc_db::{
	entity::profile::item::pay_item::{self, ActiveModel},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait, TryIntoModel},
};
use emukc_model::{prelude::*, profile::user_item::UserItem};

/// A trait for pay item related gameplay.
#[async_trait]
pub trait PayItemOps {
	/// Add pay item to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The pay item manifest ID.
	/// - `amount`: The amount of the pay item.
	async fn add_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Find pay item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The pay item manifest ID.
	async fn find_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Get all pay items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_pay_items(&self, profile_id: i64) -> Result<Vec<KcApiUserItem>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PayItemOps for T {
	async fn add_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let am = add_pay_item_impl(&tx, profile_id, mst_id, amount).await?;

		tx.commit().await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn find_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let am = find_pay_item_impl(db, profile_id, mst_id).await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn get_pay_items(&self, profile_id: i64) -> Result<Vec<KcApiUserItem>, GameplayError> {
		let db = self.db();
		let items = get_pay_items_impl(db, profile_id).await?;

		let items: Vec<UserItem> = items.into_iter().map(std::convert::Into::into).collect();
		let items: Vec<KcApiUserItem> = items.into_iter().map(std::convert::Into::into).collect();

		Ok(items)
	}
}

/// Add pay item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `count`: The count of the item.
#[allow(unused)]
pub(crate) async fn add_pay_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	amount: i64,
) -> Result<pay_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = pay_item::Entity::find()
		.filter(pay_item::Column::ProfileId.eq(profile_id))
		.filter(pay_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;

	let am = match record {
		Some(rec) => ActiveModel {
			id: ActiveValue::Unchanged(rec.id),
			count: ActiveValue::Set(rec.count + amount),
			..rec.into()
		},
		None => pay_item::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			mst_id: ActiveValue::Set(mst_id),
			count: ActiveValue::Set(amount),
		},
	};

	let model = am.save(c).await?;

	Ok(model.try_into_model()?)
}

/// Find pay item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
#[allow(unused)]
pub(crate) async fn find_pay_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
) -> Result<pay_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m = pay_item::Entity::find()
		.filter(pay_item::Column::ProfileId.eq(profile_id))
		.filter(pay_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;
	let m = m.unwrap_or(pay_item::Model {
		id: 0,
		profile_id,
		mst_id,
		count: 0,
	});
	Ok(m)
}

pub(crate) async fn get_pay_items_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<pay_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let items =
		pay_item::Entity::find().filter(pay_item::Column::ProfileId.eq(profile_id)).all(c).await?;
	Ok(items)
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
	pay_item::Entity::delete_many()
		.filter(pay_item::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
