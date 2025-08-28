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
	pub api_quest_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.quest_start(pid, params.api_quest_id).await?;

	Ok(KcApiResponse::empty())
}
