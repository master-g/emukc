use async_trait::async_trait;
use emukc_db::{
	entity::profile::airbase::{base, plane as plane_db},
	sea_orm::{entity::prelude::*, ActiveValue, QueryOrder, TransactionTrait},
};
use emukc_model::kc2::{KcApiAirBase, KcApiDistance, KcApiPlaneInfo};

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
	async fn get_airbases(&self, profile_id: i64) -> Result<Vec<KcApiAirBase>, GameplayError>;
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

	async fn get_airbases(&self, profile_id: i64) -> Result<Vec<KcApiAirBase>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let (base_models, plane_models) = get_airbases_impl(&tx, profile_id).await?;

		let mut airbases = Vec::new();
		for m in base_models.iter() {
			let api_plane_info = plane_models
				.iter()
				.filter(|p| p.rid == m.rid)
				.map(|p| KcApiPlaneInfo {
					api_count: if p.count > 0 {
						Some(p.count)
					} else {
						None
					},
					api_state: p.state as i64,
					api_cond: if p.condition > 0 {
						Some(p.condition)
					} else {
						None
					},
					api_max_count: if p.max_count > 0 {
						Some(p.max_count)
					} else {
						None
					},
					api_slotid: p.slot_id,
					api_squadron_id: p.squadron_id,
				})
				.collect();

			let base = KcApiAirBase {
				api_action_kind: m.action as i64,
				api_area_id: m.area_id,
				api_distance: KcApiDistance {
					api_base: m.base_range,
					api_bonus: m.bonus_range,
				},
				api_name: m.name.clone(),
				api_plane_info,
				api_rid: m.rid,
			};

			airbases.push(base);
		}

		Ok(airbases)
	}
}

pub async fn unlock_airbase_impl<C>(
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
		name: ActiveValue::Set(format!("\u{7B2C}{}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}", rid)),
		maintenance_level: ActiveValue::Set(1),
	};

	let m = am.insert(c).await?;

	Ok(m)
}

pub async fn get_airbases_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(Vec<base::Model>, Vec<plane_db::Model>), GameplayError>
where
	C: ConnectionTrait,
{
	let models = base::Entity::find()
		.filter(base::Column::ProfileId.eq(profile_id))
		.order_by_asc(base::Column::Id)
		.all(c)
		.await?;

	let plane_models =
		plane_db::Entity::find().filter(plane_db::Column::ProfileId.eq(profile_id)).all(c).await?;

	Ok((models, plane_models))
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
