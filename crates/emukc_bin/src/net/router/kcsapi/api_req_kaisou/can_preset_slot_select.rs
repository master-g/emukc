use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	// TODO: implement this
	// let pid = session.profile.id;
	// let preset_slots = state.0.get_preset_slot(pid).await?;
	// let flag = if preset_slots.api_preset_items.is_empty() {
	// 	0
	// } else {
	// 	1
	// };
	let flag = 1;

	let data = serde_json::json!({
	   "api_flag": flag
	});

	Ok(KcApiResponse::success_json(data))
}
