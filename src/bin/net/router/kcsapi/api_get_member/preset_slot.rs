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

	let preset_slot = state.get_preset_slots(pid).await?;
	let resp: KcApiPresetSlot = preset_slot.into();

	Ok(KcApiResponse::success(&resp))
}
