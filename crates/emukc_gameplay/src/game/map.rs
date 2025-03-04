use async_trait::async_trait;
use emukc_db::{
	entity::profile::map_record,
	sea_orm::{QueryOrder, entity::prelude::*},
};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for map related gameplay.
#[async_trait]
pub trait MapOps {
	/// Get map records of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_map_records(
		&self,
		profile_id: i64,
	) -> Result<Vec<map_record::Model>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> MapOps for T {
	async fn get_map_records(
		&self,
		profile_id: i64,
	) -> Result<Vec<map_record::Model>, GameplayError> {
		let db = self.db();

		let records = get_map_records_impl(db, profile_id).await?;

		Ok(records)
	}
}

pub(crate) async fn get_map_records_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<map_record::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = map_record::Entity::find()
		.filter(map_record::Column::ProfileId.eq(profile_id))
		.order_by_asc(map_record::Column::MapId)
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
	map_record::Entity::delete_many()
		.filter(map_record::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
