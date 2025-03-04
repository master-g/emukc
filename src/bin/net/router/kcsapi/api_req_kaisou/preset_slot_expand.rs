use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_max_num: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let api_max_num = state.expand_preset_slot_capacity(pid).await?;

	Ok(KcApiResponse::success(&Resp {
		api_max_num,
	}))
}
