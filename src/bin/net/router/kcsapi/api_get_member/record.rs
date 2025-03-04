use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::{model::kc2::level, prelude::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
	api_air_base_expanded_info: Vec<KcApiAirBaseExpandedInfo>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MissionStat {
	api_count: String,
	api_rate: String,
	api_success: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

	let air_bases = state.get_airbases(pid).await?;
	let api_air_base_expanded_info = air_bases
		.iter()
		.map(|v| KcApiAirBaseExpandedInfo {
			api_area_id: v.area_id,
			api_maintenance_level: v.maintenance_level,
		})
		.collect();

	let (_, basic) = state.get_user_basic(pid).await?;
	let (hq_lv, next_lv_exp) = level::exp_to_hq_level(basic.api_experience);

	let furnitures = state.get_furnitures(pid).await?;
	let api_material_max = state.codex.game_cfg.material.get_soft_cap(hq_lv);
	let slotitems = state.get_slot_items(pid).await?;
	let ships = state.get_ships(pid).await?;

	let rate_calculator = |v: i64, total: i64, percentage: bool| -> String {
		let rate = if total == 0 {
			0.0
		} else {
			v as f64 / total as f64
		};
		let rate = if percentage {
			rate * 100.0
		} else {
			rate
		};
		format!("{:.2}", rate)
	};

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
			api_count: basic.api_ms_count.to_string(),
			api_rate: rate_calculator(basic.api_ms_success, basic.api_ms_count, true),
			api_success: basic.api_ms_success.to_string(),
		},
		api_ndoc: basic.api_count_ndock,
		api_nickname: basic.api_nickname,
		api_nickname_id: basic.api_nickname_id,
		api_photo_url: "".to_string(),
		api_practice: Rate {
			api_lose: basic.api_pt_lose.to_string(),
			api_rate: rate_calculator(basic.api_pt_win, basic.api_pt_win + basic.api_pt_lose, true),
			api_win: basic.api_pt_win.to_string(),
		},
		api_rank: basic.api_rank,
		api_ship: [ships.len() as i64, basic.api_max_chara],
		api_slotitem: [slotitems.len() as i64, basic.api_max_slotitem],
		api_war: Rate {
			api_lose: basic.api_st_lose.to_string(),
			api_rate: rate_calculator(
				basic.api_st_win,
				basic.api_st_win + basic.api_st_lose,
				false,
			),
			api_win: basic.api_st_win.to_string(),
		},
	}))
}
