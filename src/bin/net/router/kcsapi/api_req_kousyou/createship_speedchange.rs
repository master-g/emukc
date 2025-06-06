use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// should be 1
	api_highspeed: i64,
	/// construction dock id
	api_kdock_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.speed_up_ship_construction(pid, params.api_kdock_id).await?;

	Ok(KcApiResponse::empty())
}
