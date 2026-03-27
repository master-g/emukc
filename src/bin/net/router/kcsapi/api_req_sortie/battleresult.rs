use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Default)]
pub(super) struct Params {
	#[serde(default)]
	api_btime: Option<i64>,
	#[serde(default)]
	api_l_value: Option<Vec<String>>,
	#[serde(default)]
	api_l_value2: Option<Vec<String>>,
	#[serde(default)]
	api_l_value3: Option<Vec<String>>,
	#[serde(default)]
	api_l_value4: Option<Vec<String>>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let _ = (
		params.api_btime,
		params.api_l_value,
		params.api_l_value2,
		params.api_l_value3,
		params.api_l_value4,
	);
	let pid = session.profile.id;
	let resp = state.sortie_battle_result(pid).await?;

	Ok(KcApiResponse::success(&resp))
}
