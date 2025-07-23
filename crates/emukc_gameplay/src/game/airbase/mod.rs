use async_trait::async_trait;
use emukc_db::{
	entity::profile::airbase::{base, plane as plane_db},
	sea_orm::{ActiveValue, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::profile::airbase::Airbase;

use crate::{err::GameplayError, gameplay::HasContext};

mod plane;

/// A trait for airbase related gameplay.
#[async_trait]
pub trait AirbaseOps {
	/// Unlock an airbase.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `area_id`: The area ID.
	/// - `rid`: The airbase ID.
	async fn unlock_airbase(
		&self,
		profile_id: i64,
		area_id: i64,
		rid: i64,
	) -> Result<(), GameplayError>;

	/// Get airbases of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_airbases(&self, profile_id: i64) -> Result<Vec<Airbase>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> AirbaseOps for T {
	async fn unlock_airbase(
		&self,
		profile_id: i64,
		area_id: i64,
		rid: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		unlock_airbase_impl(&tx, profile_id, area_id, rid).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn get_airbases(&self, profile_id: i64) -> Result<Vec<Airbase>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let models = get_airbases_impl(&tx, profile_id).await?;

		let airbases = models
			.iter()
			.map(|v| Airbase {
				id: v.id,
				area_id: v.area_id,
				rid: v.rid,
				action: v.action.into(),
				base_range: v.base_range,
				bonus_range: v.bonus_range,
				name: v.name.clone(),
				maintenance_level: v.maintenance_level,
			})
			.collect();

		Ok(airbases)
	}
}

pub(crate) async fn unlock_airbase_impl<C>(
	c: &C,
	profile_id: i64,
	area_id: i64,
	rid: i64,
) -> Result<base::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let model = base::Entity::find()
		.filter(base::Column::ProfileId.eq(profile_id))
		.filter(base::Column::AreaId.eq(area_id))
		.filter(base::Column::Rid.eq(rid))
		.one(c)
		.await?;

	if let Some(model) = model {
		return Ok(model);
	}

	let am = base::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		area_id: ActiveValue::Set(area_id),
		rid: ActiveValue::Set(rid),
		action: ActiveValue::Set(base::Action::Idle),
		base_range: ActiveValue::Set(0),
		bonus_range: ActiveValue::Set(0),
		name: ActiveValue::Set(format!("\u{7B2C}{rid}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}")),
		maintenance_level: ActiveValue::Set(1),
	};

	let m = am.insert(c).await?;

	Ok(m)
}

pub(crate) async fn get_airbases_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<base::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let models = base::Entity::find()
		.filter(base::Column::ProfileId.eq(profile_id))
		.order_by_asc(base::Column::Id)
		.all(c)
		.await?;

	Ok(models)
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
	base::Entity::delete_many().filter(base::Column::ProfileId.eq(profile_id)).exec(c).await?;
	plane_db::Entity::delete_many()
		.filter(plane_db::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
