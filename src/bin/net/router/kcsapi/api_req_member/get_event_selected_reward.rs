use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
// use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_selected_dict: i64,
}

pub(super) async fn handler(
	_state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	debug!("get_event_selected_reward: pid={}, selected_dict={}", pid, params.api_selected_dict);

	Ok(KcApiResponse::empty())
}
