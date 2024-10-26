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
	let codex = state.codex();

	let unset_slots = state.get_unuse_slot_items(pid).await?;
	let resp: KcApiUnsetSlot = codex.convert_unused_slot_items_to_api(&unset_slots)?;

	Ok(KcApiResponse::success(&resp))
}