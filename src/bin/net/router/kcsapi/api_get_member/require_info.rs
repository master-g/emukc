use std::collections::BTreeMap;

use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct UserBasic {
	api_member_id: i64,
	api_firstflag: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_basic: UserBasic,
	api_extra_supply: [i64; 2],
	api_furniture: Vec<KcApiFurniture>,
	api_kdock: Vec<KcApiKDock>,
	api_oss_setting: KcApiOssSetting,
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
	let (_, api_basic) = state.get_user_basic(pid).await?;
	let api_furniture = state.get_furnitures(pid).await?;
	let api_kdock = state.get_kdocks(pid).await?;
	let api_kdock = api_kdock.iter().map(|k| k.to_owned().into()).collect();

	let api_oss_setting = state.get_oss_settings(pid).await?;
	let api_game_settings = state.get_game_settings(pid).await?;
	let api_option_settings = state.get_option_settings(pid).await?;

	let api_extra_supply = api_basic.api_extra_supply;
	let api_position_id = api_game_settings.api_position_id;
	let api_skin_id = api_option_settings.map(|s| s.api_skin_id).unwrap_or(101);

	let api_slot_item = state.get_slot_items(pid).await?;
	let api_useitem = state.get_use_items(pid).await?;

	let unused_slot_items = state.get_unset_slot_items(pid).await?;
	let api_unsetslot = codex.convert_unused_slot_items_to_api(&unused_slot_items)?;

	Ok(KcApiResponse::success(&Resp {
		api_basic: UserBasic {
			api_member_id: api_basic.api_member_id,
			api_firstflag: api_basic.api_firstflag,
		},
		api_extra_supply,
		api_furniture,
		api_kdock,
		api_oss_setting,
		api_position_id,
		api_skin_id,
		api_slot_item,
		api_unsetslot,
		api_useitem,
	}))
}
