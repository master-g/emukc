use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_preset_id: i64,
	api_ship_id: i64,
	// 1: A, 2: B
	api_equip_mode: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state
		.apply_preset_slot(pid, params.api_preset_id, params.api_ship_id, params.api_equip_mode)
		.await?;

	// `api_bauxite` is not implemented yet (日進に大型飛行艇を装備させた場合など)
	Ok(KcApiResponse::empty())
}
