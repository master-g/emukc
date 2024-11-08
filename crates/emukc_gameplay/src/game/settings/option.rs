use emukc_db::{
	entity::profile::{self},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel},
};
use emukc_model::kc2::KcApiOptionSetting;

use crate::err::GameplayError;

/// Get option settings of user.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
pub(crate) async fn get_option_settings_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Option<profile::settings::option::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let settings = profile::settings::option::Entity::find()
		.filter(profile::settings::option::Column::ProfileId.eq(profile_id))
		.one(c)
		.await?;

	Ok(settings)
}

pub(crate) async fn update_options_settings_impl<C>(
	c: &C,
	profile_id: i64,
	settings: &KcApiOptionSetting,
) -> Result<profile::settings::option::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m = profile::settings::option::Entity::find_by_id(profile_id).one(c).await?;

	let mut am = if let Some(m) = m {
		m.into_active_model()
	} else {
		profile::settings::option::ActiveModel {
			profile_id: ActiveValue::Set(profile_id),
			..Default::default()
		}
	};

	am.skin_id = ActiveValue::Set(settings.api_skin_id);
	am.bgm_volume = ActiveValue::Set(settings.api_vol_bgm);
	am.se_volume = ActiveValue::Set(settings.api_vol_se);
	am.voice_volume = ActiveValue::Set(settings.api_vol_voice);
	am.v_be_left = ActiveValue::Set(settings.api_v_be_left.eq(&1));
	am.v_duty = ActiveValue::Set(settings.api_v_duty.eq(&1));

	let m = if am.profile_id.is_set() {
		am.update(c).await?
	} else {
		am.insert(c).await?
	};

	Ok(m)
}

/// Initialize option settings of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
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
	profile::settings::option::Entity::delete_by_id(profile_id).exec(c).await?;

	Ok(())
}
