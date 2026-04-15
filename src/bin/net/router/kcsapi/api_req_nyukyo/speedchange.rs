use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Debug)]
pub(super) struct Params {
	api_ndock_id: i64,
}

#[derive(Serialize)]
struct Resp {
	api_material: Vec<i64>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.speed_up_ship_repairation(pid, params.api_ndock_id).await?;

	let materials = state.get_materials(pid).await?;
	let api_material = materials.into_array().to_vec();

	Ok(KcApiResponse::success(&Resp { api_material }))
}
