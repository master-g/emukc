use axum::{Extension, Form};
use emukc::model::profile::preset_deck::PresetDeckItem;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_preset_no: i64,
	api_deck_id: i64,
	api_name: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let fleet = state.get_fleet(pid, params.api_deck_id).await?;

	let preset = PresetDeckItem {
		index: params.api_preset_no,
		name: params.api_name.clone(),
		ships: [
			fleet.ships[0],
			fleet.ships[1],
			fleet.ships[2],
			fleet.ships[3],
			fleet.ships[4],
			fleet.ships[5],
			-1,
		],
	};

	let m = state.register_preset_deck(pid, &preset).await?;
	let new_preset: PresetDeckItem = m.into();
	let resp: KcApiPresetDeckElement = new_preset.into();

	Ok(KcApiResponse::success(&resp))
}
