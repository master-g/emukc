use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
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
struct Resp {
	api_powerup_flag: i64,
	api_ship: KcApiShip,
	api_deck: Vec<KcApiDeckPort>,
	api_unset_list: Option<Vec<KcApiUnsetListElement>>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let r = state
		.powerup(pid, params.api_id, &params.api_id_items, params.api_slot_dest_flag == 0)
		.await?;

	Ok(KcApiResponse::success(&Resp {
		api_powerup_flag: r.success as i64,
		api_ship: r.ship.into(),
		api_deck: r.fleets.into_iter().map(Into::into).collect(),
		api_unset_list: r.unset_slot_items.map(|m| {
			m.into_iter()
				.map(|(k, v)| KcApiUnsetListElement {
					api_type_3_no: k,
					api_slot_list: v,
				})
				.collect()
		}),
	}))
}
