use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
// use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// 0: disband, 1=機動部隊, 2=水上部隊, 3=輸送部隊
	api_combined_type: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	/// 0: disband, 1=combined
	api_combined: i64,
}

// TODO: implement this
pub(super) async fn handler(
	_state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let _pid = session.profile.id;

	Ok(KcApiResponse::success(&Resp {
		api_combined: 1,
	}))
}
