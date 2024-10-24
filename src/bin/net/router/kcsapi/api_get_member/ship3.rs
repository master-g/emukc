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
	api_shipid: i64,
	api_sort_key: i64,
	api_sort_order: i64,
}

#[derive(Serialize)]
pub(super) struct Resp {
	api_deck_data: Vec<KcApiDeckPort>,
	api_ship_data: Vec<KcApiShip>,
	api_slot_data: KcApiUnsetSlot,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let fleets = state.get_fleets(pid).await?;
	let api_deck_data = fleets.into_iter().map(std::convert::Into::into).collect();

	let ship = state.find_ship(params.api_shipid).await?.ok_or(ApiError::NotFound(format!(
		"ship {} not found for profile {}",
		params.api_shipid, pid
	)))?;

	let unset_slot = state.get_unuse_slot_items(pid).await?;
	let api_slot_data = state.codex.convert_unused_slot_items_to_api(&unset_slot)?;

	Ok(KcApiResponse::success(&Resp {
		api_deck_data,
		api_ship_data: vec![ship],
		api_slot_data,
	}))
}
