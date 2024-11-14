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
	/// target ship id
	api_id: i64,

	/// source slot index
	api_src_idx: i64,

	/// target slot index
	api_dst_idx: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_ship_data: KcApiShip,
}

pub(super) async fn handler(
	state: AppState,
	Extension(_session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let ship = state.slot_exchange(params.api_id, params.api_src_idx, params.api_dst_idx).await?;
	let api_ship_data: KcApiShip = ship.into();

	Ok(KcApiResponse::success(&Resp {
		api_ship_data,
	}))
}
