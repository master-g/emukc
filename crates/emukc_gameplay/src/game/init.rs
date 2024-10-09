use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::{codex::Codex, profile::furniture::FurnitureConfig};

use crate::err::GameplayError;

use super::{
	furniture::{add_furniture_impl, update_furniture_config_impl},
	kdock::init_kdock_impl,
	material::init_material_impl,
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
	// furniture
	init_furniture(c, profile_id).await?;

	// material
	init_material_impl(c, codex, profile_id).await?;

	// construction docks
	init_kdock_impl(c, profile_id).await?;

	Ok(())
}

async fn init_furniture<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let cfg = FurnitureConfig::default();
	let ids = cfg.api_values();
	for id in ids {
		add_furniture_impl(c, profile_id, id).await?;
	}

	update_furniture_config_impl(c, profile_id, &cfg).await?;

	Ok(())
}
