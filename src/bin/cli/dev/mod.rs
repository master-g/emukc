use anyhow::Result;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};

use emukc::{
	model::profile::furniture::FurnitureConfig,
	prelude::{
		AccountOps, BasicOps, FleetOps, FurnitureOps, HasContext, IncentiveOps, KcApiIncentiveItem,
		KcUseItemType, MaterialCategory, MaterialOps, ProfileOps, ShipOps, SlotItemOps, UseItemOps,
	},
};

use crate::{cfg::AppConfig, state::State};

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

async fn add_ship_quietly(state: &State, pid: i64) -> Result<()> {
	for ship_mst_id in [184, 187, 433, 951] {
		state.add_ship(pid, ship_mst_id).await?;
	}

	let codex = state.codex();

	let mut rng = SmallRng::from_entropy();
	let mut i = 0;
	loop {
		let mst = codex.manifest.api_mst_ship.choose(&mut rng).unwrap();
		if codex.ship_extra.contains_key(&mst.api_id) {
			state.add_ship(pid, mst.api_id).await?;
			i += 1;
		}

		if i >= 90 {
			break;
		}
	}

	Ok(())
}

#[allow(unused)]
async fn add_ship_incentives(state: &State, pid: i64) -> Result<()> {
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

	Ok(())
}

#[allow(unused)]
async fn deplete_ship_fuel_and_ammo(state: &State, pid: i64) -> Result<()> {
	// modify ships for testing `api_req_hokyu/charge`
	let mut ships = state.get_ships(pid).await?;

	for ship in ships.iter_mut() {
		ship.api_fuel = 0;
		ship.api_bull = 0;
		ship.api_onslot = [0, 0, 0, 0, 0];
		ship.api_nowhp = 1;

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
	for (t, c) in [
		(KcUseItemType::FCoin, 100000),
		(KcUseItemType::DockKey, 5),
		(KcUseItemType::Ring, 7),
		(KcUseItemType::ReinforceExpansion, 100),
		(KcUseItemType::FCoinBox200, 1),
		(KcUseItemType::FCoinBox400, 10),
		(KcUseItemType::FCoinBox700, 23),
		(KcUseItemType::Medal, 100),
		(KcUseItemType::Chocolate, 10),
		(KcUseItemType::Irako, 100),
		(KcUseItemType::Mamiya, 100),
		(KcUseItemType::Presents, 100),
		(KcUseItemType::FirstClassMedal, 100),
		(KcUseItemType::Hishimochi, 100),
		(KcUseItemType::HQPersonnel, 100),
		(KcUseItemType::Saury, 100),
		(KcUseItemType::Sardine, 100),
	] {
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

	add_ship_quietly(state, pid).await?;

	// add incentives
	// add_ship_incentives(state, pid).await?;
	// deplete ship fuel and ammo
	deplete_ship_fuel_and_ammo(state, pid).await?;

	// update first flag
	state.update_user_first_flag(pid, 1).await?;

	Ok(())
}
