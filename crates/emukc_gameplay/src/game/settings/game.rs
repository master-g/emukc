use emukc_db::{
	entity::profile::{self},
	sea_orm::{ActiveValue, IntoActiveModel, entity::prelude::*},
};
use emukc_model::kc2::KcApiGameSetting;

use crate::err::GameplayError;

/// Get game settings of user.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
pub(crate) async fn get_game_settings_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::settings::game::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = profile::settings::game::Entity::find()
		.filter(profile::settings::game::Column::ProfileId.eq(profile_id))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"game settings not found for profile {profile_id}",
			))
		})?;

	Ok(settings)
}

pub(crate) async fn update_port_bgm_impl<C>(
	c: &C,
	profile_id: i64,
	port_bgm: i64,
) -> Result<profile::settings::game::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m =
		profile::settings::game::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"game settings not found for profile {profile_id}",
			))
		})?;

	let mut am = m.into_active_model();

	am.port_bgm = ActiveValue::Set(port_bgm);
	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn update_flagship_position_impl<C>(
	c: &C,
	profile_id: i64,
	position_id: i64,
) -> Result<profile::settings::game::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m =
		profile::settings::game::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"game settings not found for profile {profile_id}",
			))
		})?;

	let mut am = m.into_active_model();

	am.position_id = ActiveValue::Set(position_id);
	let m = am.update(c).await?;

	Ok(m)
}

pub(crate) async fn update_friendly_fleet_settings_impl<C>(
	c: &C,
	profile_id: i64,
	request: bool,
	typ: i64,
) -> Result<profile::settings::game::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m =
		profile::settings::game::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"game settings not found for profile {profile_id}",
			))
		})?;

	let mut am = m.into_active_model();

	am.friend_fleet_req_flag = ActiveValue::Set(request);
	am.friend_fleet_req_type = ActiveValue::Set(typ);
	let m = am.update(c).await?;

	Ok(m)
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
) -> Result<profile::settings::game::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = KcApiGameSetting::default();
	let mut am: profile::settings::game::ActiveModel = settings.into();

	am.profile_id = ActiveValue::Set(profile_id);
	let m = am.insert(c).await?;

	Ok(m)
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	profile::settings::game::Entity::delete_by_id(profile_id).exec(c).await?;

	Ok(())
}
