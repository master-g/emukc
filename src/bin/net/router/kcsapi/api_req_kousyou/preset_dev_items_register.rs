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
	api_item1: i64,
	api_item2: i64,
	api_item3: i64,
	api_item4: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let preset = PresetDevItemElement {
		index: params.api_preset_id,
		name: String::new(),
		item1: params.api_item1,
		item2: params.api_item2,
		item3: params.api_item3,
		item4: params.api_item4,
	};

	state.register_preset_dev_item(pid, &preset).await?;

	Ok(KcApiResponse::empty())
}
