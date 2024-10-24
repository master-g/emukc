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
	let (_, basic) = state.get_user_basic(pid).await?;
	Ok(KcApiResponse::success(&basic))
}
