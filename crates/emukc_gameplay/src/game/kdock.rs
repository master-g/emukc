use async_trait::async_trait;
use emukc_db::{
	entity::profile::kdock,
	sea_orm::{entity::prelude::*, ActiveValue, QueryOrder, TransactionTrait, TryIntoModel},
};
use emukc_model::profile::kdock::ConstructionDock;

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for construction dock related gameplay.
#[async_trait]
pub trait KDockOps {
	/// Unlock new construction dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The construction dock index, must be one of 2, 3, 4.
	async fn unlock_kdock(
		&self,
		profile_id: i64,
		index: i64,
	) -> Result<ConstructionDock, GameplayError>;

	/// Get single construction dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The construction dock index, must be one of 1, 2, 3, 4.
	async fn get_kdock(
		&self,
		profile_id: i64,
		index: i64,
	) -> Result<ConstructionDock, GameplayError>;

	/// Get all construction docks.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_kdocks(&self, profile_id: i64) -> Result<Vec<ConstructionDock>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> KDockOps for T {
	async fn unlock_kdock(
		&self,
		profile_id: i64,
		index: i64,
	) -> Result<ConstructionDock, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = unlock_kdock_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn get_kdock(
		&self,
		profile_id: i64,
		index: i64,
	) -> Result<ConstructionDock, GameplayError> {
		let db = self.db();
		let dock = get_kdock_impl(db, profile_id, index).await?;

		Ok(dock)
	}

	async fn get_kdocks(&self, profile_id: i64) -> Result<Vec<ConstructionDock>, GameplayError> {
		let db = self.db();
		let docks = get_kdocks_impl(db, profile_id).await?;

		Ok(docks.into_iter().map(std::convert::Into::into).collect())
	}
}

pub(super) async fn find_kdock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<kdock::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = kdock::Entity::find()
		.filter(kdock::Column::ProfileId.eq(profile_id))
		.filter(kdock::Column::Index.eq(index))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"Construction dock {} not found for profile {}",
				index, profile_id
			))
		})?;

	Ok(dock)
}

/// Unlock new construction dock.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The construction dock index, must be one of 2, 3, 4.
#[allow(unused)]
pub(crate) async fn unlock_kdock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<kdock::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = find_kdock_impl(c, profile_id, index).await?;

	let mut am: kdock::ActiveModel = dock.into();
	am.status = ActiveValue::Set(kdock::Status::Idle);

	let m = am.save(c).await?;

	Ok(m.try_into_model()?)
}

/// Get single construction dock.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The construction dock index, must be one of 1, 2, 3, 4.
#[allow(unused)]
pub(crate) async fn get_kdock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<ConstructionDock, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = find_kdock_impl(c, profile_id, index).await?;

	Ok(dock.into())
}

/// Get all construction docks.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub(crate) async fn get_kdocks_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<kdock::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let docks: Vec<kdock::Model> = kdock::Entity::find()
		.filter(kdock::Column::ProfileId.eq(profile_id))
		.order_by_asc(kdock::Column::Index)
		.all(c)
		.await?;

	Ok(docks)
}

/// Initialize construction docks for a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let models: Vec<kdock::ActiveModel> = [1, 2, 3, 4]
		.iter()
		.map(|i| {
			let dock = ConstructionDock::new(profile_id, *i).unwrap();
			kdock::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(*i),
				status: ActiveValue::Set(dock.status.into()),
				ship_id: ActiveValue::Set(0),
				complete_time: ActiveValue::Set(None),
				is_large: ActiveValue::Set(false),
				fuel: ActiveValue::Set(0),
				ammo: ActiveValue::Set(0),
				steel: ActiveValue::Set(0),
				bauxite: ActiveValue::Set(0),
				devmat: ActiveValue::Set(0),
			}
		})
		.collect();

	kdock::Entity::insert_many(models).exec(c).await?;

	Ok(())
}

pub(super) async fn wipe_kdock_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	kdock::Entity::delete_many().filter(kdock::Column::ProfileId.eq(profile_id)).exec(c).await?;

	Ok(())
}
