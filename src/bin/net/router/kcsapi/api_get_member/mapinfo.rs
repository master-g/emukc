use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Resp {
	api_air_base: Vec<KcApiAirBase>,
	api_air_base_expanded_info: Vec<KcApiAirBaseExpandedInfo>,
	api_map_info: Vec<KcApiMapInfo>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let airbases = state.get_airbases(pid).await?;
	let api_air_base_expanded_info = airbases
		.iter()
		.map(|v| KcApiAirBaseExpandedInfo {
			api_area_id: v.area_id,
			api_maintenance_level: v.maintenance_level,
		})
		.collect();
	let api_air_base = airbases.into_iter().map(std::convert::Into::into).collect();

	let api_map_info = state.get_map_infos(pid).await?;

	Ok(KcApiResponse::success(&Resp {
		api_air_base,
		api_air_base_expanded_info,
		api_map_info,
	}))
}
