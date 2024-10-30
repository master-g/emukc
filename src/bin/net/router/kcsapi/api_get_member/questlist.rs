use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_tab_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_count: i64,
	api_completed_kind: i64,
	api_list: Vec<KcApiQuestItem>,
	api_exec_count: i64,
	api_exec_type: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	Ok(KcApiResponse::success(&Resp {
		api_count: 0,
		api_completed_kind: 0,
		api_list: vec![],
		api_exec_count: 0,
		api_exec_type: 0,
	}))
}
