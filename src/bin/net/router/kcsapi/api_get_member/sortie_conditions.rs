use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_win: i64,
	api_lose: i64,
	api_rate: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let (m, _) = state.get_user_basic(pid).await?;

	let rate = m.sortie_wins as f64 / (m.sortie_loses + m.sortie_wins) as f64;

	Ok(KcApiResponse::success(&Resp {
		api_win: m.sortie_wins,
		api_lose: m.sortie_loses,
		api_rate: format!("{:.2}", rate * 100.0),
	}))
}
