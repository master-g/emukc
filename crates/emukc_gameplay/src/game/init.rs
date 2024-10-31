use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::codex::Codex;

use crate::err::GameplayError;

use super::{
	airbase, basic, expedition, fleet, furniture, incentive, kdock, map, material, ndock, pay_item,
	picturebook, practice, presets, quest, settings, ship, slot_item, use_item,
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
	// basic
	basic::init(c, profile_id).await?;

	// user game settings
	settings::init(c, profile_id).await?;

	// incentive
	incentive::init(c, profile_id).await?;

	// expedition
	expedition::init(c, profile_id).await?;

	// fleet
	fleet::init(c, profile_id).await?;

	// furniture
	furniture::init(c, profile_id).await?;

	// map
	map::init(c, profile_id).await?;

	// material
	material::init(c, codex, profile_id).await?;

	// construction docks
	kdock::init(c, profile_id).await?;

	// repair docks
	ndock::init(c, profile_id).await?;

	// picture book
	picturebook::init(c, profile_id).await?;

	// quest
	quest::init(c, profile_id).await?;

	// ships
	ship::init(c, profile_id).await?;

	// slot items
	slot_item::init(c, profile_id).await?;

	// use items
	use_item::init(c, profile_id).await?;

	// pay items
	pay_item::init(c, profile_id).await?;

	// practice
	practice::init(c, profile_id).await?;

	// presets
	presets::init(c, profile_id).await?;

	// airbase
	airbase::init(c, profile_id).await?;

	Ok(())
}

pub async fn wipe_profile_game_data<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	basic::wipe(c, profile_id).await?;
	settings::wipe(c, profile_id).await?;
	incentive::wipe(c, profile_id).await?;
	expedition::wipe(c, profile_id).await?;
	fleet::wipe(c, profile_id).await?;
	furniture::wipe(c, profile_id).await?;
	map::wipe(c, profile_id).await?;
	material::wipe(c, profile_id).await?;
	kdock::wipe_kdock_impl(c, profile_id).await?;
	ndock::wipe(c, profile_id).await?;
	picturebook::wipe(c, profile_id).await?;
	quest::wipe(c, profile_id).await?;
	ship::wipe(c, profile_id).await?;
	slot_item::wipe(c, profile_id).await?;
	use_item::wipe(c, profile_id).await?;
	pay_item::wipe(c, profile_id).await?;
	practice::wipe(c, profile_id).await?;
	presets::wipe(c, profile_id).await?;
	airbase::wipe(c, profile_id).await?;

	Ok(())
}
