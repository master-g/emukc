use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc::prelude::PresetOps;

#[derive(Serialize, Deserialize, Debug)]
pub struct Resp {
	api_max_num: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let new_cap = state.expand_preset_dev_item_capacity(pid).await?;

	let resp = Resp {
		api_max_num: new_cap,
	};

	Ok(KcApiResponse::success(&resp))
}
