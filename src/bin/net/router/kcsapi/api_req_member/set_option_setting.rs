use axum::{Extension, Form};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<KcApiOptionSetting>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.update_options_settings(pid, &params).await?;

	Ok(KcApiResponse::empty())
}