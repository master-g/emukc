use axum::{Extension, Form};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<super::start::Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.quest_stop(pid, params.api_quest_id).await?;

	Ok(KcApiResponse::empty())
}
