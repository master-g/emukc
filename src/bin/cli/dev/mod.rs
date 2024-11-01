use anyhow::Result;
use emukc::prelude::{AccountOps, BasicOps, FleetOps, ProfileOps, ShipOps, SlotItemOps};

use crate::{cfg::AppConfig, state::State};

#[instrument]
pub(super) async fn exec(cfg: &AppConfig) -> Result<()> {
	let game_db_path = cfg.workspace_root.join("emukc.db");
	std::fs::remove_file(&game_db_path)?;

	info!("game database removed");

	let state = State::new(cfg).await?;

	info!("state created");

	let account = state.sign_up("admin", "1234567").await?;
	let profile = state.new_profile(&account.access_token.token, "admin").await?;

	info!("account and profile created");

	let pid = profile.profile.id;

	let session = state.start_game(&account.access_token.token, pid).await?;

	info!("game started");

	init_game_stuffs(&state, pid).await?;

	info!("game stuffs initialized");

	println!("{}", session.session.token);

	let url = format!("http://{}/emukc?api_token={}", cfg.bind, session.session.token);

	open::that(url)?;

	Ok(())
}

async fn init_game_stuffs(state: &State, pid: i64) -> Result<()> {
	state.select_world(pid, 1).await?;

	let ship = state.add_ship(pid, 549).await?;
	state.update_fleet_ships(pid, 1, [ship.api_id, -1, -1, -1, -1, -1]).await?;

	// give two damage control teams
	for api_slotitem_id in [42, 42] {
		state.add_slot_item(pid, api_slotitem_id, 0, 0).await?;
	}

	// update first flag
	state.update_user_first_flag(pid, 1).await?;

	Ok(())
}
