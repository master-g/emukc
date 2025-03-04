use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// slot item id list
	#[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
	api_slotitem_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_get_material: Vec<i64>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let materials = state.destroy_items(pid, &params.api_slotitem_ids).await?;

	Ok(KcApiResponse::success(&Resp {
		api_get_material: materials.iter().map(|v| v.1).collect(),
	}))
}
