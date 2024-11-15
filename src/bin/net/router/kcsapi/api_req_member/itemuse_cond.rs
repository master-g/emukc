use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	// 1 = mamiya, 2: irako, 3: mamiya+irako
	api_use_type: i64,

	// deck id
	api_deck_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	Ok(KcApiResponse::empty())
}
