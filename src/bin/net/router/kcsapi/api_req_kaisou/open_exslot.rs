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
	api_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.open_ship_exslot(pid, params.api_id).await?;

	Ok(KcApiResponse::empty())
}
