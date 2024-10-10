use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct FirstShipParams {
	api_ship_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<FirstShipParams>,
) -> KcApiResult {
	let pid = session.profile.id;
	let ship = state.0.add_ship(pid, params.api_ship_id).await?;
	state.0.update_fleet_ships(pid, 1, [ship.api_id, -1, -1, -1, -1, -1]).await?;

	// give two damage control teams
	for api_slotitem_id in [42, 42] {
		state.0.add_slot_item(pid, api_slotitem_id, 0, 0).await?;
	}

	// update first flag
	state.0.update_user_first_flag(pid, 1).await?;

	Ok(KcApiResponse::success(&ship))
}