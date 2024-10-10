use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::codex::Codex;

use crate::{err::GameplayError, game::settings::wipe_game_settings_impl};

use super::{
	fleet::{init_fleets_impl, wipe_fleets_impl},
	furniture::{init_furniture_impl, wipe_furniture_impl},
	kdock::init_kdock_impl,
	material::init_material_impl,
	ndock::init_ndock_impl,
	settings::init_game_settings_impl,
};

/// Initialize the profile game data.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `codex`: The codex.
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub async fn init_profile_game_data<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// user game settings
	init_game_settings_impl(c, profile_id).await?;

	// fleet
	init_fleets_impl(c, profile_id).await?;

	// furniture
	init_furniture_impl(c, profile_id).await?;

	// material
	init_material_impl(c, codex, profile_id).await?;

	// construction docks
	init_kdock_impl(c, profile_id).await?;

	// repair docks
	init_ndock_impl(c, profile_id).await?;

	Ok(())
}

pub async fn wipe_profile_game_data<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	wipe_game_settings_impl(c, profile_id).await?;
	wipe_fleets_impl(c, profile_id).await?;
	wipe_furniture_impl(c, profile_id).await?;
	// TODO: more wipe functions here

	Ok(())
}
