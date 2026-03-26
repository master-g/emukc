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
	api_formation_id: i64,
	api_enemy_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let resp = state
		.practice_battle(pid, params.api_deck_id, params.api_formation_id, params.api_enemy_id)
		.await?;

	Ok(KcApiResponse::success(&resp))
}
