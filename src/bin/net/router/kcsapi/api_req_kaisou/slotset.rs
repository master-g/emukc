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
	api_id: i64,
	api_item_id: i64,
	api_slot_idx: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(_session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	state.set_slot_item(params.api_id, params.api_slot_idx, params.api_item_id).await?;

	Ok(KcApiResponse::empty())
}
