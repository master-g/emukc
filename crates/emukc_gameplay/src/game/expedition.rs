use async_trait::async_trait;
use emukc_db::{
	entity::profile::expedition,
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, TransactionTrait},
};
use emukc_time::{chrono::Utc, KcTime};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for expedition(mission) related gameplay.
#[async_trait]
pub trait ExpeditionOps {
	/// Get all expedition records of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_expeditions(
		&self,
		profile_id: i64,
	) -> Result<(Vec<expedition::Model>, Option<i64>), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> ExpeditionOps for T {
	/// Get all expedition records of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_expeditions(
		&self,
		profile_id: i64,
	) -> Result<(Vec<expedition::Model>, Option<i64>), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let records = expedition::Entity::find()
			.filter(expedition::Column::ProfileId.eq(profile_id))
			.all(&tx)
			.await?;

		let first_day_of_the_month = KcTime::jst_0500_of_nth_day(1);
		let now = Utc::now();

		let mut next_refresh_time: Option<i64> = None;
		let mut result: Vec<expedition::Model> = Vec::new();

		for record in records {
			if let expedition::Status::Completed = record.state {
				if let Some(last_completed_at) = record.last_completed_at {
					if codex
						.manifest
						.api_mst_mission
						.iter()
						.any(|v| v.api_id == record.mission_id && v.api_reset_type == 1)
					{
						// is monthly expedition, check needs to reset
						if now > first_day_of_the_month
							&& last_completed_at < first_day_of_the_month
						{
							// needs to reset
							let mut am = record.into_active_model();
							am.state = ActiveValue::Set(expedition::Status::NotStarted);
							am.last_completed_at = ActiveValue::Set(None);

							let m = am.update(&tx).await?;

							result.push(m);
							continue;
						} else if next_refresh_time.is_none() {
							// the first day of next month is not reached yet
							next_refresh_time = Some(
								first_day_of_the_month.timestamp_millis() - now.timestamp_millis(),
							);
						}
					}
				}
			}

			result.push(record);
		}

		tx.commit().await?;

		Ok((result, next_refresh_time))
	}
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
	expedition::Entity::delete_many()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
