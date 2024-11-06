use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
	api_preset_no: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.delete_preset_deck(pid, params.api_preset_no).await?;

	Ok(KcApiResponse::empty())
}
