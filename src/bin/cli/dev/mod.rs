use anyhow::Result;
use emukc::{
	model::profile::furniture::FurnitureConfig,
	prelude::{
		AccountOps, BasicOps, FleetOps, FurnitureOps, HasContext, IncentiveOps, KcApiIncentiveItem,
		KcUseItemType, MaterialCategory, MaterialOps, ProfileOps, ShipOps, SlotItemOps, UseItemOps,
	},
};

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

	Ok(())
}

async fn add_incentives(state: &State, pid: i64) -> Result<()> {
	let codex = state.codex();

	let ship_incentives: Vec<KcApiIncentiveItem> = [
		(184, "大鯨"),
		// (186, "時津風"),
		(187, "明石改"),
		// (299, "Scamp"),
		(433, "Saratoga"),
		// (892, "Drum"),
		// (927, "Valiant"),
		(951, "天津風改二"),
		// (952, "Phoenix"),
		// (964, "白雲"),
	]
	.iter()
	.filter_map(|(sid, _)| codex.new_incentive_with_ship(*sid).ok())
	.collect();

	state.add_incentive(pid, &ship_incentives).await?;

	// modify ships for testing `api_req_hokyu/charge`

	let mut ships = state.get_ships(pid).await?;

	for ship in ships.iter_mut() {
		ship.api_fuel = 0;
		ship.api_bull = 0;
		ship.api_onslot = [0, 0, 0, 0, 0];
		ship.api_lv = 99;

		state.update_ship(ship).await?;
	}

	Ok(())
}

async fn init_game_stuffs(state: &State, pid: i64) -> Result<()> {
	state.select_world(pid, 1).await?;

	let ship = state.add_ship(pid, 549).await?;
	state.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await?;

	// give two damage control teams
	for api_slotitem_id in [42, 42] {
		state.add_slot_item(pid, api_slotitem_id, 0, 0).await?;
	}

	// furniture
	let codex = state.codex();
	for furniture in &codex.manifest.api_mst_furniture {
		state.add_furniture(pid, furniture.api_id).await?;
	}

	state
		.update_furniture_config(
			pid,
			&FurnitureConfig {
				floor: 629,
				wallpaper: 630,
				window: 319,
				wall_hanging: 631,
				shelf: 222,
				desk: 248,
				season: 0,
			},
		)
		.await?;

	// give use items
	for (t, c) in
		[(KcUseItemType::FCoin, 100000), (KcUseItemType::DockKey, 5), (KcUseItemType::Ring, 7)]
	{
		state.add_use_item(pid, t as i64, c).await?;
	}

	state
		.add_material(
			pid,
			&[
				(MaterialCategory::Fuel, 10000),
				(MaterialCategory::Ammo, 10000),
				(MaterialCategory::Steel, 10000),
				(MaterialCategory::Bauxite, 10000),
				(MaterialCategory::Torch, 300),
				(MaterialCategory::DevMat, 1000),
				(MaterialCategory::Bucket, 1000),
				(MaterialCategory::Screw, 300),
			],
		)
		.await?;

	// add incentives
	add_incentives(state, pid).await?;

	// update first flag
	state.update_user_first_flag(pid, 1).await?;

	Ok(())
}
