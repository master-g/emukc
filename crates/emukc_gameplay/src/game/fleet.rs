use async_trait::async_trait;
use emukc_db::{
	entity::profile::fleet,
	sea_orm::{entity::prelude::*, QueryOrder, TransactionTrait, TryIntoModel},
};
use emukc_model::profile::fleet::Fleet;

use crate::{err::GameplayError, prelude::HasContext};

/// A trait for fleet related gameplay.
#[async_trait]
pub trait FleetOps {
	/// Unlock new deck port.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The fleet index, must be one of 2, 3, 4.
	async fn unlock_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError>;

	/// Get single deck port.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
	async fn get_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError>;

	/// Get all deck ports.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_fleets(&self, profile_id: i64) -> Result<Vec<Fleet>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> FleetOps for T {
	async fn unlock_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = unlock_fleet_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn get_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let fleet = get_fleet_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(fleet)
	}

	async fn get_fleets(&self, profile_id: i64) -> Result<Vec<Fleet>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let fleets = get_fleets_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(fleets.into_iter().map(std::convert::Into::into).collect())
	}
}

async fn find_fleet<C>(c: &C, profile_id: i64, index: i64) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let Some(fleet) = fleet::Entity::find()
		.filter(fleet::Column::ProfileId.eq(profile_id))
		.filter(fleet::Column::Index.eq(index))
		.one(c)
		.await?
	else {
		return Err(GameplayError::EntryNotFound(format!(
			"fleet {} not found for profile {}",
			index, profile_id
		)));
	};

	Ok(fleet)
}

/// Unlock new deck port.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The fleet index, must be one of 2, 3, 4.
#[allow(unused)]
pub async fn unlock_fleet_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = Fleet::new(profile_id, index).unwrap();

	let mut am: fleet::ActiveModel = fleet.into();

	let m = am.save(c).await?;

	Ok(m.try_into_model()?)
}

/// Get single fleet.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
#[allow(unused)]
pub async fn get_fleet_impl<C>(c: &C, profile_id: i64, index: i64) -> Result<Fleet, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = find_fleet(c, profile_id, index).await?;

	Ok(fleet.into())
}

/// Get all deck ports.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub async fn get_fleets_impl<C>(c: &C, profile_id: i64) -> Result<Vec<fleet::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let fleets: Vec<fleet::Model> = fleet::Entity::find()
		.filter(fleet::Column::ProfileId.eq(profile_id))
		.order_by_asc(fleet::Column::Index)
		.all(c)
		.await?;

	Ok(fleets)
}

/// Initialize deck ports for a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub async fn init_fleets_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	unlock_fleet_impl(c, profile_id, 1).await?;

	Ok(())
}