use axum::{Extension, Form};
use emukc::prelude::SettingsOps;
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
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
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	debug!(
		"set_friendly_request: pid={}, request_flag={}, request_typ={}",
		pid, params.api_request_flag, params.api_request_typ
	);

	state
		.update_friendly_fleet_settings(pid, params.api_request_flag == 1, params.api_request_typ)
		.await?;

	Ok(KcApiResponse::empty())
}
