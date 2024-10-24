use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		item::{self},
		ship,
	},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait, TryIntoModel},
};
use emukc_model::profile::picture_book::{PictureBookShip, PictureBookSlotItem};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for picturebook related gameplay.
#[async_trait]
pub trait PictureBookOps {
	/// Add ship record to picture book.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `sortno`: The ship's sort number.
	/// - `damaged`: Whether the ship is damaged.
	/// - `married`: Whether the ship is married.
	async fn add_ship_to_picturebook(
		&self,
		profile_id: i64,
		sortno: i64,
		damaged: Option<bool>,
		married: Option<bool>,
	) -> Result<(), GameplayError>;

	/// Add slot item record to picture book.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `sortno`: The slot item's sort number.
	async fn add_slot_item_to_picturebook(
		&self,
		profile_id: i64,
		sortno: i64,
	) -> Result<(), GameplayError>;

	/// Get picture book of ships.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_ship_picturebook(
		&self,
		profile_id: i64,
	) -> Result<Vec<PictureBookShip>, GameplayError>;

	/// Get picture book of slot items.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_slot_item_picturebook(
		&self,
		profile_id: i64,
	) -> Result<Vec<PictureBookSlotItem>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PictureBookOps for T {
	async fn add_ship_to_picturebook(
		&self,
		profile_id: i64,
		sortno: i64,
		damaged: Option<bool>,
		married: Option<bool>,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		add_ship_to_picturebook_impl(&tx, profile_id, sortno, damaged, married).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn add_slot_item_to_picturebook(
		&self,
		profile_id: i64,
		sortno: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		add_slot_item_to_picturebook_impl(&tx, profile_id, sortno).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn get_ship_picturebook(
		&self,
		profile_id: i64,
	) -> Result<Vec<PictureBookShip>, GameplayError> {
		let db = self.db();

		let records = get_ship_picturebook_impl(db, profile_id).await?;
		let records = records.into_iter().map(std::convert::Into::into).collect();

		Ok(records)
	}

	async fn get_slot_item_picturebook(
		&self,
		profile_id: i64,
	) -> Result<Vec<PictureBookSlotItem>, GameplayError> {
		let db = self.db();

		let records = get_slot_item_picturebook_impl(db, profile_id).await?;
		let records = records.into_iter().map(std::convert::Into::into).collect();

		Ok(records)
	}
}

/// Add ship record to picture book.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `sortno`: The ship's sort number.
/// - `damaged`: Whether the ship is damaged.
/// - `married`: Whether the ship is married.
#[allow(unused)]
pub async fn add_ship_to_picturebook_impl<C>(
	c: &C,
	profile_id: i64,
	sortno: i64,
	damaged: Option<bool>,
	married: Option<bool>,
) -> Result<ship::picturebook::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let mut am = match ship::picturebook::Entity::find()
		.filter(ship::picturebook::Column::ProfileId.eq(profile_id))
		.filter(ship::picturebook::Column::SortNum.eq(sortno))
		.one(c)
		.await?
	{
		Some(record) => ship::picturebook::ActiveModel {
			id: ActiveValue::Unchanged(record.id),
			profile_id: ActiveValue::Unchanged(profile_id),
			sort_num: ActiveValue::Unchanged(sortno),
			damaged: damaged.map_or(ActiveValue::Unchanged(record.damaged), ActiveValue::Set),
			married: married.map_or(ActiveValue::Unchanged(record.married), ActiveValue::Set),
		},
		None => ship::picturebook::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			sort_num: ActiveValue::Set(sortno),
			damaged: ActiveValue::Set(damaged.unwrap_or(false)),
			married: ActiveValue::Set(married.unwrap_or(false)),
		},
	};

	let model = am.save(c).await?;

	Ok(model.try_into_model()?)
}

/// Add slot item record to picture book.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `sortno`: The slot item's sort number.
#[allow(unused)]
pub async fn add_slot_item_to_picturebook_impl<C>(
	c: &C,
	profile_id: i64,
	sortno: i64,
) -> Result<item::picturebook::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let mut am = match item::picturebook::Entity::find()
		.filter(item::picturebook::Column::ProfileId.eq(profile_id))
		.filter(item::picturebook::Column::SortNum.eq(sortno))
		.one(c)
		.await?
	{
		Some(record) => item::picturebook::ActiveModel {
			id: ActiveValue::Unchanged(record.id),
			profile_id: ActiveValue::Unchanged(profile_id),
			sort_num: ActiveValue::Unchanged(sortno),
		},
		None => item::picturebook::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			sort_num: ActiveValue::Set(sortno),
		},
	};

	let model = am.save(c).await?;

	Ok(model.try_into_model()?)
}

pub async fn get_ship_picturebook_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<ship::picturebook::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = ship::picturebook::Entity::find()
		.filter(ship::picturebook::Column::ProfileId.eq(profile_id))
		.all(c)
		.await?;

	Ok(records)
}

pub async fn get_slot_item_picturebook_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<item::picturebook::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = item::picturebook::Entity::find()
		.filter(item::picturebook::Column::ProfileId.eq(profile_id))
		.all(c)
		.await?;

	Ok(records)
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
	ship::picturebook::Entity::delete_many()
		.filter(ship::picturebook::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	item::picturebook::Entity::delete_many()
		.filter(item::picturebook::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
