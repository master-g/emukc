use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
	api_deck_id: i64,
	api_preset_no: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let fleets = state.get_fleets(pid).await?;
	let preset = state.find_preset_deck(pid, params.api_preset_no).await?;

	let other_fleet_ships: Vec<i64> = fleets
		.iter()
		.filter_map(|f| {
			if f.id == params.api_deck_id {
				None
			} else {
				Some(f.ships.to_vec())
			}
		})
		.flatten()
		.filter(|&sid| sid != -1)
		.collect();

	let mut new_ship_ids: [i64; 6] = [-1; 6];
	for (i, sid) in preset.ships.iter().take(6).enumerate() {
		if *sid == -1 || other_fleet_ships.contains(sid) {
			new_ship_ids[i] = -1;
		} else {
			new_ship_ids[i] = *sid;
		}
	}

	new_ship_ids.move_value_to_end(-1);

	let fleet = state.update_fleet_ships(pid, params.api_deck_id, &new_ship_ids).await?;
	let resp: KcApiDeckPort = fleet.into();

	Ok(KcApiResponse::success(&resp))
}
