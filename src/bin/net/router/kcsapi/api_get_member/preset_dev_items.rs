use axum::Extension;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let preset_dev_items = state.get_preset_dev_items(pid).await?;
	let resp: KcApiPresetDevItem = preset_dev_items.into();

	Ok(KcApiResponse::success(&resp))
}
