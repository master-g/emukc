use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::{model::kc2::KcApiIncentive, prelude::IncentiveOps};

#[derive(Serialize, Deserialize, Debug)]
struct Basic {
	api_member_id: i64,
	api_firstflag: i64,
}

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
	api_extra_supply: Vec<i64>,
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
	todo!()
}
