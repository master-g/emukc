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
	api_deck_id: i64,
	api_name_id: String,
	api_name: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	debug!(
		"update_deck_name: pid={}, deck_id={}, name_id={}, name={}",
		pid, params.api_deck_id, params.api_name_id, params.api_name
	);

	state.update_deck_name(pid, params.api_deck_id, &params.api_name).await?;

	Ok(KcApiResponse::empty())
}
