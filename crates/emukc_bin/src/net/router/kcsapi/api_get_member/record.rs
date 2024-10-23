use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Record {
	api_air_base_expanded_info: Vec<AirBaseExpandedInfo>,
	api_cmt: String,
	api_cmt_id: String,
	/// Unknown
	api_complate: Vec<String>,
	api_deck: i64,
	api_experience: Vec<i64>,
	api_friend: i64,
	api_furniture: i64,
	api_kdoc: i64,
	api_large_dock: i64,
	api_level: i64,
	api_material_max: i64,
	api_member_id: i64,
	api_mission: Mission,
	api_ndoc: i64,
	api_nickname: String,
	api_nickname_id: String,
	api_photo_url: String,
	api_practice: Rate,
	api_rank: i64,
	api_ship: Vec<i64>,
	api_slotitem: Vec<i64>,
	api_war: Rate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AirBaseExpandedInfo {
	api_area_id: i64,
	api_maintenance_level: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Mission {
	api_count: String,
	api_rate: String,
	api_success: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Rate {
	api_lose: String,
	api_rate: String,
	api_win: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let air_bases = state.0.get_airbases(pid).await?;
	let api_air_base_expanded_info = air_bases
		.iter()
		.map(|v| AirBaseExpandedInfo {
			api_area_id: v.area_id,
			api_maintenance_level: v.maintenance_level,
		})
		.collect();

	Ok(KcApiResponse::success(&Record {
		api_air_base_expanded_info,
		api_cmt: todo!(),
		api_cmt_id: todo!(),
		api_complate: todo!(),
		api_deck: todo!(),
		api_experience: todo!(),
		api_friend: todo!(),
		api_furniture: todo!(),
		api_kdoc: todo!(),
		api_large_dock: todo!(),
		api_level: todo!(),
		api_material_max: todo!(),
		api_member_id: todo!(),
		api_mission: todo!(),
		api_ndoc: todo!(),
		api_nickname: todo!(),
		api_nickname_id: todo!(),
		api_photo_url: todo!(),
		api_practice: todo!(),
		api_rank: todo!(),
		api_ship: todo!(),
		api_slotitem: todo!(),
		api_war: todo!(),
	}))
}
