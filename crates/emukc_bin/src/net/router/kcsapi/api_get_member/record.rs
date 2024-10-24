use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::{model::kc2::level, prelude::*};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Record {
	api_air_base_expanded_info: Vec<AirBaseExpandedInfo>,
	api_cmt: String,
	api_cmt_id: String,
	/// Unknown
	api_complate: [String; 2],
	api_deck: i64,
	api_experience: [i64; 2],
	api_friend: i64,
	api_furniture: i64,
	api_kdoc: i64,
	api_large_dock: i64,
	api_level: i64,
	api_material_max: i64,
	api_member_id: i64,
	api_mission: MissionStat,
	api_ndoc: i64,
	api_nickname: String,
	api_nickname_id: String,
	api_photo_url: String,
	api_practice: Rate,
	api_rank: i64,
	api_ship: [i64; 2],
	api_slotitem: [i64; 2],
	api_war: Rate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AirBaseExpandedInfo {
	api_area_id: i64,
	api_maintenance_level: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MissionStat {
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

	let (user, basic) = state.0.get_user_basic(pid).await?;
	let (hq_lv, next_lv_exp) = level::exp_to_hq_level(user.experience);

	let furnitures = state.0.get_furnitures(pid).await?;
	let api_material_max = state.codex.material_cfg.get_soft_cap(hq_lv);
	let slotitems = state.0.get_slot_items(pid).await?;
	let ships = state.0.get_ships(pid).await?;

	Ok(KcApiResponse::success(&Record {
		api_air_base_expanded_info,
		api_cmt: basic.api_comment,
		api_cmt_id: basic.api_comment_id,
		api_complate: ["0.0".to_string(), "0.0".to_string()],
		api_deck: basic.api_count_deck,
		api_experience: [basic.api_experience, next_lv_exp],
		api_friend: 0,
		api_furniture: furnitures.len() as i64,
		api_kdoc: basic.api_count_kdock,
		api_large_dock: basic.api_large_dock,
		api_level: hq_lv,
		api_material_max,
		api_member_id: pid,
		api_mission: MissionStat {
			api_count: user.expeditions.to_string(),
			api_rate: (user.expeditions_success as f64 / user.expeditions as f64 * 100.0)
				.to_string(),
			api_success: user.expeditions_success.to_string(),
		},
		api_ndoc: basic.api_count_ndock,
		api_nickname: basic.api_nickname,
		api_nickname_id: basic.api_nickname_id,
		api_photo_url: "".to_string(),
		api_practice: Rate {
			api_lose: user.practice_battles.to_string(),
			api_rate: (user.practice_battle_wins as f64 / user.practice_battles as f64 * 100.0)
				.to_string(),
			api_win: user.practice_battle_wins.to_string(),
		},
		api_rank: user.hq_rank,
		api_ship: [ships.len() as i64, basic.api_max_chara],
		api_slotitem: [slotitems.len() as i64, basic.api_max_slotitem],
		api_war: Rate {
			api_lose: user.sortie_loses.to_string(),
			api_rate: (user.sortie_wins as f64 / ((user.sortie_wins + user.sortie_loses) as f64)
				* 100.0)
				.to_string(),
			api_win: user.sortie_wins.to_string(),
		},
	}))
}
