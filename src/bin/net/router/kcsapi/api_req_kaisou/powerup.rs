use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// target ship id
	api_id: i64,

	/// ship ids as materials
	#[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
	api_id_items: Vec<i64>,

	/// 0: keep slot items, 1: destroy slot items
	api_slot_dest_flag: i64,

	/// 0: default, 1: pumpkin involes
	/// this feature is not supported in this implementation yet
	api_limited_feed_type: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UnsetListElement {
	#[serde(rename = "api_type3No")]
	api_type_3_no: i64,
	api_slot_list: Vec<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_powerup_flag: i64,
	api_ship: KcApiShip,
	api_deck: Vec<KcApiDeckPort>,
	api_unset_list: Vec<UnsetListElement>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.powerup(pid, params.api_id, &params.api_id_items, params.api_slot_dest_flag != 0).await?;

	Ok(KcApiResponse::empty())
}
