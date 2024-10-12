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
	let docks = state.get_kdocks(pid).await?;
	let docks: Vec<KcApiKDock> = docks.into_iter().map(std::convert::Into::into).collect();

	Ok(KcApiResponse::success(&docks))
}
