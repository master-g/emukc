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
	let materials = state.get_materials(pid).await?;
	let materials: Vec<KcApiMaterialElement> = materials.into();
	Ok(KcApiResponse::success(&materials))
}
