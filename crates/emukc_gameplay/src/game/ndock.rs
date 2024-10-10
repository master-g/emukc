use async_trait::async_trait;
use emukc_db::{
	entity::profile::ndock,
	sea_orm::{entity::prelude::*, ActiveValue, QueryOrder, TransactionTrait, TryIntoModel},
};
use emukc_model::profile::ndock::RepairDock;

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for repair dock related gameplay.
#[async_trait]
pub trait NDockOps {
	/// Unlock new repair dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The repair dock index, must be one of 2, 3, 4.
	async fn unlock_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError>;

	/// Get single repair dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The repair dock index, must be one of 1, 2, 3, 4.
	async fn get_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError>;

	/// Get all repair docks.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_ndocks(&self, profile_id: i64) -> Result<Vec<RepairDock>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> NDockOps for T {
	async fn unlock_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = unlock_ndock_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn get_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let dock = get_ndock_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(dock)
	}

	async fn get_ndocks(&self, profile_id: i64) -> Result<Vec<RepairDock>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let docks = get_ndocks_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(docks.into_iter().map(std::convert::Into::into).collect())
	}
}

async fn find_dock<C>(c: &C, profile_id: i64, index: i64) -> Result<ndock::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = ndock::Entity::find()
		.filter(ndock::Column::ProfileId.eq(profile_id))
		.filter(ndock::Column::Index.eq(index))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"Repair dock {} not found for profile {}",
				index, profile_id
			))
		})?;

	Ok(dock)
}

/// Unlock new repair dock.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The construction dock index, must be one of 2, 3, 4.
#[allow(unused)]
pub async fn unlock_ndock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<ndock::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = find_dock(c, profile_id, index).await?;

	let mut am: ndock::ActiveModel = dock.into();
	am.status = ActiveValue::Set(ndock::Status::Idle);

	let m = am.save(c).await?;

	Ok(m.try_into_model()?)
}

/// Get single repair dock.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The construction dock index, must be one of 1, 2, 3, 4.
#[allow(unused)]
pub async fn get_ndock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<RepairDock, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = find_dock(c, profile_id, index).await?;

	Ok(dock.into())
}

/// Get all repair docks.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub async fn get_ndocks_impl<C>(c: &C, profile_id: i64) -> Result<Vec<ndock::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let docks: Vec<ndock::Model> = ndock::Entity::find()
		.filter(ndock::Column::ProfileId.eq(profile_id))
		.order_by_asc(ndock::Column::Index)
		.all(c)
		.await?;

	Ok(docks)
}

/// Initialize repair docks for a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init_ndock_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let models: Vec<ndock::ActiveModel> = [1, 2, 3, 4]
		.iter()
		.map(|i| {
			let dock = RepairDock::new(profile_id, *i).unwrap();
			ndock::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(*i),
				status: ActiveValue::Set(dock.status.into()),
				ship_id: ActiveValue::Set(0),
				last_update: ActiveValue::Set(None),
				complete_time: ActiveValue::Set(None),
				fuel: ActiveValue::Set(0),
				steel: ActiveValue::Set(0),
			}
		})
		.collect();

	ndock::Entity::insert_many(models).exec(c).await?;

	Ok(())
}
