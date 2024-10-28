use async_trait::async_trait;
use emukc_db::{
	entity::profile::{self},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait},
};
use emukc_model::profile::practice::PracticeConfig;
use emukc_time::{chrono::DateTime, is_before_or_after_jst_today_hour};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for practice related gameplay.
#[async_trait]
pub trait PracticeOps {
	/// Get practice rivals.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_practice_rivals(&self, profile_id: i64) -> Result<Vec<i64>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PracticeOps for T {
	async fn get_practice_rivals(&self, profile_id: i64) -> Result<Vec<i64>, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		todo!()
	}
}

pub async fn get_practice_rivals_impl<C>(c: &C, profile_id: i64) -> Result<Vec<i64>, GameplayError>
where
	C: ConnectionTrait,
{
	let config = profile::practice::config::Entity::find_by_id(profile_id)
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"Practice config not found for profile {}",
				profile_id
			))
		})?;

	let rivals = profile::practice::rival::Entity::find()
		.filter(profile::practice::rival::Column::ProfileId.eq(profile_id))
		.order_by_asc(profile::practice::rival::Column::Index)
		.all(c)
		.await?;

	let should_generate =
		rivals.is_empty() || is_before_or_after_jst_today_hour(config.last_generated, 3, 15);

	todo!()
}

/// Initialize practice of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::practice::config::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let config = PracticeConfig::default();
	let am: profile::practice::config::ActiveModel = profile::practice::config::ActiveModel {
		id: ActiveValue::Set(profile_id),
		selected_type: ActiveValue::Set(config.selected_type.into()),
		generated_type: ActiveValue::Set(config.generated_type.into()),
		last_generated: ActiveValue::Set(DateTime::UNIX_EPOCH),
	};

	let m = am.insert(c).await?;

	Ok(m)
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	profile::practice::config::Entity::delete_by_id(profile_id).exec(c).await?;
	profile::practice::rival::Entity::delete_many()
		.filter(profile::practice::rival::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	profile::practice::detail::Entity::delete_many()
		.filter(profile::practice::detail::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	profile::practice::ship::Entity::delete_many()
		.filter(profile::practice::ship::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
