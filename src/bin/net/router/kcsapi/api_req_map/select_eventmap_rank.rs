use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_maparea_id: i64,
	api_map_no: i64,
	api_rank: i64,
}

#[derive(Serialize)]
struct RespMapHp {
	api_now_maphp: i64,
	api_max_maphp: i64,
	api_gauge_type: i64,
	api_gauge_num: i64,
}

#[derive(Serialize)]
struct Resp {
	api_maphp: RespMapHp,
	api_sally_flag: [i64; 3],
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let selected = state
		.select_eventmap_rank(pid, params.api_maparea_id, params.api_map_no, params.api_rank)
		.await?;

	Ok(KcApiResponse::success(&Resp {
		api_maphp: RespMapHp {
			api_now_maphp: selected.now_maphp,
			api_max_maphp: selected.max_maphp,
			api_gauge_type: selected.gauge_type,
			api_gauge_num: selected.gauge_num,
		},
		api_sally_flag: selected.sally_flag,
	}))
}
