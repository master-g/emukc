use async_trait::async_trait;
use emukc_db::{
	entity::profile::{self, fleet},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::kc2::KcApiGameSetting;

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for game settings related gameplay.
#[async_trait]
pub trait GameSettingsOps {
	/// Get user game settings.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_game_settings(&self, profile_id: i64) -> Result<KcApiGameSetting, GameplayError>;

	/// Update user game settings.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `settings`: The game settings.
	async fn update_game_settings(
		&self,
		profile_id: i64,
		settings: &KcApiGameSetting,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> GameSettingsOps for T {
	async fn get_game_settings(&self, profile_id: i64) -> Result<KcApiGameSetting, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let settings = get_game_settings_impl(&tx, profile_id).await?;

		Ok(settings.into())
	}

	async fn update_game_settings(
		&self,
		profile_id: i64,
		settings: &KcApiGameSetting,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_game_settings_impl(&tx, profile_id, settings).await?;

		Ok(())
	}
}

/// Get game settings of user.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
pub async fn get_game_settings_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::setting::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = profile::setting::Entity::find()
		.filter(fleet::Column::ProfileId.eq(profile_id))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"game settings not found for profile {}",
				profile_id
			))
		})?;

	Ok(settings)
}

/// Update game settings of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `settings`: The game settings.
pub async fn update_game_settings_impl<C>(
	c: &C,
	profile_id: i64,
	settings: &KcApiGameSetting,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let _ = profile::setting::Entity::find_by_id(profile_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::ProfileNotFound(profile_id))?;

	let mut am: profile::setting::ActiveModel = settings.clone().into();
	am.profile_id = ActiveValue::Unchanged(profile_id);

	am.save(c).await?;

	Ok(())
}

/// Initialize game settings of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init_game_settings_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::setting::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = KcApiGameSetting::default();
	let mut am: profile::setting::ActiveModel = settings.into();

	am.profile_id = ActiveValue::Set(profile_id);
	let m = am.insert(c).await?;

	Ok(m)
}
