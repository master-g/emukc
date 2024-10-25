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

	let preset_decks = state.get_preset_deck(pid).await?;
	let resp: KcApiPresetDeck = preset_decks.into();

	Ok(KcApiResponse::success(&resp))
}
