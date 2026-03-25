use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_deck_id: i64,
	api_mission_id: i64,
	#[serde(default)]
	api_mission: String,
}

#[derive(Serialize)]
struct Resp {
	api_complatetime: i64,
	api_complatetime_str: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let _ = params.api_mission.as_str();
	let pid = session.profile.id;
	let result = state.start_expedition(pid, params.api_deck_id, params.api_mission_id).await?;
	let complete_ms = result.complete_time.timestamp_millis();

	Ok(KcApiResponse::success(&Resp {
		api_complatetime: complete_ms,
		api_complatetime_str: KcTime::format_date(complete_ms, " "),
	}))
}
