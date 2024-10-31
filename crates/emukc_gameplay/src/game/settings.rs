use async_trait::async_trait;
use emukc_db::{
	entity::profile::{self},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, TransactionTrait},
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

	/// Update oss settings.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `lan_type`: The language type.
	/// - `oss_items`: The OSS items.
	async fn update_oss_settings(
		&self,
		profile_id: i64,
		lan_type: i64,
		oss_items: &[i64],
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> GameSettingsOps for T {
	async fn get_game_settings(&self, profile_id: i64) -> Result<KcApiGameSetting, GameplayError> {
		let db = self.db();
		let settings = get_game_settings_impl(db, profile_id).await?;

		Ok(settings.into())
	}

	async fn update_oss_settings(
		&self,
		profile_id: i64,
		lan_type: i64,
		oss_items: &[i64],
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_oss_settings_impl(&tx, profile_id, lan_type, oss_items).await?;

		tx.commit().await?;

		Ok(())
	}
}

/// Get game settings of user.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
pub(crate) async fn get_game_settings_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::setting::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = profile::setting::Entity::find()
		.filter(profile::setting::Column::ProfileId.eq(profile_id))
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

pub(crate) async fn update_oss_settings_impl<C>(
	c: &C,
	profile_id: i64,
	lan_type: i64,
	oss_items: &[i64],
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let m = profile::setting::Entity::find_by_id(profile_id)
		.one(c)
		.await?
		.ok_or(GameplayError::ProfileNotFound(profile_id))?;

	let mut am: profile::setting::ActiveModel = m.into_active_model();
	am.profile_id = ActiveValue::Unchanged(profile_id);

	let language = profile::setting::Language::n(lan_type)
		.ok_or_else(|| GameplayError::WrongType(format!("invalid language type {}", lan_type)))?;

	am.language = ActiveValue::Set(language);
	let oss = [
		&mut am.oss_1,
		&mut am.oss_2,
		&mut am.oss_3,
		&mut am.oss_4,
		&mut am.oss_5,
		&mut am.oss_6,
		&mut am.oss_7,
		&mut am.oss_8,
	];

	for (i, item) in oss_items.iter().enumerate() {
		*oss[i] = ActiveValue::Set(*item);
	}

	am.update(c).await?;

	Ok(())
}

/// Initialize game settings of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(
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

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	profile::setting::Entity::delete_by_id(profile_id).exec(c).await?;

	Ok(())
}
