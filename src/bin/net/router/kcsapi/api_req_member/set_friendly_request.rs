use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
// use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	// 0: denied, 1: approved
	api_request_flag: i64,

	// 0: default, 1: powerful
	api_request_typ: i64,
}

pub(super) async fn handler(
	_state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	debug!(
		"set_friendly_request: pid={}, request_flag={}, request_typ={}",
		pid, params.api_request_flag, params.api_request_typ
	);

	Ok(KcApiResponse::empty())
}
