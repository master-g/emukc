use axum::{Extension, Form};
use emukc::model::profile::fleet::Fleet;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
	api_deck_id: i64,
	api_preset_no: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let fleet = state.apply_preset_deck(pid, params.api_deck_id, params.api_preset_no).await?;
	let fleet: Fleet = fleet.into();
	let resp: KcApiDeckPort = fleet.into();

	Ok(KcApiResponse::success(&resp))
}
