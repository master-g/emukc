use axum::Extension;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
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
