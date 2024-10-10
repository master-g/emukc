use std::collections::BTreeMap;

use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct Furniture {
	api_id: i64,
	api_furniture_id: i64,
	api_furniture_no: i64,
	api_furniture_type: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_basic: KcApiUserBasic,
	api_extra_supply: [i64; 2],
	api_furniture: Vec<Furniture>,
	api_kdock: Vec<KcApiKDock>,
	api_oss_setting: KcApiGameSetting,
	api_position_id: i64,
	api_skin_id: i64,
	api_slot_item: Vec<KcApiSlotItem>,
	api_unsetslot: BTreeMap<String, Vec<i64>>,
	api_useitem: Vec<KcApiUserItem>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let codex = &*state.codex;
	let pid = session.profile.id;
	let api_basic = state.get_user_basic(pid).await?;
	let api_furniture = api_basic
		.api_furniture
		.iter()
		.filter_map(|f| {
			codex
				.find::<ApiMstFurniture>(f)
				.map(|mst| Furniture {
					api_id: *f,
					api_furniture_id: *f,
					api_furniture_no: mst.api_no,
					api_furniture_type: mst.api_type,
				})
				.inspect_err(|e| error!("Failed to find furniture {}: {}", f, e))
				.ok()
		})
		.collect();
	let api_kdock = state.get_kdocks(pid).await?;
	let api_kdock = api_kdock.iter().map(|k| k.to_owned().into()).collect();

	let api_oss_setting = state.get_game_settings(pid).await?;
	let api_extra_supply = api_basic.api_extra_supply;
	let api_position_id = api_oss_setting.api_position_id;
	let api_skin_id = api_oss_setting.api_skin_id;

	let api_slot_item = state.get_slot_items(pid).await?;

	let api_useitem = state.get_use_items(pid).await?;

	Ok(KcApiResponse::success(&Resp {
		api_basic,
		api_extra_supply,
		api_furniture,
		api_kdock,
		api_oss_setting,
		api_position_id,
		api_skin_id,
		api_slot_item,
		api_unsetslot: todo!(),
		api_useitem,
	}))
}
