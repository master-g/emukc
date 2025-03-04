use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_ndock_id: i64,
	api_ship_id: i64,
	api_highspeed: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state
		.ndock_start_repair(pid, params.api_ndock_id, params.api_ship_id, params.api_highspeed == 1)
		.await?;

	Ok(KcApiResponse::empty())
}
