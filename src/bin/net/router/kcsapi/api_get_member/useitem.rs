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
	let use_items = state.get_use_items(pid).await?;
	Ok(KcApiResponse::success(&use_items))
}
