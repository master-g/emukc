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
	api_deck_id: i64,
	api_formation_id: i64,
	api_maparea_id: i64,
	api_mapinfo_no: i64,
	#[serde(default)]
	api_serial_cid: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let _ = params.api_serial_cid.as_str();
	let pid = session.profile.id;
	let resp = state
		.start_sortie(
			pid,
			params.api_deck_id,
			params.api_maparea_id,
			params.api_mapinfo_no,
			params.api_formation_id,
		)
		.await?;

	Ok(KcApiResponse::success(&resp))
}
