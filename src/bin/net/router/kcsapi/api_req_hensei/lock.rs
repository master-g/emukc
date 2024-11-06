use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_ship_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_locked: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(_session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let ship = state.toggle_ship_locked(params.api_ship_id).await?;

	Ok(KcApiResponse::success(&Resp {
		api_locked: ship.api_locked,
	}))
}
