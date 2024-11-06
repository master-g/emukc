use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	err::ApiError,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// 0: plane, 1: fuel, 2: ammo, 3: all
	api_kind: i64,

	/// 1: aircraft replenishment
	api_onslot: i64,

	/// ship ids
	#[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
	api_id_items: Vec<i64>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let charge_type = KcApiChargeKind::n(params.api_kind)
		.ok_or_else(|| ApiError::Unknown(format!("Invalid charge type: {}", params.api_kind)))?;

	let resp =
		state.charge_supply(pid, &params.api_id_items, charge_type, params.api_onslot == 1).await?;

	Ok(KcApiResponse::success(&resp))
}
