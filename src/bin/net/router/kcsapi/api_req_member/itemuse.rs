use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_useitem_id: i64,

	// 1: medal for blueprint
	// 61: raw food materials for rice balls
	api_exchange_type: i64,

	// 0: response.api_caution_flag will be 1 if the material will be capped by limit.
	// 1: response.api_caution_flag will be 0 if the material will be capped by limit.
	api_force_flag: i64,
}

#[derive(Serialize, Default)]
struct Resp {
	api_caution_flag: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	Ok(KcApiResponse::empty())
}
