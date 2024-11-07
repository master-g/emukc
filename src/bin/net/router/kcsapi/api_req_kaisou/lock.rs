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
	api_slotitem_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_locked: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(_session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let item = state.toggle_slot_item_locked(params.api_slotitem_id).await?;

	Ok(KcApiResponse::success(&Resp {
		api_locked: item.api_locked,
	}))
}
