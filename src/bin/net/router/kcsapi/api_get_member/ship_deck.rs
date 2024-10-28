use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	auth::GameSession,
	err::ApiError,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	#[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
	api_deck_rid: Vec<i64>,
}

#[derive(Serialize)]
pub(super) struct Resp {
	api_deck_data: Vec<KcApiDeckPort>,
	api_ship_data: Vec<KcApiShip>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let fleets = state.get_fleets(pid).await?;

	let api_deck_data: Vec<KcApiDeckPort> = fleets
		.into_iter()
		.filter_map(|model| {
			if params.api_deck_rid.contains(&model.index) {
				Some(model.into())
			} else {
				None
			}
		})
		.collect();

	let mut api_ship_data: Vec<KcApiShip> = Vec::new();

	for ship_id in
		api_deck_data.iter().flat_map(|deck| deck.api_ship.iter()).filter(|ship_id| **ship_id > 0)
	{
		let ship = state
			.find_ship(*ship_id)
			.await?
			.ok_or(ApiError::NotFound(format!("ship {} not found for profile {}", ship_id, pid)))?;

		api_ship_data.push(ship);
	}

	Ok(KcApiResponse::success(&Resp {
		api_deck_data,
		api_ship_data,
	}))
}
