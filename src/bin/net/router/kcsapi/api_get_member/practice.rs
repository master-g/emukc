use axum::Extension;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let rivals = state.get_practice_rivals(pid).await?;

	Ok(KcApiResponse::success(&KcApiPracticeResp {
		api_create_kind: rivals.cfg.generated_type as i64,
		api_selected_kind: rivals.cfg.selected_type as i64,
		api_entry_limit: rivals.entry_limit,
		api_list: rivals.rivals.into_iter().map(std::convert::Into::into).collect(),
	}))
}
