use emukc_db::{
	entity::profile::{self},
	sea_orm::{ActiveValue, IntoActiveModel, entity::prelude::*},
};
use emukc_model::kc2::KcApiOssSetting;

use crate::err::GameplayError;

/// Get oss settings of user.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
pub(crate) async fn get_oss_settings_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::settings::oss::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = profile::settings::oss::Entity::find()
		.filter(profile::settings::oss::Column::ProfileId.eq(profile_id))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"oss settings not found for profile {}",
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
	let m =
		profile::settings::oss::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"oss settings not found for profile {}",
				profile_id
			))
		})?;

	let mut am = m.into_active_model();

	let lang = profile::settings::oss::Language::n(lan_type)
		.ok_or_else(|| GameplayError::WrongType(format!("invalid language type {}", lan_type)))?;

	am.language = ActiveValue::Set(lang);
	am.oss_1 = ActiveValue::Set(oss_items[0]);
	am.oss_2 = ActiveValue::Set(oss_items[1]);
	am.oss_3 = ActiveValue::Set(oss_items[2]);
	am.oss_4 = ActiveValue::Set(oss_items[3]);
	am.oss_5 = ActiveValue::Set(oss_items[4]);
	am.oss_6 = ActiveValue::Set(oss_items[5]);
	am.oss_7 = ActiveValue::Set(oss_items[6]);
	am.oss_8 = ActiveValue::Set(oss_items[7]);

	am.update(c).await?;

	Ok(())
}

/// Initialize oss settings of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::settings::oss::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = KcApiOssSetting::default();
	let mut am: profile::settings::oss::ActiveModel = settings.into();

	am.profile_id = ActiveValue::Set(profile_id);
	let m = am.insert(c).await?;

	Ok(m)
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	profile::settings::oss::Entity::delete_by_id(profile_id).exec(c).await?;

	Ok(())
}
