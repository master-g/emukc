use axum::Extension;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(_session): Extension<GameSession>,
) -> KcApiResult {
	let codex = state.codex();
	let music_list = &codex.music_list;

	Ok(KcApiResponse::success(music_list))
}
