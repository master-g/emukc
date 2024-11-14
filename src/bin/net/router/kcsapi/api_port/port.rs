use axum::Extension;
use serde::Serialize;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	router::kcs2::GIT_HASH,
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize)]
struct Resp {
	api_material: Vec<KcApiMaterialElement>,
	api_deck_port: Vec<KcApiDeckPort>,
	api_ndock: Vec<KcApiNDock>,
	api_ship: Vec<KcApiShip>,
	api_basic: KcApiUserBasic,
	api_log: Vec<KcApiLogElement>,
	api_combined_flag: i64,
	api_p_bgm_id: i64,
	// api_event_object: KcApiEventObject,
	api_parallel_quest_count: i64,
	api_dest_ship_slot: i64,
	// api_plane_info: Vec<KcApiPlaneInfo>,
	// api_furniture_affect_items: Vec<i64>,
	api_c_flags: Vec<i64>,
	api_c_flag2: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;
	let (_, api_basic) = state.get_user_basic(pid).await?;

	state.update_materials(pid).await?;

	// TODO: update quests here

	let api_material = state.get_materials(pid).await?;
	let api_material: Vec<KcApiMaterialElement> = api_material.into();

	let api_deck_port = state.get_fleets(pid).await?;
	let api_deck_port: Vec<KcApiDeckPort> =
		api_deck_port.into_iter().map(std::convert::Into::into).collect();
	let api_dest_ship_slot = 1;
	let api_ndock = state.get_ndocks(pid).await?;
	let api_ndock: Vec<KcApiNDock> = api_ndock.into_iter().map(std::convert::Into::into).collect();

	// FIXME: the ndock info is not correct
	let api_ship = state.get_ships(pid).await?;

	let ver = format!("Welcome to EmuKC {}-{}", VERSION, GIT_HASH.to_uppercase());
	let api_log = vec![KcApiLogElement {
		api_no: 0,
		api_type: "10".to_string(),
		api_state: "0".to_string(),
		api_message: ver,
	}];

	// log type
	// 1: ndock
	// 2: factory
	// 3: expedition
	// 4: provision
	// 5: practice
	// 6: medal
	// 7: sortie
	// 8: quest
	// 9: apply
	// 10: promotion
	// 11: picturebook
	// 12: complete
	// 13: n/a
	// 14: sortie
	// 15: remodel

	let settings = state.get_game_settings(pid).await?;
	let api_p_bgm_id = settings.api_p_bgm_id;
	let api_parallel_quest_count = api_basic.api_max_quests;
	let api_c_flags: Vec<i64> = vec![0]; // event functional flags
	let api_c_flag2 = 0; // mini event item usage lock flag

	Ok(KcApiResponse::success(&Resp {
		api_material,
		api_deck_port,
		api_dest_ship_slot,
		api_ndock,
		api_ship,
		api_basic,
		api_log,
		api_p_bgm_id,
		api_parallel_quest_count,
		api_c_flags,
		api_c_flag2,
		api_combined_flag: 0,
	}))
}
