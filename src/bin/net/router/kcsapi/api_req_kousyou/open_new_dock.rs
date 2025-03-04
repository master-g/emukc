use axum::Extension;

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.expand_construction_dock(pid).await?;

	Ok(KcApiResponse::empty())
}
