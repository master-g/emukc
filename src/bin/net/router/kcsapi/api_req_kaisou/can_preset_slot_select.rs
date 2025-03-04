use axum::Extension;
use emukc::prelude::PresetOps;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;
	let preset_slots = state.get_preset_slots(pid).await?;
	let flag = if preset_slots.records.is_empty() {
		0
	} else {
		1
	};

	let data = serde_json::json!({
	   "api_flag": flag
	});

	Ok(KcApiResponse::success_json(data))
}
