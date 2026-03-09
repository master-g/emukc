use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc::prelude::PresetOps;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_preset_id: i64,
	api_name: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.update_preset_dev_item_name(pid, params.api_preset_id, params.api_name).await?;

	Ok(KcApiResponse::empty())
}
