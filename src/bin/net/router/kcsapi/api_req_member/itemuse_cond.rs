use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
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

	state.consume_cond_use_item(pid, params.api_deck_id, params.api_use_type).await?;

	Ok(KcApiResponse::empty())
}
