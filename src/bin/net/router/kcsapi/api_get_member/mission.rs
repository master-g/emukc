use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_list_items: Vec<KcApiMission>,
	api_limit_time: Option<[i64; 1]>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let codex = state.codex();
	let pid = session.profile.id;
	let (models, enter_limit) = state.get_expeditions(pid).await?;

	let api_list_items = codex
		.manifest
		.api_mst_mission
		.iter()
		.map(|v| KcApiMission {
			api_mission_id: v.api_id,
			api_state: models
				.iter()
				.find(|m| m.mission_id == v.api_id)
				.map(|m| m.state as i64)
				.unwrap_or(0),
		})
		.collect();

	Ok(KcApiResponse::success(&Resp {
		api_list_items,
		api_limit_time: enter_limit.map(|v| [v]),
	}))
}
