use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct NicknameParams {
	api_nickname: String,
	api_nickname_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<NicknameParams>,
) -> KcApiResult {
	let pid = session.profile.id;
	state.update_user_nickname(pid, &params.api_nickname).await?;

	let (_, basic) = state.get_user_basic(pid).await?;

	Ok(KcApiResponse::success(&basic))
}
