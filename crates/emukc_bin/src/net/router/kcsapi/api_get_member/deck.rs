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
	let fleets = state.get_fleets(pid).await?;
	let deck_ports: Vec<KcApiDeckPort> = fleets.into_iter().map(std::convert::Into::into).collect();

	Ok(KcApiResponse::success(&deck_ports))
}
